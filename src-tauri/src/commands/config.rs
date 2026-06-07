use std::sync::Mutex;
use tauri::{Manager, State};

use crate::domain::AppConfig;
use crate::infrastructure::config_store::ConfigStore;

#[tauri::command]
pub async fn get_config(config: State<'_, Mutex<AppConfig>>) -> Result<AppConfig, String> {
    Ok(config.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub async fn save_config(
    new_config: AppConfig,
    config: State<'_, Mutex<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let config_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    let store = ConfigStore::new(config_path);
    store.save(&new_config)?;
    *config.lock().map_err(|e| e.to_string())? = new_config;
    Ok(())
}
