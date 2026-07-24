//! Commands: database backup & restore at the Tauri boundary (feature 012).
//! Backup writes a consistent snapshot to a user-chosen folder; restore validates a
//! backup, saves a safety copy of the current data, swaps the file in, and reloads the
//! in-memory state (config + invoice store) so the UI reflects the restored data.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::store::SharedStore;
use crate::domain::AppConfig;
use crate::infrastructure::db::SharedDb;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    /// Full path of the backup file that was written.
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// Full path of the safety copy of the previous database (to revert if needed).
    pub backup_of_previous: String,
}

/// Back up the live database into `dest_dir`, returning the file written.
#[tauri::command]
pub async fn backup_database(
    dest_dir: String,
    db: State<'_, SharedDb>,
) -> Result<BackupResult, String> {
    let path = db
        .lock()
        .map_err(|e| e.to_string())?
        .backup_to(&PathBuf::from(dest_dir))?;
    Ok(BackupResult { path: path.to_string_lossy().to_string() })
}

/// Restore the database from `source_path`, then refresh in-memory state so the dashboard
/// (which reads the invoice store + config, not the file) reflects the restored data.
#[tauri::command]
pub async fn restore_database(
    source_path: String,
    db: State<'_, SharedDb>,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
) -> Result<RestoreResult, String> {
    let (safety, new_config, invoices) = {
        let mut guard = db.lock().map_err(|e| e.to_string())?;
        let safety = guard.restore_from(&PathBuf::from(source_path))?;
        let new_config = guard.load_config()?;
        let invoices = guard.load_invoices()?;
        (safety, new_config, invoices)
    };

    *config.lock().map_err(|e| e.to_string())? = new_config;
    store.lock().map_err(|e| e.to_string())?.replace_all(invoices);

    Ok(RestoreResult { backup_of_previous: safety.to_string_lossy().to_string() })
}
