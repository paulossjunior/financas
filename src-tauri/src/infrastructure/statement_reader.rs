//! Strategy: one bank-statement reader per bank, behind a single trait.
//!
//! Each bank ships a different file (Banestes: PDF; BTG: `.xls`/`.xlsx` spreadsheet)
//! but the rest of the app must not care which: dispatch picks the strategy by file
//! extension and every reader converges on the shared domain
//! [`ParsedStatement`] — with the bank carried in `.bank` — so classification,
//! categorization, dedup and persistence stay a single, bank-agnostic path.
//!
//! Adding a bank = implement [`StatementReader`] next to its reader module and add
//! one line to [`STATEMENT_READERS`] (recipe in `docs/MAINTENANCE.md`).

use std::path::Path;

use crate::domain::bank_statement::ParsedStatement;
use crate::infrastructure::banestes_statement::BanestesStatementReader;
use crate::infrastructure::btg_statement::BtgStatementReader;

/// Strategy interface: how to read one bank's statement file.
pub trait StatementReader: Sync {
    /// Bank name exactly as persisted in `BankEntry.bank` ("Banestes", "BTG").
    fn bank(&self) -> &'static str;

    /// Lowercase file extensions this reader accepts.
    fn extensions(&self) -> &'static [&'static str];

    /// Content sniff: is this file really this bank's statement — and not another
    /// document wearing the same extension (a payslip PDF, say)? The auto-import
    /// folder scan uses it to leave foreign files alone.
    fn recognizes(&self, path: &str) -> bool;

    /// Read + parse the file into the shared statement model.
    fn read(&self, path: &str) -> Result<ParsedStatement, String>;
}

/// Every registered reader, in dispatch order.
pub static STATEMENT_READERS: [&dyn StatementReader; 2] =
    [&BanestesStatementReader, &BtgStatementReader];

/// Pick the strategy for a file by its extension. `None` = no bank reads this format.
pub fn statement_reader_for(path: &str) -> Option<&'static dyn StatementReader> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    STATEMENT_READERS.iter().copied().find(|r| r.extensions().contains(&ext.as_str()))
}

/// User-facing list of the accepted formats, for "unsupported format" messages.
/// Built from the registry so it never drifts from the readers actually installed.
pub fn supported_formats() -> String {
    STATEMENT_READERS
        .iter()
        .map(|r| {
            let exts =
                r.extensions().iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join("/");
            format!("{} ({exts})", r.bank())
        })
        .collect::<Vec<_>>()
        .join(" ou ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_by_extension_case_insensitive() {
        assert_eq!(statement_reader_for("/x/extrato.pdf").unwrap().bank(), "Banestes");
        assert_eq!(statement_reader_for("/x/EXTRATO.PDF").unwrap().bank(), "Banestes");
        assert_eq!(statement_reader_for("/x/extrato.xls").unwrap().bank(), "BTG");
        assert_eq!(statement_reader_for("/x/extrato.XLSX").unwrap().bank(), "BTG");
        assert!(statement_reader_for("/x/extrato.csv").is_none());
        assert!(statement_reader_for("/x/sem_extensao").is_none());
    }

    #[test]
    fn supported_formats_lists_every_registered_bank() {
        let s = supported_formats();
        assert!(s.contains("Banestes (.pdf)"), "{s}");
        assert!(s.contains("BTG (.xls/.xlsx)"), "{s}");
    }
}
