//! Commands: list all imported transactions at the Tauri boundary.

use tauri::State;

use crate::application::store::SharedStore;
use crate::domain::Transaction;

#[tauri::command]
pub async fn list_all_transactions(
    store: State<'_, SharedStore>,
) -> Result<Vec<Transaction>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.list_all_transactions())
}
