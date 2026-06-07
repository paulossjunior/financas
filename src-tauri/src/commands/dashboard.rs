use serde::{Deserialize, Serialize};

use tauri::State;
use uuid::Uuid;

use crate::application::{get_dashboard::get_dashboard, store::SharedStore};
use crate::domain::{DashboardData, DashboardFilter};

#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceInfo {
    pub id: String,
    pub filename: String,
    pub month: String,
    pub due_date: Option<String>,
    pub row_count: usize,
    pub imported_at: String,
}

#[tauri::command]
pub async fn get_dashboard_cmd(
    filter: Option<DashboardFilter>,
    store: State<'_, SharedStore>,
) -> Result<DashboardData, String> {
    let store_lock = store.lock().map_err(|e| e.to_string())?;
    get_dashboard(&store_lock, filter.unwrap_or_default())
}

#[tauri::command]
pub async fn list_invoices(store: State<'_, SharedStore>) -> Result<Vec<InvoiceInfo>, String> {
    let store_lock = store.lock().map_err(|e| e.to_string())?;
    let infos: Vec<InvoiceInfo> = store_lock
        .list()
        .into_iter()
        .map(|inv| InvoiceInfo {
            id: inv.id.to_string(),
            filename: inv.filename.clone(),
            month: inv.reference_month.to_string_iso(),
            due_date: inv.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            row_count: inv.transactions.len(),
            imported_at: inv.imported_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        })
        .collect();
    Ok(infos)
}

#[tauri::command]
pub async fn remove_invoice(
    invoice_id: String,
    store: State<'_, SharedStore>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&invoice_id).map_err(|e| e.to_string())?;
    let mut store_lock = store.lock().map_err(|e| e.to_string())?;
    if store_lock.remove(&id) {
        Ok(())
    } else {
        Err("INVOICE_NOT_FOUND".into())
    }
}
