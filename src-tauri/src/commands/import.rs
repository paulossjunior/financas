//! Commands: import BTG card invoices (`.xlsx`) at the Tauri boundary — handles the
//! optional password and persists the result.

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::application::{
    import_folder::{import_from_folder, FolderImportSummary},
    import_invoice::{import_invoice, ImportResult},
    store::SharedStore,
};
use crate::domain::AppConfig;
use crate::infrastructure::db::{persist, persist_config, SharedDb};
use crate::infrastructure::invoice_reader::invoice_reader_for;
use crate::infrastructure::secrets;

#[tauri::command]
pub async fn import_invoices(
    paths: Vec<String>,
    password: Option<String>,
    remember: Option<bool>,
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<Vec<ImportResult>, String> {
    let mut results = vec![];
    let cfg = config.lock().map_err(|e| e.to_string())?.clone();
    // Which banks' invoices were imported with the explicit password — `remember`
    // must save the credential under the right bank's keychain entry.
    let mut banks_imported: Vec<&'static str> = vec![];

    for path_str in paths {
        let path = PathBuf::from(&path_str);
        // The bank owns the password: each file resolves its own credential
        // (BTG .xlsx vs Santander .pdf), falling back to that bank's saved one so
        // remembered passwords import silently without a prompt.
        let bank = invoice_reader_for(&path).map(|r| r.bank());
        let saved = match (password.is_none(), bank) {
            (true, Some(b)) => secrets::get_password_for(b),
            _ => None,
        };
        let effective = password.as_deref().or(saved.as_deref());

        let mut store_lock = store.lock().map_err(|e| e.to_string())?;
        match import_invoice(&path, &mut store_lock, &cfg, effective) {
            Ok(result) => {
                results.push(result);
                if let Some(b) = bank {
                    if !banks_imported.contains(&b) {
                        banks_imported.push(b);
                    }
                }
            }
            Err(e) => {
                // Password codes carry the bank: banks have different passwords, so
                // the UI must ask for (and label) the right one per file.
                let code = e.to_string();
                let enriched = match (&code[..], bank) {
                    ("ENCRYPTED_FILE" | "WRONG_PASSWORD", Some(b)) => format!("{code}:{b}"),
                    _ => code,
                };
                // Persist what already imported before surfacing the error, so the
                // store and the database never drift apart mid-batch.
                drop(store_lock);
                let snapshot = store.lock().map_err(|e| e.to_string())?.list_owned();
                persist(&db, &snapshot);
                return Err(enriched);
            }
        }
    }

    let snapshot = store.lock().map_err(|e| e.to_string())?.list_owned();
    persist(&db, &snapshot);

    // Only persist an explicitly-supplied password that just decrypted successfully,
    // under the keychain entry of each bank it actually worked for.
    if remember.unwrap_or(false) {
        if let Some(p) = password.as_deref() {
            for bank in banks_imported {
                if let Err(e) = secrets::save_password_for(bank, p) {
                    eprintln!("[secrets] falha ao salvar senha ({bank}): {e}");
                }
            }
        }
    }

    Ok(results)
}

/// Set (or clear, with `None`/empty) the auto-import folder. When a valid folder is
/// given, scans and imports it immediately and returns the summary. Returns `None`
/// when the feature is cleared.
#[tauri::command]
pub async fn set_import_directory(
    dir: Option<String>,
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<Option<FolderImportSummary>, String> {
    let dir = dir.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    // Update + persist config (release the config lock before touching the DB lock).
    let new_cfg = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        cfg.import_directory = dir.clone();
        cfg.clone()
    };
    persist_config(&db, &new_cfg)?;

    let Some(dir) = dir else {
        return Ok(None);
    };
    let path = PathBuf::from(&dir);
    if !path.is_dir() {
        return Err("IMPORT_DIR_INVALID".into());
    }
    let password = secrets::get_password();
    let summary = import_from_folder(&path, &db, &store, &new_cfg, password.as_deref());
    Ok(Some(summary))
}

/// Read (and clear) the summary of the auto-import that ran at app startup, so the UI
/// can show it once. `None` when nothing ran (no folder configured) or already read.
#[tauri::command]
pub async fn get_startup_import_summary(
    cell: State<'_, Mutex<Option<FolderImportSummary>>>,
) -> Result<Option<FolderImportSummary>, String> {
    Ok(cell.lock().map_err(|e| e.to_string())?.take())
}
