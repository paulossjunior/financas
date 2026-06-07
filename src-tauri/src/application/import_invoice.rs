use std::path::Path;
use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{categorizer::Categorizer, invoice::{Invoice, YearMonth}, AppConfig};
use crate::infrastructure::{
    btg_mapper::map_sheet_to_transactions,
    xlsx_parser::{parse_xlsx, ParseError},
};
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
) -> Result<ImportResult, ImportError> {
    if !path.exists() {
        return Err(ImportError::FileNotFound);
    }

    let sheet = parse_xlsx(path).map_err(|e| match e {
        ParseError::Encrypted => ImportError::Encrypted,
        ParseError::InvalidFormat(s) => ImportError::InvalidFormat(s),
        ParseError::IoError(s) => ImportError::ParseError(s),
        ParseError::EmptySheet => ImportError::ParseError("Planilha vazia".into()),
    })?;

    let invoice_id = Uuid::new_v4();

    let cat_rules: Vec<_> = config.category_rules.clone();
    let categorizer = if cat_rules.is_empty() {
        Categorizer::with_defaults()
    } else {
        Categorizer::new(cat_rules)
    };

    let (transactions, raw_warnings) =
        map_sheet_to_transactions(&sheet, invoice_id, &categorizer)
            .map_err(|e| ImportError::InvalidFormat(e.to_string()))?;

    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let reference_month = infer_month_from_filename(&filename);
    let row_count = transactions.len();

    let invoice = Invoice::new(
        filename.clone(),
        reference_month.clone(),
        None,
        transactions,
        Local::now().naive_local(),
    );

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
