use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{transaction::{InstallmentInfo, Transaction}, categorizer::Categorizer};
use super::xlsx_parser::ParsedSheet;

#[derive(Debug, Error)]
pub enum MapperError {
    #[error("Nenhuma seção de transações BTG encontrada. Verifique se o arquivo é uma fatura BTG válida.")]
    NoTransactionSection,
}

#[derive(Debug)]
pub struct ParseWarning {
    pub row: u32,
    pub message: String,
}

pub fn map_sheet_to_transactions(
    sheet: &ParsedSheet,
    invoice_id: Uuid,
    categorizer: &Categorizer,
) -> Result<(Vec<Transaction>, Vec<ParseWarning>), MapperError> {
    let mut transactions = vec![];
    let mut warnings = vec![];

    // Collect indices of BTG transaction section headers
    let header_indices: Vec<usize> = sheet
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| is_transaction_header(&row.cells))
        .map(|(i, _)| i)
        .collect();

    if header_indices.is_empty() {
        return Err(MapperError::NoTransactionSection);
    }

    for header_idx in header_indices {
        let col_map = build_col_map(&sheet.rows[header_idx].cells);

        let date_col = match col_map.get("data") {
            Some(&i) => i,
            None => continue,
        };
        let desc_col = match col_map.get("descrição").or_else(|| col_map.get("descricao")) {
            Some(&i) => i,
            None => continue,
        };
        let val_col = match col_map.get("valor") {
            Some(&i) => i,
            None => continue,
        };

        for row in &sheet.rows[header_idx + 1..] {
            if is_transaction_header(&row.cells) {
                break;
            }

            let get = |idx: usize| -> String {
                row.cells.get(idx).map(|s| s.trim().to_string()).unwrap_or_default()
            };

            let date_str = get(date_col);
            let desc = get(desc_col);
            let amount_str = get(val_col);

            // Non-date rows (summaries, gaps) are skipped silently
            let date = match parse_date(&date_str) {
                Ok(d) => d,
                Err(_) => continue,
            };

            if desc.is_empty() {
                warnings.push(ParseWarning {
                    row: row.index,
                    message: "Descrição vazia — linha ignorada".into(),
                });
                continue;
            }

            let amount = match parse_decimal(&amount_str) {
                Ok(a) => a,
                Err(e) => {
                    warnings.push(ParseWarning {
                        row: row.index,
                        message: format!("Valor inválido '{amount_str}': {e}"),
                    });
                    continue;
                }
            };

            let category = categorizer.categorize(&desc);
            let installment = parse_installment_from_desc(&desc);

            transactions.push(Transaction::new(
                invoice_id,
                row.index,
                date,
                desc,
                amount,
                category,
                installment,
            ));
        }
    }

    Ok((transactions, warnings))
}

/// Returns true if the row is a BTG transaction section header.
/// Criterion: has 'Data', 'Descrição', and 'Código de autorização' columns.
fn is_transaction_header(cells: &[String]) -> bool {
    let normalized: Vec<String> = cells.iter().map(|s| normalize(s)).collect();
    normalized.iter().any(|c| c == "data")
        && normalized.iter().any(|c| c == "descrição" || c == "descricao")
        && normalized.iter().any(|c| c.contains("autoriza"))
}

fn build_col_map(cells: &[String]) -> HashMap<String, usize> {
    cells
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.is_empty())
        .map(|(i, s)| (normalize(s), i))
        .collect()
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    if s.is_empty() {
        return Err("empty".to_string());
    }
    for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d/%m/%y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s.trim(), fmt) {
            return Ok(d);
        }
    }
    Err(format!("formato desconhecido: {s}"))
}

fn parse_decimal(s: &str) -> Result<Decimal, String> {
    if s.is_empty() {
        return Err("empty".to_string());
    }
    let cleaned = s.trim().replace("R$", "").replace([' ', '\u{a0}'], "");
    let normalized = if cleaned.contains(',') && cleaned.contains('.') {
        cleaned.replace('.', "").replace(',', ".")
    } else if cleaned.contains(',') {
        cleaned.replace(',', ".")
    } else {
        cleaned
    };
    Decimal::from_str(&normalized).map_err(|e| e.to_string())
}

