//! Application glue for recurring categories: turns imported data (card invoices +
//! bank statements) into [`Observation`]s, then uses the pure `domain::recurring`
//! functions to derive fixed expenses, baselines, per-category info and suggestions.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::bank_statement::BankEntry;
use crate::domain::invoice::Invoice;
use crate::domain::manual_entry::EntryKind;
use crate::domain::recurring::{
    baseline, derive_month, detect_suggestions, DerivedFixed, FixedOrigin, Observation,
    RecurringCategory, RecurringSuggestion, BASELINE_MONTHS,
};

const DETECT_WINDOW: usize = 4;
const DETECT_MIN_MONTHS: u32 = 3;
const DETECT_CV_MAX: f64 = 0.15;
/// Amounts within this coefficient of variation are shown as a fixed value;
/// above it, as a varying (média) value.
const VARIES_CV: f64 = 0.05;

/// Realized expense observations from card charges (origin Fatura) and bank
/// statement debits (origin Extrato). Card amounts keep their sign (reversals are
/// negative) so a category+month nets out; bank debits are taken as positive.
pub fn build_observations(invoices: &[Invoice], bank: &[BankEntry]) -> Vec<Observation> {
    let mut out = Vec::new();
    for inv in invoices {
        for t in &inv.transactions {
            let month = t.date.format("%Y-%m").to_string();
            // Card charges are always expenses (reversals keep their negative sign).
            out.push(Observation::new(month, t.category.clone(), t.amount, FixedOrigin::Fatura, EntryKind::Expense));
        }
    }
    for b in bank {
        // Keep credit/debit so income categories derive as renda recorrente.
        let kind = if b.kind == "income" { EntryKind::Income } else { EntryKind::Expense };
        out.push(Observation::new(b.month.clone(), b.category.clone(), b.amount.abs(), FixedOrigin::Extrato, kind));
    }
    out
}

/// Distinct months present in the observations, ascending.
pub fn observation_months(obs: &[Observation]) -> Vec<String> {
    let set: BTreeSet<String> = obs.iter().map(|o| o.month.clone()).collect();
    set.into_iter().collect()
}

/// Per-category info for the "Categorias & Regras" screen: baseline, origin, and
/// whether the amount varies month to month.
#[derive(Debug, Clone, Serialize)]
pub struct RecurringCategoryInfo {
    pub category: String,
    pub start_month: Option<String>,
    pub end_month: Option<String>,
    /// Computed average of the last months (data-driven baseline). None if no history.
    pub baseline: Option<String>,
    /// User-set base value override (editable), if any.
    pub base_amount: Option<String>,
    /// Dominant origin of the most recent realized month, if any.
    pub origin: Option<FixedOrigin>,
    /// True when monthly amounts vary meaningfully (show "média/varia" vs "valor fixo").
    pub varies: bool,
}

pub fn recurring_category_infos(cats: &[RecurringCategory], obs: &[Observation]) -> Vec<RecurringCategoryInfo> {
    cats.iter()
        .map(|c| {
            let base = baseline(c, "9999-12", obs, BASELINE_MONTHS);
            // per-month totals for this category (in vigência)
            let mut per_month: BTreeMap<String, Decimal> = BTreeMap::new();
            for o in obs.iter().filter(|o| o.category == c.category && c.active_in(&o.month)) {
                *per_month.entry(o.month.clone()).or_insert(Decimal::ZERO) += o.amount;
            }
            let origin = per_month
                .keys()
                .next_back()
                .and_then(|m| dominant_origin(m, &c.category, obs));
            let varies = coefficient_of_variation(per_month.values().copied()).map(|cv| cv > VARIES_CV).unwrap_or(false);
            RecurringCategoryInfo {
                category: c.category.clone(),
                start_month: c.start_month.clone(),
                end_month: c.end_month.clone(),
                baseline: base.map(|d| d.to_string()),
                base_amount: c.base_amount.map(|d| d.to_string()),
                origin,
                varies,
            }
        })
        .collect()
}

