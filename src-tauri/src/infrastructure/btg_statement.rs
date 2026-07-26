//! Read a BTG bank statement (.xls) into stringified rows and parse them.
//! Isolated here so other banks can add their own reader later; the domain
//! parsing/classification stays format-agnostic on the row grid.

use calamine::{open_workbook_auto, Data, Reader};

use crate::domain::bank_statement::{parse_statement_rows, ParsedStatement};
use crate::infrastructure::statement_reader::StatementReader;

fn cell_to_string(c: &Data) -> String {
    match c {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            // Whole numbers print without a trailing .0; decimals keep precision.
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(n) => n.to_string(),
        Data::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// Read + parse a statement file. Errors if the file can't be opened or no
/// recognizable transactions are found.
pub fn read_statement(path: &str) -> Result<ParsedStatement, String> {
    let mut wb = open_workbook_auto(path).map_err(|e| format!("Não consegui abrir o extrato: {e}"))?;
    let names = wb.sheet_names().to_owned();
    let sheet = names
        .iter()
        .find(|n| n.eq_ignore_ascii_case("Extrato"))
        .cloned()
        .or_else(|| names.first().cloned())
        .ok_or("Extrato vazio.")?;
    let range = wb
        .worksheet_range(&sheet)
        .map_err(|e| format!("Falha ao ler o extrato: {e}"))?;
    let rows: Vec<Vec<String>> = range
        .rows()
        .map(|r| r.iter().map(cell_to_string).collect())
        .collect();
    let mut parsed = parse_statement_rows(&rows);
    parsed.bank = "BTG".to_string();
    if parsed.entries.is_empty() {
        return Err("Não encontrei lançamentos no extrato (formato não reconhecido).".into());
    }
    Ok(parsed)
}

/// Strategy: the BTG statement is an `.xls`/`.xlsx` spreadsheet.
pub struct BtgStatementReader;

impl StatementReader for BtgStatementReader {
    fn bank(&self) -> &'static str {
        "BTG"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["xls", "xlsx"]
    }

    /// Extension is sniff enough here: a spreadsheet in the import folder was put
    /// there to be imported (invoice or statement — the folder scan tries the
    /// invoice reader first for `.xlsx`), and one that parses as neither is
    /// *reported*, not silently skipped. See `application/import_folder.rs`.
    fn recognizes(&self, _path: &str) -> bool {
        true
    }

    fn read(&self, path: &str) -> Result<ParsedStatement, String> {
        read_statement(path)
    }
}
