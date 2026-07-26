//! Strategy: one card-invoice reader per bank, behind a single trait.
//!
//! Mirror of `statement_reader` for the invoice pipeline: the bank-specific work
//! (open the file, decrypt if needed, map rows to [`Transaction`]s) lives behind
//! [`InvoiceReader`], and the application layer (`import_invoice`) stays
//! bank-agnostic — it asks the registry for the strategy, stamps the invoice with
//! `reader.bank()`, and persists through the same store/DB path for every bank.
//!
//! Today only BTG issues invoices the app reads; the seam is where the next bank
//! plugs in (one impl + one line in [`INVOICE_READERS`]).

use std::path::Path;

use uuid::Uuid;

use crate::domain::categorizer::Categorizer;
use crate::domain::invoice::YearMonth;
use crate::domain::transaction::{ParseWarning, Transaction};
use crate::infrastructure::btg_mapper::map_sheet_to_transactions;
use crate::infrastructure::xlsx_parser::{parse_xlsx, ParseError};

/// Why an invoice file could not be read. Infrastructure-level; the application
/// layer maps these onto its user-facing `ImportError` codes.
#[derive(Debug)]
pub enum InvoiceReadError {
    Encrypted,
    WrongPassword,
    InvalidFormat(String),
    Io(String),
    Empty,
}

/// Everything one read of an invoice file yields.
pub struct InvoiceRead {
    pub transactions: Vec<Transaction>,
    pub warnings: Vec<ParseWarning>,
    /// Reference month when the reader can determine it from the file itself
    /// (Santander: `Fatura_MMYYYY` filename / printed due date). `None` = the
    /// application's filename inference (BTG `YYYY-MM-…`) applies.
    pub reference_month: Option<YearMonth>,
}

/// Strategy interface: how to read one bank's card invoice file.
pub trait InvoiceReader: Sync {
    /// Bank name exactly as persisted in `Invoice.bank` ("BTG", "Santander").
    fn bank(&self) -> &'static str;

    /// Lowercase file extensions this reader accepts.
    fn extensions(&self) -> &'static [&'static str];

    /// Read the file and map its rows to transactions. `invoice_id` seeds the
    /// deterministic per-row transaction ids; `categorizer` applies the app's rules.
    fn read(
        &self,
        path: &Path,
        password: Option<&str>,
        invoice_id: Uuid,
        categorizer: &Categorizer,
    ) -> Result<InvoiceRead, InvoiceReadError>;
}

/// Strategy: BTG card invoice — `.xlsx`, possibly password-protected.
pub struct BtgInvoiceReader;

impl InvoiceReader for BtgInvoiceReader {
    fn bank(&self) -> &'static str {
        "BTG"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["xlsx"]
    }

    fn read(
        &self,
        path: &Path,
        password: Option<&str>,
        invoice_id: Uuid,
        categorizer: &Categorizer,
    ) -> Result<InvoiceRead, InvoiceReadError> {
        let sheet = parse_xlsx(path, password).map_err(|e| match e {
            ParseError::Encrypted => InvoiceReadError::Encrypted,
            ParseError::WrongPassword => InvoiceReadError::WrongPassword,
            ParseError::InvalidFormat(s) => InvoiceReadError::InvalidFormat(s),
            ParseError::IoError(s) => InvoiceReadError::Io(s),
            ParseError::EmptySheet => InvoiceReadError::Empty,
        })?;
        let (transactions, warnings) = map_sheet_to_transactions(&sheet, invoice_id, categorizer)
            .map_err(|e| InvoiceReadError::InvalidFormat(e.to_string()))?;
        // BTG filenames are `YYYY-MM-…` — the application-level inference handles it.
        Ok(InvoiceRead { transactions, warnings, reference_month: None })
    }
}

/// Every registered invoice reader, in dispatch order.
pub static INVOICE_READERS: [&dyn InvoiceReader; 2] =
    [&BtgInvoiceReader, &crate::infrastructure::santander_invoice::SantanderInvoiceReader];

/// Pick the strategy for a file by its extension. `None` = no bank issues invoices
/// in this format.
pub fn invoice_reader_for(path: &Path) -> Option<&'static dyn InvoiceReader> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    INVOICE_READERS.iter().copied().find(|r| r.extensions().contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // T028 — BTG regression: the reader must not take over month inference (its
    // filenames are YYYY-MM-…, handled by the application), and the fixture invoice
    // still parses identically through the strategy.
    #[test]
    fn btg_reader_keeps_legacy_month_inference_and_parses_fixture() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/fixtures/sample_fatura.xlsx");
        let invoice_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, b"sample_fatura.xlsx");
        let read = BtgInvoiceReader
            .read(&fixture, None, invoice_id, &Categorizer::with_defaults())
            .expect("fixture BTG deve continuar lendo");
        assert!(read.reference_month.is_none(), "BTG usa a inferência da aplicação");
        assert!(!read.transactions.is_empty());
    }

    // T018 — one invoice reader per extension, both banks registered.
    #[test]
    fn dispatches_by_extension_per_bank() {
        assert_eq!(invoice_reader_for(Path::new("/x/fatura.xlsx")).unwrap().bank(), "BTG");
        assert_eq!(invoice_reader_for(Path::new("/x/FATURA.XLSX")).unwrap().bank(), "BTG");
        assert_eq!(invoice_reader_for(Path::new("/x/fatura.pdf")).unwrap().bank(), "Santander");
        assert_eq!(invoice_reader_for(Path::new("/x/FATURA.PDF")).unwrap().bank(), "Santander");
        assert!(invoice_reader_for(Path::new("/x/extrato.xls")).is_none());
        assert!(invoice_reader_for(Path::new("/x/sem_extensao")).is_none());
    }
}
