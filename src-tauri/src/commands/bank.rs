//! Commands: bank statement (extrato) import at the Tauri boundary — preview, save,
//! list, remove and recategorize bank entries.

use std::collections::HashSet;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::domain::bank_statement::{classify_statement, BankEntry, ClassifiedEntry, ParsedStatement};
use crate::domain::{AppConfig, Categorizer};
use crate::infrastructure::statement_reader::{statement_reader_for, supported_formats};
use crate::infrastructure::db::SharedDb;

#[derive(Debug, Serialize)]
pub struct StatementPreview {
    /// Which bank the file came from — detected from the file, not asked of the user.
    pub bank: String,
    pub holder: String,
    pub account: String,
    pub included: Vec<ClassifiedEntry>,
    pub excluded: Vec<ClassifiedEntry>,
}

/// Strategy dispatch: the registry picks the reader for the file (Banestes ships
/// PDF, BTG a spreadsheet); each strategy fills `ParsedStatement.bank` itself.
fn read_statement(path: &str) -> Result<ParsedStatement, String> {
    match statement_reader_for(path) {
        Some(reader) => reader.read(path),
        None => Err(format!("Formato não suportado. Use {}.", supported_formats())),
    }
}

/// Read + classify a statement file (no persistence).
fn classify_all(
    path: &str,
    config: &State<'_, Mutex<AppConfig>>,
    db: &State<'_, SharedDb>,
) -> Result<(ParsedStatement, Vec<ClassifiedEntry>), String> {
    let parsed = read_statement(path)?;
    let rules = config.lock().map_err(|e| e.to_string())?.category_rules.clone();
    let cz = if rules.is_empty() { Categorizer::with_defaults() } else { Categorizer::new(rules) };
    let payslip_months: HashSet<String> = db
        .lock()
        .map_err(|e| e.to_string())?
        .load_payslips()
        .unwrap_or_default()
        .iter()
        .map(|p| p.month.clone())
        .collect();
    let classified = classify_statement(&parsed, &cz, &payslip_months);
    Ok((parsed, classified))
}

/// Preview what will be imported (included) and what is dropped (excluded + reason).
#[tauri::command]
pub async fn preview_bank_statement(
    path: String,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<StatementPreview, String> {
    let (parsed, classified) = classify_all(&path, &config, &db)?;
    let (included, excluded): (Vec<_>, Vec<_>) = classified.into_iter().partition(|c| c.included);
    Ok(StatementPreview {
        bank: parsed.bank,
        holder: parsed.holder,
        account: parsed.account,
        included,
        excluded,
    })
}

/// Import: classify + persist the included entries (dedup). Returns how many were saved.
#[tauri::command]
pub async fn import_bank_statement(
    path: String,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<usize, String> {
    let (parsed, classified) = classify_all(&path, &config, &db)?;
    let entries: Vec<BankEntry> = classified
        .iter()
        .filter(|c| c.included)
        .map(|c| BankEntry::from_classified(c, &parsed.bank, &parsed.account))
        .collect();
    let n = entries.len();
    db.lock().map_err(|e| e.to_string())?.save_bank_entries(&entries)?;
    Ok(n)
}

/// Save the (possibly re-categorized) included entries from a preview. Dedup by id.
#[tauri::command]
pub async fn save_bank_statement(
    bank: String,
    account: String,
    entries: Vec<ClassifiedEntry>,
    db: State<'_, SharedDb>,
) -> Result<usize, String> {
    let items: Vec<BankEntry> = entries
        .iter()
        .filter(|c| c.included)
        .map(|c| BankEntry::from_classified(c, &bank, &account))
        .collect();
    let n = items.len();
    db.lock().map_err(|e| e.to_string())?.save_bank_entries(&items)?;
    Ok(n)
}

/// Change the category of an already-imported entry.
#[tauri::command]
pub async fn set_bank_entry_category(id: String, category: String, db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.update_bank_entry_category(&id, &category)
}

#[tauri::command]
pub async fn list_bank_entries(db: State<'_, SharedDb>) -> Result<Vec<BankEntry>, String> {
    db.lock().map_err(|e| e.to_string())?.load_bank_entries()
}

#[tauri::command]
pub async fn remove_bank_entry(id: String, db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.remove_bank_entry(&id)
}

#[tauri::command]
pub async fn clear_bank_entries(db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.clear_bank_entries()
}