/// Extracts installment info from description "(N/M)" suffix.
/// Example: "Porto Seguro Seguros (6/10)" → InstallmentInfo { current: 6, total: 10 }
fn parse_installment_from_desc(desc: &str) -> Option<InstallmentInfo> {
    let paren_start = desc.rfind('(')?;
    let tail = &desc[paren_start + 1..];
    let close = tail.find(')')?;
    let inner = &tail[..close];
    let mut parts = inner.splitn(2, '/');
    let current = parts.next()?.trim().parse::<u8>().ok()?;
    let total = parts.next()?.trim().parse::<u8>().ok()?;
    Some(InstallmentInfo { current, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::xlsx_parser::RawRow;

    fn make_sheet(all_rows: Vec<Vec<&str>>) -> ParsedSheet {
        ParsedSheet {
            rows: all_rows
                .iter()
                .enumerate()
                .map(|(i, r)| RawRow {
                    index: (i + 1) as u32,
                    cells: r.iter().map(|s| s.to_string()).collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_btg_finds_transaction_section() {
        let sheet = make_sheet(vec![
            // metadata
            vec!["", "Fatura Cartão de Crédito", "", "", "", "", "Junho/2026", ""],
            // transaction section header
            vec!["", "Data", "Descrição", "", "Valor", "Tipo de compra", "Código de autorização", "Final Cartão"],
            // data row
            vec!["", "2026-05-07", "Ifood", "", "42.9", "Compra à vista", "AUTH123", "5623"],
        ]);
        let categorizer = Categorizer::with_defaults();
        let invoice_id = Uuid::new_v4();
        let (txs, warnings) = map_sheet_to_transactions(&sheet, invoice_id, &categorizer).unwrap();
        assert_eq!(txs.len(), 1);
        assert!(warnings.is_empty());
        assert_eq!(txs[0].description, "Ifood");
        assert_eq!(txs[0].category, "Alimentação");
    }

    #[test]
    fn test_btg_two_sections_merged() {
        let sheet = make_sheet(vec![
            // section 1 header (discounts)
            vec!["", "Data", "Descrição", "", "Valor", "Código de autorização", "Final Cartão", ""],
            // section 1 data
            vec!["", "2026-05-07", "Desconto Parcela", "", "-3.45", "HQJ9V4", "9986", ""],
            // summary row (no date)
            vec!["", "Total de compras", "", "", "13519.14", "", "", ""],
            // section 2 header (purchases with extra column)
            vec!["", "Data", "Descrição", "", "Valor", "Tipo de compra", "Código de autorização", "Final Cartão"],
            // section 2 data
            vec!["", "2026-05-10", "Supermercado", "", "150.00", "Compra à vista", "XYZ123", "5623"],
        ]);
        let categorizer = Categorizer::with_defaults();
        let invoice_id = Uuid::new_v4();
        let (txs, _) = map_sheet_to_transactions(&sheet, invoice_id, &categorizer).unwrap();
        assert_eq!(txs.len(), 2);
    }

    #[test]
    fn test_btg_no_section_returns_error() {
        let sheet = make_sheet(vec![
            vec!["", "Data", "Descrição", "", "Valor"],
            vec!["", "2026-05-07", "Pagamento de fatura", "", "-9717.22"],
        ]);
        let categorizer = Categorizer::with_defaults();
        let invoice_id = Uuid::new_v4();
        let result = map_sheet_to_transactions(&sheet, invoice_id, &categorizer);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_installment_from_desc() {
        let info = parse_installment_from_desc("Porto Seguro Seguros (6/10)").unwrap();
        assert_eq!(info.current, 6);
        assert_eq!(info.total, 10);

        let info2 = parse_installment_from_desc("Leroy Merlin (5/6)").unwrap();
        assert_eq!(info2.current, 5);
        assert_eq!(info2.total, 6);

        assert!(parse_installment_from_desc("Ifood").is_none());
        assert!(parse_installment_from_desc("Desconto Na Parcela 6 De 6 Loja").is_none());
    }

    #[test]
    fn test_parse_decimal_brazilian_format() {
        assert_eq!(
            parse_decimal("1.234,56").unwrap(),
            Decimal::from_str("1234.56").unwrap()
        );
        assert_eq!(
            parse_decimal("42,90").unwrap(),
            Decimal::from_str("42.90").unwrap()
        );
        assert_eq!(
            parse_decimal("-50.00").unwrap(),
            Decimal::from_str("-50.00").unwrap()
        );
    }
}
