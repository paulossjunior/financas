use tauri::State;

use crate::application::recurring_fixed::{
    build_observations, fixed_for_month, recurring_category_infos, suggestions, RecurringCategoryInfo,
};
use crate::application::store::SharedStore;
use crate::domain::recurring::{DerivedFixed, RecurringSuggestion};
use crate::infrastructure::db::SharedDb;

/// Recurring categories with their baseline, origin and whether the amount varies —
/// feeds the "Categorias & Regras" table.
#[tauri::command]
pub async fn list_recurring_categories(
    store: State<'_, SharedStore>,
    db: State<'_, SharedDb>,
) -> Result<Vec<RecurringCategoryInfo>, String> {
    let invoices = { store.lock().map_err(|e| e.to_string())?.list_owned() };
    let d = db.lock().map_err(|e| e.to_string())?;
    let cats = d.load_recurring_categories()?;
    let bank = d.load_bank_entries().unwrap_or_default();
    let obs = build_observations(&invoices, &bank);
    Ok(recurring_category_infos(&cats, &obs))
}

/// Mark a category recurring (with optional vigência) or clear its recurrence.
#[tauri::command]
pub async fn set_category_recurring(
    category: String,
    recurring: bool,
    start_month: Option<String>,
    end_month: Option<String>,
    db: State<'_, SharedDb>,
) -> Result<(), String> {
    if category.trim().is_empty() {
        return Err("categoria não pode ficar vazia".into());
    }
    let mut d = db.lock().map_err(|e| e.to_string())?;
    d.set_recurring_category(category.trim(), recurring, start_month.as_deref(), end_month.as_deref())
}

/// Categories that look recurring (opt-in suggestions), excluding already-recurring
/// and dismissed ones.
#[tauri::command]
pub async fn recurring_suggestions(
    store: State<'_, SharedStore>,
    db: State<'_, SharedDb>,
) -> Result<Vec<RecurringSuggestion>, String> {
    let invoices = { store.lock().map_err(|e| e.to_string())?.list_owned() };
    let d = db.lock().map_err(|e| e.to_string())?;
    let cats = d.load_recurring_categories()?;
    let bank = d.load_bank_entries().unwrap_or_default();
    let dismissed = d.load_dismissed_suggestions()?;
    let obs = build_observations(&invoices, &bank);
    Ok(suggestions(&obs, &cats, &dismissed))
}

/// Dismiss a recurrence suggestion so it does not reappear.
#[tauri::command]
pub async fn dismiss_recurring_suggestion(target: String, db: State<'_, SharedDb>) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("alvo não pode ficar vazio".into());
    }
    let mut d = db.lock().map_err(|e| e.to_string())?;
    d.dismiss_suggestion(target.trim())
}

/// Derived fixed expenses for a month (realized where imported, else baseline) —
/// feeds "Fixos & Renda".
#[tauri::command]
pub async fn list_fixed_expenses(
    month: String,
    store: State<'_, SharedStore>,
    db: State<'_, SharedDb>,
) -> Result<Vec<DerivedFixed>, String> {
    let invoices = { store.lock().map_err(|e| e.to_string())?.list_owned() };
    let d = db.lock().map_err(|e| e.to_string())?;
    let cats = d.load_recurring_categories()?;
    let bank = d.load_bank_entries().unwrap_or_default();
    let obs = build_observations(&invoices, &bank);
    Ok(fixed_for_month(&month, &cats, &obs))
}
