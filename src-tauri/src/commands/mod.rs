//! Commands layer root: the `#[tauri::command]` boundary, one submodule per area.

pub mod backup;
pub mod bank;
pub mod categories;
pub mod config;
pub mod dashboard;
pub mod import;
pub mod inflation;
pub mod manual_entries;
pub mod payslips;
pub mod recurring;
pub mod secrets;
pub mod transactions;
