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

/// Category spend weights (name, total) + total income from the current data, used to
/// reweight the IPCA groups. Empty/zero when there is no data yet.
fn category_weights(
    store: &State<'_, SharedStore>,
    config: &State<'_, Mutex<AppConfig>>,
    db: &State<'_, SharedDb>,
) -> Result<(Vec<(String, Decimal)>, Decimal), String> {
    let mut manual = config.lock().map_err(|e| e.to_string())?.manual_entries.clone();
    let (payslips, bank) = {
        let d = db.lock().map_err(|e| e.to_string())?;
        (d.load_payslips().unwrap_or_default(), d.load_bank_entries().unwrap_or_default())
    };
    manual.extend(bank.iter().map(|b| b.to_manual_entry()));
    let store_lock = store.lock().map_err(|e| e.to_string())?;
    match get_dashboard(&store_lock, &manual, &payslips, DashboardFilter::default()) {
        Ok(d) => {
            let cats = d
                .categories
                .iter()
                .map(|c| (c.name.clone(), Decimal::from_str(&c.net_total).unwrap_or_default()))
                .collect();
            let income = Decimal::from_str(&d.total_income).unwrap_or_default();
            Ok((cats, income))
        }
        Err(_) => Ok((Vec::new(), Decimal::ZERO)), // NO_DATA → no weights; personal == general
    }
}

fn build(cache: InflationCache, cats: &[(String, Decimal)]) -> InflationData {
    let (personal, diff) = compute_personal_inflation(cats, &cache.groups, cache.headline.month);
    InflationData {
        available: true,
        headline: Some(cache.headline),
        groups: cache.groups,
        series: cache.series,
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
    let (cats, _income) = category_weights(&store, &config, &db)?;
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
    let (cats, _income) = category_weights(&store, &config, &db)?;
    Ok(build(cache, &cats))
}

/// Detailed personal-inflation breakdown (contributions, official comparison, basket
/// and income impact, optional behavioral simulation). Uses the cached indices; per
/// category the mapped IPCA group's monthly variation, else the general index (with
/// provenance). Returns None when there is no cache or no spending. No network.
#[tauri::command]
pub async fn get_personal_inflation_detail(
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<Option<crate::domain::personal_inflation::PersonalInflationResult>, String> {
    use crate::domain::inflation::map_category_to_group;
    use crate::domain::personal_inflation::{compute, CategoryInput, WeightMode, DEFAULT_BEHAVIORAL_COEFFICIENT};
    use rust_decimal::prelude::ToPrimitive;

    let cache_json = db.lock().map_err(|e| e.to_string())?.load_inflation_cache()?;
    let Some(json) = cache_json else {
        return Ok(None); // no indices cached yet
    };
    let cache: InflationCache = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let (cats, income) = category_weights(&store, &config, &db)?;

    // Percent → decimal rate (IPCA groups/headline are stored in percent, e.g. 0.63 = 0,63%).
    let general = cache.headline.month.to_f64().unwrap_or(0.0) / 100.0;
    let group_rate = |name: &str| -> f64 {
        cache
            .groups
            .iter()
            .find(|g| g.name == name)
            .map(|g| g.month_var.to_f64().unwrap_or(0.0) / 100.0)
            .unwrap_or(general)
    };

    let inputs: Vec<CategoryInput> = cats
        .into_iter()
        .filter(|(_, amt)| *amt > Decimal::ZERO)
        .map(|(name, amt)| {
            let (inflacao, provenance) = match map_category_to_group(&name) {
                Some(g) => (group_rate(g), None),
                None => (general, Some(format!("Sem grupo do IPCA para «{name}» — usou o IPCA geral."))),
            };
            CategoryInput { category: name, gasto: amt, base_gasto: None, inflacao, provenance }
        })
        .collect();

    if inputs.is_empty() {
        return Ok(None);
    }

    match compute(&inputs, general, income, Some(DEFAULT_BEHAVIORAL_COEFFICIENT), WeightMode::Current) {
        Ok(res) => Ok(Some(res)),
        Err(e) => Err(e.to_string()),
    }
}
