//! Application use-case: import a card invoice — pick the bank's reader strategy,
//! map rows to transactions, categorize them, and add the resulting [`Invoice`]
//! (stamped with the bank) to the store. Bank-agnostic: everything specific to a
//! bank's file lives behind `infrastructure::invoice_reader::InvoiceReader`.

use std::path::Path;
use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{categorizer::Categorizer, invoice::{Invoice, YearMonth}, AppConfig};
use crate::infrastructure::invoice_reader::{invoice_reader_for, InvoiceReadError};
use super::store::InvoiceStore;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseWarningDto {
    pub row: u32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub invoice_id: String,
    pub filename: String,
    pub month: String,
    pub row_count: usize,
    pub is_replace: bool,
    pub warnings: Vec<ParseWarningDto>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum ImportError {
    #[error("ENCRYPTED_FILE")]
    Encrypted,
    #[error("WRONG_PASSWORD")]
    WrongPassword,
    #[error("INVALID_FORMAT:{0}")]
    InvalidFormat(String),
    #[error("FILE_NOT_FOUND")]
    FileNotFound,
    #[error("PARSE_ERROR:{0}")]
    ParseError(String),
}

pub fn import_invoice(
    path: &Path,
    store: &mut InvoiceStore,
    config: &AppConfig,
    password: Option<&str>,
) -> Result<ImportResult, ImportError> {
    if !path.exists() {
        return Err(ImportError::FileNotFound);
    }

    // Strategy dispatch: which bank's invoice is this file?
    let reader = invoice_reader_for(path)
        .ok_or_else(|| ImportError::InvalidFormat("extensão sem leitor de fatura".into()))?;

    // Deterministic invoice_id from filename so transaction IDs are stable across sessions
    let filename_str = path.file_name().unwrap_or_default().to_string_lossy();
    let invoice_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, filename_str.as_bytes());

    let cat_rules: Vec<_> = config.category_rules.clone();
    let categorizer = if cat_rules.is_empty() {
        Categorizer::with_defaults()
    } else {
        Categorizer::new(cat_rules)
    };

    let (transactions, raw_warnings) = reader
        .read(path, password, invoice_id, &categorizer)
        .map_err(|e| match e {
            InvoiceReadError::Encrypted => ImportError::Encrypted,
            InvoiceReadError::WrongPassword => ImportError::WrongPassword,
            InvoiceReadError::InvalidFormat(s) => ImportError::InvalidFormat(s),
            InvoiceReadError::Io(s) => ImportError::ParseError(s),
            InvoiceReadError::Empty => ImportError::ParseError("Planilha vazia".into()),
        })?;

    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let reference_month = infer_month_from_filename(&filename);
    let row_count = transactions.len();

    // Apply transaction overrides on top of rule-based categorization
    let mut transactions = transactions;
    for tx in transactions.iter_mut() {
        if let Some(cat) = config.transaction_overrides.get(&tx.id.to_string()) {
            tx.category = cat.clone();
        }
    }

    let mut invoice = Invoice::new(
        filename.clone(),
        reference_month.clone(),
        None,
        transactions,
        Local::now().naive_local(),
    );
    // The record is bank-generic; the strategy that read the file says whose it is.
    invoice.bank = reader.bank().to_string();

    let is_replace = store.add(invoice);
    let invoice_id_str = {
        let list = store.list();
        list.iter()
            .find(|i| i.filename == filename)
            .map(|i| i.id.to_string())
            .unwrap_or_default()
    };

    let warnings: Vec<ParseWarningDto> = raw_warnings
        .into_iter()
        .map(|w| ParseWarningDto { row: w.row, message: w.message })
        .collect();

    Ok(ImportResult {
        invoice_id: invoice_id_str,
        filename,
        month: reference_month.to_string_iso(),
        row_count,
        is_replace,
        warnings,
    })
}

fn infer_month_from_filename(filename: &str) -> YearMonth {
    if filename.len() >= 7 {
        let year: Option<i32> = filename[0..4].parse().ok();
        let month: Option<u8> = filename[5..7].parse().ok();
        if let (Some(y), Some(m)) = (year, month) {
            if (1..=12).contains(&m) {
                return YearMonth::new(y, m);
            }
        }
    }
    let now = Local::now();
    YearMonth::new(now.year(), now.month() as u8)
}

use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::store::InvoiceStore;
    use std::collections::HashMap;
    use std::path::Path;

    fn fixture_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/fixtures/sample_fatura.xlsx")
    }

    #[test]
    fn import_applies_transaction_override() {
        let fixture = fixture_path();
        assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

        // First import: no overrides — get real transaction ID
        let no_override_config = AppConfig {
            category_rules: vec![],
            transaction_overrides: HashMap::new(),
            manual_entries: vec![],
            import_directory: None,
        };
        let mut store = InvoiceStore::new();
        import_invoice(&fixture, &mut store, &no_override_config, None).expect("first import failed");
        let invoices = store.list();
        assert!(!invoices.is_empty());
        let first_tx_id = invoices[0].transactions[0].id.to_string();

        // Second import: override that transaction's category
        let mut overrides = HashMap::new();
        overrides.insert(first_tx_id.clone(), "TestOverrideCategory".to_string());
        let override_config = AppConfig {
            category_rules: vec![],
            transaction_overrides: overrides,
            manual_entries: vec![],
            import_directory: None,
        };
        let mut store2 = InvoiceStore::new();
        import_invoice(&fixture, &mut store2, &override_config, None).expect("second import failed");

        let invoices2 = store2.list();
        let actual_category = &invoices2[0].transactions[0].category;
        assert_eq!(
            actual_category, "TestOverrideCategory",
            "override must be applied during import"
        );
    }
}
