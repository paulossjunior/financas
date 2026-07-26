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
use crate::domain::transaction::Transaction;
use crate::infrastructure::btg_mapper::{map_sheet_to_transactions, ParseWarning};
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

/// Strategy interface: how to read one bank's card invoice file.
pub trait InvoiceReader: Sync {
    /// Bank name exactly as persisted in `Invoice.bank` ("BTG").
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
    ) -> Result<(Vec<Transaction>, Vec<ParseWarning>), InvoiceReadError>;
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
    ) -> Result<(Vec<Transaction>, Vec<ParseWarning>), InvoiceReadError> {
        let sheet = parse_xlsx(path, password).map_err(|e| match e {
            ParseError::Encrypted => InvoiceReadError::Encrypted,
            ParseError::WrongPassword => InvoiceReadError::WrongPassword,
            ParseError::InvalidFormat(s) => InvoiceReadError::InvalidFormat(s),
            ParseError::IoError(s) => InvoiceReadError::Io(s),
            ParseError::EmptySheet => InvoiceReadError::Empty,
        })?;
        map_sheet_to_transactions(&sheet, invoice_id, categorizer)
            .map_err(|e| InvoiceReadError::InvalidFormat(e.to_string()))
    }
}

/// Every registered invoice reader, in dispatch order.
pub static INVOICE_READERS: [&dyn InvoiceReader; 1] = [&BtgInvoiceReader];

/// Pick the strategy for a file by its extension. `None` = no bank issues invoices
/// in this format.
pub fn invoice_reader_for(path: &Path) -> Option<&'static dyn InvoiceReader> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    INVOICE_READERS.iter().copied().find(|r| r.extensions().contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_btg_for_xlsx_only() {
        assert_eq!(invoice_reader_for(Path::new("/x/fatura.xlsx")).unwrap().bank(), "BTG");
        assert_eq!(invoice_reader_for(Path::new("/x/FATURA.XLSX")).unwrap().bank(), "BTG");
        assert!(invoice_reader_for(Path::new("/x/extrato.pdf")).is_none());
        assert!(invoice_reader_for(Path::new("/x/sem_extensao")).is_none());
    }
}
