use std::sync::Mutex;
use tauri::State;

use crate::application::{recategorize::recategorize_invoices, store::SharedStore};
use crate::domain::{AppConfig, CategoryRule};
use crate::infrastructure::db::{persist, persist_config, SharedDb};

#[tauri::command]
pub async fn recategorize_invoices_cmd(
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<usize, String> {
    let config = config.lock().map_err(|e| e.to_string())?.clone();
    let changed = recategorize_invoices(&store, &config);
    let snapshot = store.lock().map_err(|e| e.to_string())?.list_owned();
    persist(&db, &snapshot);
    Ok(changed)
}

/// Add a keyword to a category's rules and immediately recategorize all invoices.
/// This is the core action of the Mapeamento screen: categorizing a merchant turns
/// its name into a keyword that applies to every matching transaction, now and future.
#[tauri::command]
pub async fn add_category_keyword(
    keyword: String,
    category: String,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
    db: State<'_, SharedDb>,
) -> Result<usize, String> {
    let kw = keyword.trim().to_string();
    let cat = category.trim().to_string();
    if kw.is_empty() || cat.is_empty() {
        return Err("Palavra-chave e categoria não podem ficar vazias.".into());
    }

    let cfg_snapshot = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        let kw_up = kw.to_uppercase();
        match cfg.category_rules.iter_mut().find(|r| r.category == cat) {
            Some(rule) => {
                if !rule.keywords.iter().any(|k| k.to_uppercase() == kw_up) {
                    rule.keywords.push(kw.clone());
                }
            }
            None => {
                let max_priority = cfg.category_rules.iter().map(|r| r.priority).max().unwrap_or(0);
                cfg.category_rules.push(CategoryRule {
                    keywords: vec![kw.clone()],
                    category: cat.clone(),
                    priority: max_priority + 10,
                });
            }
        }
        cfg.clone()
    };
    persist_config(&db, &cfg_snapshot)?;

    let changed = recategorize_invoices(&store, &cfg_snapshot);
    let snapshot = store.lock().map_err(|e| e.to_string())?.list_owned();
    persist(&db, &snapshot);
    Ok(changed)
}

#[tauri::command]
pub async fn override_transaction_category(
    transaction_id: String,
    category: String,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
    db: State<'_, SharedDb>,
) -> Result<(), String> {
    if category.trim().is_empty() {
        return Err("category must not be empty".into());
    }

    {
        let cfg = {
            let mut cfg = config.lock().map_err(|e| e.to_string())?;
            cfg.transaction_overrides.insert(transaction_id.clone(), category.clone());
            cfg.clone()
        };
        persist_config(&db, &cfg)?;
    }

    let snapshot = {
        let mut store_lock = store.lock().map_err(|e| e.to_string())?;
        store_lock.update_transaction_category(&transaction_id, &category);
        store_lock.list_owned()
    };
    persist(&db, &snapshot);

    Ok(())
}

#[tauri::command]
pub async fn remove_transaction_override(
    transaction_id: String,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
    db: State<'_, SharedDb>,
) -> Result<(), String> {
    use crate::domain::categorizer::Categorizer;

    let (categorizer, cfg_snapshot) = {
        let mut cfg = config.lock().map_err(|e| e.to_string())?;
        cfg.transaction_overrides.remove(&transaction_id);
        let categorizer = if cfg.category_rules.is_empty() {
            Categorizer::with_defaults()
        } else {
            Categorizer::new(cfg.category_rules.clone())
        };
        (categorizer, cfg.clone())
    };
    persist_config(&db, &cfg_snapshot)?;

    let snapshot = {
        let mut store_guard = store.lock().map_err(|e| e.to_string())?;
        store_guard.for_each_transaction_mut(|tx| {
            if tx.id.to_string() == transaction_id {
                tx.category = categorizer.categorize(&tx.description);
                true
            } else {
                false
            }
        });
        store_guard.list_owned()
    };
    persist(&db, &snapshot);

    Ok(())
}
