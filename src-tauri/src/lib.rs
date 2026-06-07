mod application;
mod commands;
pub mod domain;
pub mod infrastructure;

use std::sync::Mutex;
use tauri::Manager;

use application::store::new_shared_store;
use commands::{
    config::{get_config, save_config},
    dashboard::{get_dashboard_cmd, list_invoices, remove_invoice},
    import::import_invoices,
};

use infrastructure::config_store::ConfigStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_path = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("config.json");
            let store = ConfigStore::new(config_path);
            let config = store.load();
            app.manage(Mutex::new(config));
            app.manage(new_shared_store());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_invoices,
            get_dashboard_cmd,
            list_invoices,
            remove_invoice,
            get_config,
            save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
