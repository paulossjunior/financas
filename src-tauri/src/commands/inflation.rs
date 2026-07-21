use std::str::FromStr;
use std::sync::Mutex;

use rust_decimal::Decimal;
use tauri::State;

use crate::application::get_dashboard::get_dashboard;
use crate::application::store::SharedStore;
use crate::domain::inflation::{compute_personal_inflation, InflationCache};
use crate::domain::{AppConfig, DashboardFilter, InflationData};
use crate::infrastructure::db::SharedDb;
use crate::infrastructure::ibge::fetch_inflation;

/// Category spend weights (name, total) from the current data, used to reweight
/// the IPCA groups. Empty when there is no data yet.
fn category_weights(
    store: &State<'_, SharedStore>,
    config: &State<'_, Mutex<AppConfig>>,
    db: &State<'_, SharedDb>,
) -> Result<Vec<(String, Decimal)>, String> {
    let manual = config.lock().map_err(|e| e.to_string())?.manual_entries.clone();
    let payslips = db.lock().map_err(|e| e.to_string())?.load_payslips().unwrap_or_default();
    let store_lock = store.lock().map_err(|e| e.to_string())?;
    match get_dashboard(&store_lock, &manual, &payslips, DashboardFilter::default()) {
        Ok(d) => Ok(d
            .categories
            .iter()
            .map(|c| (c.name.clone(), Decimal::from_str(&c.net_total).unwrap_or_default()))
            .collect()),
        Err(_) => Ok(Vec::new()), // NO_DATA → no weights; personal == general
    }
}

fn build(cache: InflationCache, cats: &[(String, Decimal)]) -> InflationData {
    let (personal, diff) = compute_personal_inflation(cats, &cache.groups, cache.headline.month);
    InflationData {
        available: true,
        headline: Some(cache.headline),
        groups: cache.groups,
        personal_month: personal,
        personal_diff: diff,
        fetched_at: cache.fetched_at,
    }
}

/// Read the cached indices (offline) and compute personal inflation. No network.
#[tauri::command]
pub async fn get_inflation(
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<InflationData, String> {
    let cache_json = db.lock().map_err(|e| e.to_string())?.load_inflation_cache()?;
    let Some(json) = cache_json else {
        return Ok(InflationData::empty());
    };
    let cache: InflationCache = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let cats = category_weights(&store, &config, &db)?;
    Ok(build(cache, &cats))
}

/// OPT-IN: fetch the latest indices from the IBGE, persist locally, return the
/// updated data. On network/source failure the previous cache is preserved.
#[tauri::command]
pub async fn fetch_ipca(
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<InflationData, String> {
    let cache = fetch_inflation().await?;
    let json = serde_json::to_string(&cache).map_err(|e| e.to_string())?;
    db.lock()
        .map_err(|e| e.to_string())?
        .save_inflation_cache(&json, &cache.fetched_at)?;
    let cats = category_weights(&store, &config, &db)?;
    Ok(build(cache, &cats))
}
