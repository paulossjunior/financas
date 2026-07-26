//! Read a Banestes bank statement (PDF) and parse it.
//!
//! Thin I/O shell: PDF → text → domain. All grammar and integrity rules live in
//! `domain::banestes_statement`, so they are testable without a binary fixture
//! (the real statement carries personal data and never enters the repository).

use std::path::Path;

use crate::domain::banestes_statement::{is_banestes_statement, parse_banestes_text};
use crate::domain::bank_statement::ParsedStatement;
use crate::infrastructure::statement_reader::StatementReader;

/// Extract the text of a statement PDF. Errors are user-facing.
pub fn extract_text(path: &str) -> Result<String, String> {
    if !Path::new(path).exists() {
        return Err("Arquivo não encontrado.".into());
    }
    let text = pdf_extract::extract_text(path).map_err(|e| format!("Não consegui ler o PDF: {e}"))?;
    if text.trim().is_empty() {
        return Err("Este PDF não tem texto para ler (pode ser digitalizado ou protegido).".into());
    }
    Ok(text)
}

/// True when the PDF at `path` is a Banestes statement. Used by the folder scan to
/// tell an extrato apart from a payslip without importing anything.
pub fn is_banestes_pdf(path: &str) -> bool {
    extract_text(path).map(|t| is_banestes_statement(&t)).unwrap_or(false)
}

/// Read + parse a Banestes statement PDF.
pub fn read_statement(path: &str) -> Result<ParsedStatement, String> {
    let text = extract_text(path)?;
    let mut parsed = parse_banestes_text(&text)?;
    // Positions carry which file produced them (traceability; id ignores this).
    let filename = Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
    for p in &mut parsed.positions {
        p.source_file = filename.to_string();
    }
    Ok(parsed)
}

/// Strategy: the Banestes statement is a PDF whose extracted text carries the whole
/// grammar (see `domain::banestes_statement`).
pub struct BanestesStatementReader;

impl StatementReader for BanestesStatementReader {
    fn bank(&self) -> &'static str {
        "Banestes"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
    }

    /// A PDF is only ours if its text has the statement's structural markers — the
    /// user keeps payslip PDFs in the same folder. (Costs one extra text extraction
    /// before `read`; these PDFs are small and the folder scan runs at startup.)
    fn recognizes(&self, path: &str) -> bool {
        is_banestes_pdf(path)
    }

    fn read(&self, path: &str) -> Result<ParsedStatement, String> {
        read_statement(path)
    }
}
