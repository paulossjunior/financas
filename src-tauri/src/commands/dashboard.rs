//! Commands: month dashboard and annual summary, plus invoice listing/removal, at
//! the Tauri boundary.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use tauri::State;
use uuid::Uuid;

use crate::application::recurring_fixed::build_observations;
use crate::application::{get_dashboard::get_dashboard, store::SharedStore};
use crate::domain::bank_statement::BankEntry;
use crate::domain::manual_entry::{EntryKind, ManualEntry};
use crate::domain::recurring::{is_manual_superseded, RecurringCategory};
use crate::domain::{compute_year_summary, AppConfig, DashboardData, DashboardFilter, YearSummary};
use crate::infrastructure::db::{persist, SharedDb};

/// Fold bank entries into the manual-entry pipeline as recurring-aware fixed expenses:
/// - a user manual fixo is dropped when a realized recurring entry supersedes it
///   (same category + month), so nothing is counted twice;
/// - a bank expense in a recurring, in-vigência category is reclassified as a fixed
///   (recurring) expense instead of a one-off (avulso).
fn apply_recurring(
    mut manual: Vec<ManualEntry>,
    bank: &[BankEntry],
    invoices: &[crate::domain::invoice::Invoice],
    recurring_cats: &[RecurringCategory],
) -> Vec<ManualEntry> {
    let obs = build_observations(invoices, bank);
    manual.retain(|e| {
        !(e.kind == EntryKind::Expense
            && e.recurring
            && is_manual_superseded(&e.category, &e.month, recurring_cats, &obs))
    });
    manual.extend(bank.iter().map(|b| {
        let mut m = b.to_manual_entry();
        if m.kind == EntryKind::Expense
            && recurring_cats.iter().any(|c| c.category == m.category && c.active_in(&m.month))
        {
            m.recurring = true;
        }
        m
    }));
    manual
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceInfo {
    pub id: String,
    /// Bank that issued the invoice (stamped by the reader strategy on import).
    pub bank: String,
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
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<DashboardData, String> {
    let manual_entries = config.lock().map_err(|e| e.to_string())?.manual_entries.clone();
    let (payslips, bank, recurring_cats) = {
        let d = db.lock().map_err(|e| e.to_string())?;
        (
            d.load_payslips().unwrap_or_default(),
            d.load_bank_entries().unwrap_or_default(),
            d.load_recurring_categories().unwrap_or_default(),
        )
    };
    let invoices = store.lock().map_err(|e| e.to_string())?.list_owned();
    let manual_entries = apply_recurring(manual_entries, &bank, &invoices, &recurring_cats);
    let store_lock = store.lock().map_err(|e| e.to_string())?;
    get_dashboard(&store_lock, &manual_entries, &payslips, filter.unwrap_or_default())
}

#[tauri::command]
pub async fn get_year_summary_cmd(
    year_from: Option<i32>,
    year_to: Option<i32>,
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<YearSummary, String> {
    let manual = config.lock().map_err(|e| e.to_string())?.manual_entries.clone();
    let invoices = store.lock().map_err(|e| e.to_string())?.list_owned();
    let (payslips, bank, recurring_cats) = {
        let d = db.lock().map_err(|e| e.to_string())?;
        (
            d.load_payslips().unwrap_or_default(),
            d.load_bank_entries().unwrap_or_default(),
            d.load_recurring_categories().unwrap_or_default(),
        )
    };
    let manual = apply_recurring(manual, &bank, &invoices, &recurring_cats);
    Ok(compute_year_summary(&invoices, &manual, &payslips, year_from, year_to))
}

#[tauri::command]
pub async fn list_invoices(store: State<'_, SharedStore>) -> Result<Vec<InvoiceInfo>, String> {
    let store_lock = store.lock().map_err(|e| e.to_string())?;
    let infos: Vec<InvoiceInfo> = store_lock
        .list()
        .into_iter()
        .map(|inv| InvoiceInfo {
            id: inv.id.to_string(),
            bank: inv.bank.clone(),
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
    db: State<'_, SharedDb>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&invoice_id).map_err(|e| e.to_string())?;
    let removed = {
        let mut store_lock = store.lock().map_err(|e| e.to_string())?;
        store_lock.remove(&id)
    };
    if removed {
        let snapshot = store.lock().map_err(|e| e.to_string())?.list_owned();
        persist(&db, &snapshot);
        Ok(())
    } else {
        Err("INVOICE_NOT_FOUND".into())
    }
}