/// Fixed expenses derived for a single month (realized where present, else baseline).
pub fn fixed_for_month(month: &str, cats: &[RecurringCategory], obs: &[Observation]) -> Vec<DerivedFixed> {
    derive_month(month, cats, obs)
}

/// Suggestions of categories that look recurring, given the current recurring set
/// and dismissed targets.
pub fn suggestions(obs: &[Observation], cats: &[RecurringCategory], dismissed: &[String]) -> Vec<RecurringSuggestion> {
    let months = observation_months(obs);
    detect_suggestions(obs, &months, cats, dismissed, DETECT_WINDOW, DETECT_MIN_MONTHS, DETECT_CV_MAX)
}

fn dominant_origin(month: &str, category: &str, obs: &[Observation]) -> Option<FixedOrigin> {
    let mut saw_extrato = false;
    let mut saw_any = false;
    for o in obs.iter().filter(|o| o.month == month && o.category == category) {
        saw_any = true;
        if o.origin == FixedOrigin::Extrato {
            saw_extrato = true;
        }
    }
    if !saw_any {
        None
    } else if saw_extrato {
        Some(FixedOrigin::Extrato)
    } else {
        Some(FixedOrigin::Fatura)
    }
}

fn coefficient_of_variation(vals: impl Iterator<Item = Decimal>) -> Option<f64> {
    use rust_decimal::prelude::ToPrimitive;
    let xs: Vec<f64> = vals.map(|d| d.to_f64().unwrap_or(0.0)).collect();
    if xs.len() < 2 {
        return None;
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    if mean <= 0.0 {
        return None;
    }
    let var = xs.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / xs.len() as f64;
    Some(var.sqrt() / mean)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recurring::RecurringCategory;
    use rust_decimal_macros::dec;

    fn ob(m: &str, c: &str, a: Decimal, o: FixedOrigin) -> Observation {
        Observation::new(m, c, a, o, EntryKind::Expense)
    }

    #[test]
    fn info_baseline_and_varies() {
        let cats = vec![RecurringCategory::ongoing("Água")];
        let obs = vec![
            ob("2026-03", "Água", dec!(90), FixedOrigin::Extrato),
            ob("2026-04", "Água", dec!(100), FixedOrigin::Extrato),
            ob("2026-05", "Água", dec!(110), FixedOrigin::Extrato),
        ];
        let info = recurring_category_infos(&cats, &obs);
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].baseline.as_deref(), Some("100"));
        assert_eq!(info[0].origin, Some(FixedOrigin::Extrato));
        assert!(info[0].varies);
    }

    #[test]
    fn info_fixed_value_does_not_vary() {
        let cats = vec![RecurringCategory::ongoing("Aluguel")];
        let obs = vec![
            ob("2026-04", "Aluguel", dec!(2000), FixedOrigin::Extrato),
            ob("2026-05", "Aluguel", dec!(2000), FixedOrigin::Extrato),
        ];
        let info = recurring_category_infos(&cats, &obs);
        assert!(!info[0].varies);
    }

    #[test]
    fn build_observations_keeps_bank_income_and_expense_kinds() {
        let bank = vec![
            BankEntry {
                id: "1".into(), bank: "BTG".into(), account: "x".into(), date: "2026-06-05".into(),
                month: "2026-06".into(), description: "Aluguel".into(), category: "Aluguel".into(),
                amount: dec!(-2000), kind: "expense".into(),
            },
            BankEntry {
                id: "2".into(), bank: "BTG".into(), account: "x".into(), date: "2026-06-01".into(),
                month: "2026-06".into(), description: "Bolsa".into(), category: "Bolsa".into(),
                amount: dec!(5000), kind: "income".into(),
            },
        ];
        let obs = build_observations(&[], &bank);
        assert_eq!(obs.len(), 2);
        let aluguel = obs.iter().find(|o| o.category == "Aluguel").unwrap();
        assert_eq!(aluguel.amount, dec!(2000));
        assert_eq!(aluguel.kind, EntryKind::Expense);
        let bolsa = obs.iter().find(|o| o.category == "Bolsa").unwrap();
        assert_eq!(bolsa.amount, dec!(5000));
        assert_eq!(bolsa.kind, EntryKind::Income);
    }
}
