use std::sync::Mutex;
use tauri::{Manager, State};

use crate::application::{recategorize::recategorize_invoices, store::SharedStore};
use crate::domain::AppConfig;
use crate::infrastructure::config_store::ConfigStore;

#[tauri::command]
pub async fn recategorize_invoices_cmd(
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
) -> Result<usize, String> {
    let config = config.lock().map_err(|e| e.to_string())?.clone();
    Ok(recategorize_invoices(&store, &config))
}

#[tauri::command]
pub async fn override_transaction_category(
    transaction_id: String,
    category: String,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if category.trim().is_empty() {
        return Err("category must not be empty".into());
    }

    let config_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        cfg.transaction_overrides.insert(transaction_id.clone(), category.clone());
        ConfigStore::new(config_path).save(&cfg)?;
    }

    store.lock().map_err(|e| e.to_string())?.update_transaction_category(&transaction_id, &category);

    Ok(())
}

#[tauri::command]
pub async fn remove_transaction_override(
    transaction_id: String,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::domain::categorizer::Categorizer;

    let config_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    let categorizer = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        cfg.transaction_overrides.remove(&transaction_id);
        ConfigStore::new(config_path).save(&cfg)?;
        if cfg.category_rules.is_empty() {
            Categorizer::with_defaults()
        } else {
            Categorizer::new(cfg.category_rules.clone())
        }
    };

    let mut store_guard = store.lock().map_err(|e| e.to_string())?;
    store_guard.for_each_transaction_mut(|tx| {
        if tx.id.to_string() == transaction_id {
            tx.category = categorizer.categorize(&tx.description);
            true
        } else {
            false
        }
    });

    Ok(())
}
