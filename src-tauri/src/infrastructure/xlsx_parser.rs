use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Arquivo protegido por senha. Abra no Excel/Numbers, remova a proteção e salve novamente.")]
    Encrypted,
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

pub fn parse_xlsx(path: &Path) -> Result<ParsedSheet, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::IoError(e.to_string()))?;

    if is_encrypted(&bytes) {
        return Err(ParseError::Encrypted);
    }

    let mut workbook = open_workbook_auto(path)
        .map_err(|e| ParseError::IoError(e.to_string()))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names.first().ok_or(ParseError::EmptySheet)?.clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| ParseError::IoError(e.to_string()))?;

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
