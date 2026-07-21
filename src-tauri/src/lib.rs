mod application;
mod commands;
pub mod domain;
pub mod infrastructure;

use std::sync::Mutex;
use tauri::Manager;

use application::store::shared_store_with;
use infrastructure::db::{new_shared_db, Database};
use commands::{
    bank::{clear_bank_entries, import_bank_statement, list_bank_entries, preview_bank_statement, remove_bank_entry},
    categories::{add_category_keyword, override_transaction_category, recategorize_invoices_cmd, remove_transaction_override},
    config::{get_config, save_config},
    dashboard::{get_dashboard_cmd, get_year_summary_cmd, list_invoices, remove_invoice},
    import::import_invoices,
    inflation::{fetch_ipca, get_inflation},
    manual_entries::{add_manual_entry, list_manual_entries, remove_manual_entry, update_manual_entry},
    payslips::{import_payslip, list_payslips, remove_payslip, save_payslip},
    secrets::{clear_saved_password, has_saved_password},
    transactions::list_all_transactions,
};

use infrastructure::config_store::ConfigStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // SQLite is the single source of truth for invoices, transactions,
            // rules, overrides and manual entries.
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let _ = std::fs::create_dir_all(&data_dir);
            let db_path = data_dir.join("financas.db");
            let mut db = Database::open(&db_path).expect("falha ao abrir banco de dados");

            // One-time migration: seed the DB from a legacy config.json if the DB is fresh.
            let mut config = if db.config_is_empty() {
                let config_path = app
                    .path()
                    .app_config_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join("config.json");
                let legacy = ConfigStore::new(config_path).load();
                let _ = db.save_config(&legacy);
                legacy
            } else {
                db.load_config().unwrap_or_default()
            };

            let mut invoices = db.load_invoices().unwrap_or_default();

            // Prune orphan transaction overrides (their transaction no longer exists),
            // so stale overrides don't accumulate across re-imports.
            {
                let ids: std::collections::HashSet<String> = invoices
                    .iter()
                    .flat_map(|i| i.transactions.iter().map(|t| t.id.to_string()))
                    .collect();
                let before = config.transaction_overrides.len();
                config.transaction_overrides.retain(|k, _| ids.contains(k));
                if config.transaction_overrides.len() != before {
                    let _ = db.save_config(&config);
                }
            }

            // Recategorize on startup so keyword/rule improvements always take effect.
            // Per-transaction overrides win over the rules.
            {
                let categorizer = if config.category_rules.is_empty() {
                    crate::domain::Categorizer::with_defaults()
                } else {
                    crate::domain::Categorizer::new(config.category_rules.clone())
                };
                let mut changed = false;
                for inv in invoices.iter_mut() {
                    for tx in inv.transactions.iter_mut() {
                        let new_cat = match config.transaction_overrides.get(&tx.id.to_string()) {
                            Some(ov) => ov.clone(),
                            None => categorizer.categorize(&tx.description),
                        };
                        if tx.category != new_cat {
                            tx.category = new_cat;
                            changed = true;
                        }
                    }
                }
                if changed {
                    let _ = db.save_all(&invoices);
                }
            }

            app.manage(Mutex::new(config));
            app.manage(shared_store_with(invoices));
            app.manage(new_shared_db(db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_invoices,
            preview_bank_statement,
            import_bank_statement,
            list_bank_entries,
            remove_bank_entry,
            clear_bank_entries,
            fetch_ipca,
            get_inflation,
            get_dashboard_cmd,
            get_year_summary_cmd,
            list_invoices,
            remove_invoice,
            get_config,
            save_config,
            recategorize_invoices_cmd,
            add_category_keyword,
            override_transaction_category,
            remove_transaction_override,
            list_all_transactions,
            list_manual_entries,
            add_manual_entry,
            update_manual_entry,
            remove_manual_entry,
            has_saved_password,
            clear_saved_password,
            import_payslip,
            save_payslip,
            list_payslips,
            remove_payslip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
