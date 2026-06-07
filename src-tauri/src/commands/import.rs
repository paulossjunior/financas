use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::application::{
    import_invoice::{import_invoice, ImportResult},
    store::SharedStore,
};
use crate::domain::AppConfig;

#[tauri::command]
pub async fn import_invoices(
    paths: Vec<String>,
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
) -> Result<Vec<ImportResult>, String> {
    let mut results = vec![];
    let cfg = config.lock().map_err(|e| e.to_string())?.clone();

    for path_str in paths {
        let path = PathBuf::from(&path_str);
        let mut store_lock = store.lock().map_err(|e| e.to_string())?;
        match import_invoice(&path, &mut store_lock, &cfg) {
            Ok(result) => results.push(result),
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(results)
}
