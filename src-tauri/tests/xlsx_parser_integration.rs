use std::path::Path;
use chrono::Datelike;
use uuid::Uuid;

use financas_lib::domain::categorizer::Categorizer;
use financas_lib::infrastructure::{
    btg_mapper::map_sheet_to_transactions,
    xlsx_parser::parse_xlsx,
};

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/fixtures/sample_fatura.xlsx")
}

#[test]
fn test_parse_btg_fixture_returns_transactions() {
    let path = fixture_path();
    assert!(path.exists(), "Fixture not found: {}", path.display());

    let sheet = parse_xlsx(&path).expect("parse_xlsx failed");
    let categorizer = Categorizer::with_defaults();
    let invoice_id = Uuid::new_v4();

    let (txs, warnings) =
        map_sheet_to_transactions(&sheet, invoice_id, &categorizer).expect("map failed");

    assert!(!txs.is_empty(), "Expected transactions, got none");
    assert!(txs.len() >= 4, "Expected at least 4 transactions, got {}", txs.len());
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings.iter().map(|w| &w.message).collect::<Vec<_>>());
}

#[test]
fn test_fixture_transactions_have_valid_dates() {
    let path = fixture_path();
    let sheet = parse_xlsx(&path).unwrap();
    let categorizer = Categorizer::with_defaults();
    let invoice_id = Uuid::new_v4();
    let (txs, _) = map_sheet_to_transactions(&sheet, invoice_id, &categorizer).unwrap();

    for tx in &txs {
        // All dates must be in 2026 (our fixture dates)
        assert_eq!(tx.date.year(), 2026, "Unexpected year in date: {}", tx.date);
    }
}

#[test]
fn test_fixture_categories_inferred() {
    let path = fixture_path();
    let sheet = parse_xlsx(&path).unwrap();
    let categorizer = Categorizer::with_defaults();
    let invoice_id = Uuid::new_v4();
    let (txs, _) = map_sheet_to_transactions(&sheet, invoice_id, &categorizer).unwrap();

    let ifood_tx = txs.iter().find(|t| t.description.to_lowercase().contains("ifood"));
    assert!(ifood_tx.is_some(), "Ifood transaction not found");
    assert_eq!(ifood_tx.unwrap().category, "Alimentação");

    let uber_tx = txs.iter().find(|t| t.description.to_lowercase().contains("uber"));
    assert!(uber_tx.is_some(), "Uber transaction not found");
    assert_eq!(uber_tx.unwrap().category, "Transporte");
}

#[test]
fn test_fixture_reversal_detected() {
    let path = fixture_path();
    let sheet = parse_xlsx(&path).unwrap();
    let categorizer = Categorizer::with_defaults();
    let invoice_id = Uuid::new_v4();
    let (txs, _) = map_sheet_to_transactions(&sheet, invoice_id, &categorizer).unwrap();

    let reversals: Vec<_> = txs.iter().filter(|t| t.is_reversal).collect();
    assert!(!reversals.is_empty(), "Expected at least 1 reversal (Desconto Parcela)");
    assert!(reversals[0].amount < rust_decimal::Decimal::ZERO, "Reversal amount must be negative");
}

#[test]
fn test_fixture_installment_parsed_from_description() {
    let path = fixture_path();
    let sheet = parse_xlsx(&path).unwrap();
    let categorizer = Categorizer::with_defaults();
    let invoice_id = Uuid::new_v4();
    let (txs, _) = map_sheet_to_transactions(&sheet, invoice_id, &categorizer).unwrap();

    let ml_tx = txs.iter().find(|t| t.description.contains("Mercado Livre"));
    assert!(ml_tx.is_some(), "Mercado Livre transaction not found");
    let inst = ml_tx.unwrap().installment.as_ref().expect("Expected installment info");
    assert_eq!(inst.current, 2);
    assert_eq!(inst.total, 3);
}

#[test]
fn test_encrypted_file_returns_error() {
    use financas_lib::infrastructure::xlsx_parser::ParseError;
    // Construct fake OLE2 encrypted bytes
    let encrypted_bytes: Vec<u8> = vec![
        0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1,
        0x00, 0x00, 0x00, 0x00,
    ];
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &encrypted_bytes).unwrap();
    let result = parse_xlsx(tmp.path());
    assert!(matches!(result, Err(ParseError::Encrypted)), "Expected Encrypted error");
}
