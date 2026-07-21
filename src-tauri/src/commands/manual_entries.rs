use std::str::FromStr;
use std::sync::Mutex;

use rust_decimal::Decimal;
use tauri::State;
use uuid::Uuid;

use crate::domain::manual_entry::{EntryKind, ManualEntry};
use crate::domain::AppConfig;
use crate::infrastructure::db::{persist_config, SharedDb};

fn validate(description: &str, amount: Decimal, category: &str) -> Result<(), String> {
    if description.trim().is_empty() {
        return Err("A descrição não pode ficar vazia.".into());
    }
    if category.trim().is_empty() {
        return Err("A categoria não pode ficar vazia.".into());
    }
    if amount <= Decimal::ZERO {
        return Err("O valor deve ser maior que zero.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn list_manual_entries(
    config: State<'_, Mutex<AppConfig>>,
) -> Result<Vec<ManualEntry>, String> {
    Ok(config.lock().map_err(|e| e.to_string())?.manual_entries.clone())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn add_manual_entry(
    kind: EntryKind,
    description: String,
    amount: String,
    category: String,
    month: String,
    recurring: bool,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<ManualEntry, String> {
    let amount = Decimal::from_str(amount.trim())
        .map_err(|_| "Valor inválido.".to_string())?;
    validate(&description, amount, &category)?;

    let entry = ManualEntry::new(
        kind,
        description.trim().to_string(),
        amount,
        category.trim().to_string(),
        month,
        recurring,
    );

    let snapshot = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        cfg.manual_entries.push(entry.clone());
        cfg.clone()
    };
    persist_config(&db, &snapshot)?;
    Ok(entry)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn update_manual_entry(
    id: String,
    kind: EntryKind,
    description: String,
    amount: String,
    category: String,
    month: String,
    recurring: bool,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<ManualEntry, String> {
    let target = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let amount = Decimal::from_str(amount.trim())
        .map_err(|_| "Valor inválido.".to_string())?;
    validate(&description, amount, &category)?;

    let (updated, snapshot) = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        let entry = cfg
            .manual_entries
            .iter_mut()
            .find(|e| e.id == target)
            .ok_or_else(|| "ENTRY_NOT_FOUND".to_string())?;
        entry.kind = kind;
        entry.description = description.trim().to_string();
        entry.amount = amount.abs();
        entry.category = category.trim().to_string();
        entry.month = month;
        entry.recurring = recurring;
        (entry.clone(), cfg.clone())
    };
    persist_config(&db, &snapshot)?;
    Ok(updated)
}

#[tauri::command]
pub async fn remove_manual_entry(
    id: String,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<(), String> {
    let target = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let snapshot = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        let before = cfg.manual_entries.len();
        cfg.manual_entries.retain(|e| e.id != target);
        if cfg.manual_entries.len() == before {
            return Err("ENTRY_NOT_FOUND".into());
        }
        cfg.clone()
    };
    persist_config(&db, &snapshot)?;
    Ok(())
}
