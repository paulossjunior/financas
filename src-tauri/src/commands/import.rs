//! Commands: import BTG card invoices (`.xlsx`) at the Tauri boundary — handles the
//! optional password and persists the result.

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::application::{
    import_invoice::{import_invoice, ImportResult},
    store::SharedStore,
};
use crate::domain::AppConfig;
use crate::infrastructure::db::{persist, SharedDb};
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

    // When the caller sends no password, fall back to the one saved in the keychain,
    // so remembered passwords import silently without a prompt.
    let saved = if password.is_none() {
        secrets::get_password()
    } else {
        None
    };
    let effective = password.as_deref().or(saved.as_deref());

    for path_str in paths {
        let path = PathBuf::from(&path_str);
        let mut store_lock = store.lock().map_err(|e| e.to_string())?;
        match import_invoice(&path, &mut store_lock, &cfg, effective) {
            Ok(result) => results.push(result),
            Err(e) => return Err(e.to_string()),
        }
    }

    let snapshot = store.lock().map_err(|e| e.to_string())?.list_owned();
    persist(&db, &snapshot);

    // Only persist an explicitly-supplied password that just decrypted successfully.
    if remember.unwrap_or(false) {
        if let Some(p) = password.as_deref() {
            if let Err(e) = secrets::save_password(p) {
                eprintln!("[secrets] falha ao salvar senha: {e}");
            }
        }
    }

    Ok(results)
}
