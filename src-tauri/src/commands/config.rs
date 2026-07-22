//! Commands: read and persist the app [`AppConfig`] at the Tauri boundary.

use std::sync::Mutex;
use tauri::State;

use crate::domain::AppConfig;
use crate::infrastructure::db::{persist_config, SharedDb};

#[tauri::command]
pub async fn get_config(config: State<'_, Mutex<AppConfig>>) -> Result<AppConfig, String> {
    Ok(config.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub async fn save_config(
    new_config: AppConfig,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<(), String> {
    persist_config(&db, &new_config)?;
    *config.lock().map_err(|e| e.to_string())? = new_config;
    Ok(())
}
