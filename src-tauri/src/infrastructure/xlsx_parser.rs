//! Infrastructure: read `.xlsx` invoice files (calamine), transparently decrypting
//! password-protected workbooks (office-crypto) into raw rows.

use calamine::{open_workbook_auto, Data, Range, Reader, Xlsx};
use std::io::Cursor;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Arquivo protegido por senha. Informe a senha para abrir a fatura.")]
    Encrypted,
    #[error("Senha incorreta. Verifique a senha da fatura e tente novamente.")]
    WrongPassword,
    #[error("Formato inválido: colunas obrigatórias ausentes: {0}")]
    InvalidFormat(String),
    #[error("Erro ao abrir arquivo: {0}")]
    IoError(String),
    #[error("Planilha vazia ou sem dados")]
    EmptySheet,
}

pub struct RawRow {
    pub index: u32,
    pub cells: Vec<String>,
}

pub struct ParsedSheet {
    pub rows: Vec<RawRow>,
}

/// Parse a BTG xlsx invoice. When the file is password-protected (ECMA-376/OLE),
/// `password` is required — the bytes are decrypted in memory before parsing.
pub fn parse_xlsx(path: &Path, password: Option<&str>) -> Result<ParsedSheet, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::IoError(e.to_string()))?;

    if is_encrypted(&bytes) {
        let pw = match password {
            Some(p) if !p.trim().is_empty() => p,
            _ => return Err(ParseError::Encrypted),
        };
        let decrypted = office_crypto::decrypt_from_bytes(bytes, pw)
            .map_err(|e| ParseError::InvalidFormat(format!("decrypt: {e:?}")))?;
        let mut wb: Xlsx<_> = Xlsx::new(Cursor::new(decrypted))
            .map_err(|e| ParseError::InvalidFormat(e.to_string()))?;
        let sheet_name = wb.sheet_names().first().ok_or(ParseError::EmptySheet)?.clone();
        let range = wb
            .worksheet_range(&sheet_name)
            .map_err(|e| ParseError::IoError(e.to_string()))?;
        return range_to_sheet(range);
    }

    let mut workbook = open_workbook_auto(path)
        .map_err(|e| ParseError::IoError(e.to_string()))?;
    let sheet_name = workbook.sheet_names().first().ok_or(ParseError::EmptySheet)?.clone();
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| ParseError::IoError(e.to_string()))?;
    range_to_sheet(range)
}

fn range_to_sheet(range: Range<Data>) -> Result<ParsedSheet, ParseError> {
    let rows: Vec<RawRow> = range
        .rows()
        .enumerate()
        .filter_map(|(i, row)| {
            let cells: Vec<String> = row.iter().map(cell_to_string).collect();
            if cells.iter().all(|c| c.is_empty()) {
                return None;
            }
            Some(RawRow {
                index: (i + 1) as u32,
                cells,
            })
        })
        .collect();

    if rows.is_empty() {
        return Err(ParseError::EmptySheet);
    }

    Ok(ParsedSheet { rows })
}

fn is_encrypted(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && bytes[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => format!("{f}"),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => d
            .as_datetime()
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("ERROR({e:?})"),
        Data::Empty => String::new(),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_detection() {
        let encrypted_magic = [
            0xD0u8, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0x00, 0x00,
        ];
        assert!(is_encrypted(&encrypted_magic));
    }

    #[test]
    fn test_not_encrypted_zip_header() {
        let zip_magic = [0x50u8, 0x4B, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00];
        assert!(!is_encrypted(&zip_magic));
    }

}
