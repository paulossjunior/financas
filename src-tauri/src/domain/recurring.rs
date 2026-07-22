//! Recurring categories: derive monthly fixed expenses from real imported data,
//! compute a baseline for months with no data yet, and suggest which categories
//! look recurring. Pure domain logic — no Tauri, no DB, deterministic.
//!
//! Model:
//! - A [`RecurringCategory`] marks a category as a fixed monthly expense, optionally
//!   bounded by a *vigência* (`start_month`..`end_month`, inclusive, "YYYY-MM").
//! - "Contas fixas" for a month are **derived** from the realized imported entries
//!   ([`Observation`]s coming from the bank statement / card invoice) in those
//!   categories. When a month has no realized data yet, the [`baseline`] (average of
//!   the last N months) fills in so the card ceiling still works before importing.
//! - A manual fixo in a recurring category is *superseded* when a realized entry
//!   exists for the same category+month — same anti-duplication pattern the payslip
//!   uses to supersede the manual salary. Nothing is ever counted twice.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default number of past months averaged for the baseline.
pub const BASELINE_MONTHS: usize = 3;

/// A category flagged as a recurring (fixed) monthly expense.
/// `start_month`/`end_month` ("YYYY-MM") bound a *finite* recurrence (e.g. a
/// therapist paid for 3 months); `None` on either side means open-ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurringCategory {
    pub category: String,
    #[serde(default)]
    pub start_month: Option<String>,
    #[serde(default)]
    pub end_month: Option<String>,
    /// User-set base value for months without imported data. Overrides the computed
    /// average baseline; realized imported data still wins over it.
    #[serde(default)]
    pub base_amount: Option<Decimal>,
}

impl RecurringCategory {
    pub fn ongoing(category: impl Into<String>) -> Self {
        Self { category: category.into(), start_month: None, end_month: None, base_amount: None }
    }

    /// Whether this recurrence is in effect during `month` ("YYYY-MM").
    /// Month strings are zero-padded, so lexicographic comparison == chronological.
    pub fn active_in(&self, month: &str) -> bool {
        let after_start = self.start_month.as_deref().is_none_or(|s| month >= s);
        let before_end = self.end_month.as_deref().is_none_or(|e| month <= e);
        after_start && before_end
    }
}

/// Where a realized fixed expense came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixedOrigin {
    /// Bank statement debit.
    Extrato,
    /// Credit-card invoice charge.
    Fatura,
    /// Hand-entered fixed expense (fallback).
    Manual,
    /// Estimated from the baseline (no realized data for the month yet).
    Baseline,
}

/// One realized, categorized expense observation from imported data.
#[derive(Debug, Clone)]
pub struct Observation {
    pub month: String,
    pub category: String,
    /// Positive expense amount (reversals already netted by the caller).
    pub amount: Decimal,
    pub origin: FixedOrigin,
}

impl Observation {
    pub fn new(month: impl Into<String>, category: impl Into<String>, amount: Decimal, origin: FixedOrigin) -> Self {
        Self { month: month.into(), category: category.into(), amount, origin }
    }
}

/// A derived fixed expense for a single (category, month).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedFixed {
    pub category: String,
    pub month: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub origin: FixedOrigin,
    /// True when `amount` is a baseline estimate, not realized data.
    pub is_baseline: bool,
}

/// Sum realized observations of one recurring category in one month.
/// Returns the total and the dominant origin (Extrato wins over Fatura when both
/// exist, since the statement is the primary data source). `None` if nothing realized.
fn realized_for(month: &str, category: &str, obs: &[Observation]) -> Option<(Decimal, FixedOrigin)> {
    let mut total = Decimal::ZERO;
    let mut saw_extrato = false;
    let mut saw_any = false;
    for o in obs.iter().filter(|o| o.month == month && o.category == category) {
        total += o.amount;
        saw_any = true;
        if o.origin == FixedOrigin::Extrato {
            saw_extrato = true;
        }
    }
    if !saw_any {
        return None;
    }
    let origin = if saw_extrato { FixedOrigin::Extrato } else { FixedOrigin::Fatura };
    Some((total, origin))
}

/// Baseline for a recurring category as of `upto_month` (exclusive): the average of
/// the category's realized monthly totals over the most recent [`BASELINE_MONTHS`]
/// months that have data and fall strictly before `upto_month` and within the
/// recurrence vigência. Uses fewer months when less history exists; `None` if none.
pub fn baseline(cat: &RecurringCategory, upto_month: &str, obs: &[Observation], n: usize) -> Option<Decimal> {
    // Sum realized amounts per month (only months before upto_month and in vigência).
    let mut per_month: BTreeMap<String, Decimal> = BTreeMap::new();
    for o in obs.iter().filter(|o| o.category == cat.category && o.month.as_str() < upto_month && cat.active_in(&o.month)) {
        *per_month.entry(o.month.clone()).or_insert(Decimal::ZERO) += o.amount;
    }
    if per_month.is_empty() {
        return None;
    }
    // BTreeMap iterates ascending by month; take the last `n` (most recent).
    let recent: Vec<Decimal> = per_month.values().rev().take(n.max(1)).copied().collect();
    let count = Decimal::from(recent.len() as u64);
    let sum: Decimal = recent.into_iter().sum();
    Some(sum / count)
}

/// Derive the fixed expenses for `month` across all recurring categories:
/// realized total when the month has imported data, otherwise the baseline estimate.
/// Categories whose vigência does not cover `month` are excluded.
pub fn derive_month(month: &str, cats: &[RecurringCategory], obs: &[Observation]) -> Vec<DerivedFixed> {
    let mut out = Vec::new();
    for cat in cats.iter().filter(|c| c.active_in(month)) {
        if let Some((amount, origin)) = realized_for(month, &cat.category, obs) {
            out.push(DerivedFixed { category: cat.category.clone(), month: month.to_string(), amount, origin, is_baseline: false });
        } else if let Some(amount) = cat.base_amount.or_else(|| baseline(cat, month, obs, BASELINE_MONTHS)) {
            // User-set base value takes precedence over the computed average; both are estimates.
            out.push(DerivedFixed { category: cat.category.clone(), month: month.to_string(), amount, origin: FixedOrigin::Baseline, is_baseline: true });
        }
        // else: recurring category with no realized data and no history → nothing this month.
    }
    out.sort_by(|a, b| a.category.cmp(&b.category));
    out
}

/// Whether a manual fixo for (`category`, `month`) is superseded by realized imported
/// data — i.e. the category is recurring, active that month, and has an observation.
/// When true, the manual entry must be dropped so the expense counts exactly once.
pub fn is_manual_superseded(category: &str, month: &str, cats: &[RecurringCategory], obs: &[Observation]) -> bool {
    cats.iter().any(|c| c.category == category && c.active_in(month))
        && realized_for(month, category, obs).is_some()
}

/// A suggestion to mark a category as recurring, inferred from history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecurringSuggestion {
    pub category: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub avg: Decimal,
    pub months_seen: u32,
}

/// Suggest categories that look recurring: present in at least `min_months` of the
/// most recent `window` months with low value variation (coefficient of variation
/// ≤ `cv_max`). Already-recurring and dismissed categories are excluded. `months`
/// is the sorted ascending list of the reference months to consider (most recent last).
pub fn detect_suggestions(
    obs: &[Observation],
    months: &[String],
    already: &[RecurringCategory],
    dismissed: &[String],
    window: usize,
    min_months: u32,
    cv_max: f64,
) -> Vec<RecurringSuggestion> {
    let recent: Vec<&String> = months.iter().rev().take(window).collect();
    let recent_set: std::collections::BTreeSet<&str> = recent.iter().map(|s| s.as_str()).collect();

    // category -> month -> total
    let mut per_cat: BTreeMap<String, BTreeMap<String, Decimal>> = BTreeMap::new();
    for o in obs.iter().filter(|o| recent_set.contains(o.month.as_str())) {
        *per_cat
            .entry(o.category.clone())
            .or_default()
            .entry(o.month.clone())
            .or_insert(Decimal::ZERO) += o.amount;
    }

    let mut out = Vec::new();
    for (cat, by_month) in per_cat {
        if cat == "Outros" || cat.is_empty() {
            continue;
        }
        if already.iter().any(|c| c.category == cat) || dismissed.iter().any(|d| d == &cat) {
            continue;
        }
        let months_seen = by_month.len() as u32;
        if months_seen < min_months {
            continue;
        }
        let vals: Vec<f64> = by_month.values().map(decimal_to_f64).collect();
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        if mean <= 0.0 {
            continue;
        }
        let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
        let cv = var.sqrt() / mean;
        if cv > cv_max {
            continue;
        }
        let sum: Decimal = by_month.values().copied().sum();
        let avg = sum / Decimal::from(by_month.len() as u64);
        out.push(RecurringSuggestion { category: cat, avg, months_seen });
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.avg));
    out
}

fn decimal_to_f64(d: &Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn obs(month: &str, cat: &str, amt: Decimal, origin: FixedOrigin) -> Observation {
        Observation::new(month, cat, amt, origin)
    }

    #[test]
    fn active_in_respects_vigencia() {
        let c = RecurringCategory { category: "Psicólogo".into(), start_month: Some("2026-01".into()), end_month: Some("2026-03".into()), base_amount: None };
        assert!(!c.active_in("2025-12"));
        assert!(c.active_in("2026-01"));
        assert!(c.active_in("2026-03"));
        assert!(!c.active_in("2026-04"));
    }

    #[test]
    fn ongoing_is_always_active() {
        let c = RecurringCategory::ongoing("Aluguel");
        assert!(c.active_in("2020-01"));
        assert!(c.active_in("2099-12"));
    }

    #[test]
    fn derive_uses_realized_when_present() {
        let cats = vec![RecurringCategory::ongoing("Aluguel")];
        let o = vec![obs("2026-06", "Aluguel", dec!(2000), FixedOrigin::Extrato)];
        let d = derive_month("2026-06", &cats, &o);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].amount, dec!(2000));
        assert_eq!(d[0].origin, FixedOrigin::Extrato);
        assert!(!d[0].is_baseline);
    }

    #[test]
    fn derive_sums_multiple_entries_same_category() {
        let cats = vec![RecurringCategory::ongoing("Energia")];
        let o = vec![
            obs("2026-06", "Energia", dec!(120), FixedOrigin::Extrato),
            obs("2026-06", "Energia", dec!(95), FixedOrigin::Extrato),
        ];
        let d = derive_month("2026-06", &cats, &o);
        assert_eq!(d[0].amount, dec!(215));
    }

    #[test]
    fn extrato_wins_origin_over_fatura() {
        let cats = vec![RecurringCategory::ongoing("Internet")];
        let o = vec![
            obs("2026-06", "Internet", dec!(100), FixedOrigin::Fatura),
            obs("2026-06", "Internet", dec!(20), FixedOrigin::Extrato),
        ];
        let d = derive_month("2026-06", &cats, &o);
        assert_eq!(d[0].origin, FixedOrigin::Extrato);
        assert_eq!(d[0].amount, dec!(120));
    }

    #[test]
    fn derive_falls_back_to_baseline_when_no_realized() {
        let cats = vec![RecurringCategory::ongoing("Água")];
        let o = vec![
            obs("2026-03", "Água", dec!(90), FixedOrigin::Extrato),
            obs("2026-04", "Água", dec!(100), FixedOrigin::Extrato),
            obs("2026-05", "Água", dec!(110), FixedOrigin::Extrato),
        ];
        // June has no data → baseline = avg(90,100,110) = 100
        let d = derive_month("2026-06", &cats, &o);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].amount, dec!(100));
        assert_eq!(d[0].origin, FixedOrigin::Baseline);
        assert!(d[0].is_baseline);
    }

    #[test]
    fn baseline_uses_last_n_months_only() {
        let cats = RecurringCategory::ongoing("Luz");
        let o = vec![
            obs("2026-01", "Luz", dec!(1000), FixedOrigin::Extrato), // outside last 3
            obs("2026-03", "Luz", dec!(100), FixedOrigin::Extrato),
            obs("2026-04", "Luz", dec!(200), FixedOrigin::Extrato),
            obs("2026-05", "Luz", dec!(300), FixedOrigin::Extrato),
        ];
        // upto June, last 3 = mar/apr/may = avg(100,200,300)=200 (jan ignored)
        assert_eq!(baseline(&cats, "2026-06", &o, 3), Some(dec!(200)));
    }

    #[test]
    fn user_base_amount_overrides_computed_baseline() {
        let mut cat = RecurringCategory::ongoing("Aluguel");
        cat.base_amount = Some(dec!(2500));
        let cats = vec![cat];
        // history would average to 2000, but the user base (2500) wins for an unimported month
        let o = vec![
            obs("2026-04", "Aluguel", dec!(2000), FixedOrigin::Extrato),
            obs("2026-05", "Aluguel", dec!(2000), FixedOrigin::Extrato),
        ];
        let d = derive_month("2026-06", &cats, &o);
        assert_eq!(d[0].amount, dec!(2500));
        assert!(d[0].is_baseline);
    }

    #[test]
    fn realized_still_wins_over_user_base_amount() {
        let mut cat = RecurringCategory::ongoing("Aluguel");
        cat.base_amount = Some(dec!(2500));
        let cats = vec![cat];
        let o = vec![obs("2026-06", "Aluguel", dec!(2000), FixedOrigin::Extrato)];
        let d = derive_month("2026-06", &cats, &o);
        assert_eq!(d[0].amount, dec!(2000));
        assert!(!d[0].is_baseline);
    }

    #[test]
    fn baseline_none_without_history() {
        let cats = RecurringCategory::ongoing("Luz");
        assert_eq!(baseline(&cats, "2026-06", &[], 3), None);
    }

    #[test]
    fn baseline_partial_history() {
        let cats = RecurringCategory::ongoing("Luz");
        let o = vec![obs("2026-05", "Luz", dec!(150), FixedOrigin::Extrato)];
        assert_eq!(baseline(&cats, "2026-06", &o, 3), Some(dec!(150)));
    }

    #[test]
    fn finite_recurrence_dropped_after_end() {
        let cats = vec![RecurringCategory { category: "Psicólogo".into(), start_month: Some("2026-01".into()), end_month: Some("2026-03".into()), base_amount: None }];
        let o = vec![obs("2026-02", "Psicólogo", dec!(400), FixedOrigin::Extrato)];
        // In vigência
        assert_eq!(derive_month("2026-02", &cats, &o).len(), 1);
        // After end: excluded even though there's history for baseline
        assert_eq!(derive_month("2026-04", &cats, &o).len(), 0);
    }

    #[test]
    fn manual_superseded_when_realized_exists() {
        let cats = vec![RecurringCategory::ongoing("Aluguel")];
        let o = vec![obs("2026-06", "Aluguel", dec!(2000), FixedOrigin::Extrato)];
        assert!(is_manual_superseded("Aluguel", "2026-06", &cats, &o));
    }

    #[test]
    fn manual_kept_when_no_realized() {
        let cats = vec![RecurringCategory::ongoing("Seguro")];
        let o = vec![obs("2026-06", "Aluguel", dec!(2000), FixedOrigin::Extrato)];
        // Seguro has no realized entry → manual stays (fallback, e.g. débito automático)
        assert!(!is_manual_superseded("Seguro", "2026-06", &cats, &o));
    }

    #[test]
    fn manual_kept_when_category_not_recurring() {
        let cats: Vec<RecurringCategory> = vec![];
        let o = vec![obs("2026-06", "Aluguel", dec!(2000), FixedOrigin::Extrato)];
        assert!(!is_manual_superseded("Aluguel", "2026-06", &cats, &o));
    }

    #[test]
    fn detect_suggests_stable_monthly_category() {
        let months: Vec<String> = ["2026-03", "2026-04", "2026-05", "2026-06"].iter().map(|s| s.to_string()).collect();
        let o = vec![
            obs("2026-03", "Academia", dec!(110), FixedOrigin::Fatura),
            obs("2026-04", "Academia", dec!(110), FixedOrigin::Fatura),
            obs("2026-05", "Academia", dec!(109), FixedOrigin::Fatura),
            obs("2026-06", "Academia", dec!(110), FixedOrigin::Fatura),
        ];
        let s = detect_suggestions(&o, &months, &[], &[], 4, 3, 0.25);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].category, "Academia");
        assert_eq!(s[0].months_seen, 4);
    }

    #[test]
    fn detect_skips_high_variation() {
        let months: Vec<String> = ["2026-03", "2026-04", "2026-05", "2026-06"].iter().map(|s| s.to_string()).collect();
        let o = vec![
            obs("2026-03", "Lazer", dec!(10), FixedOrigin::Fatura),
            obs("2026-04", "Lazer", dec!(500), FixedOrigin::Fatura),
            obs("2026-05", "Lazer", dec!(30), FixedOrigin::Fatura),
            obs("2026-06", "Lazer", dec!(900), FixedOrigin::Fatura),
        ];
        assert!(detect_suggestions(&o, &months, &[], &[], 4, 3, 0.25).is_empty());
    }

    #[test]
    fn detect_skips_infrequent() {
        let months: Vec<String> = ["2026-03", "2026-04", "2026-05", "2026-06"].iter().map(|s| s.to_string()).collect();
        let o = vec![
            obs("2026-05", "Curso", dec!(200), FixedOrigin::Fatura),
            obs("2026-06", "Curso", dec!(200), FixedOrigin::Fatura),
        ];
        // only 2 of 4 months → below min_months=3
        assert!(detect_suggestions(&o, &months, &[], &[], 4, 3, 0.25).is_empty());
    }

    #[test]
    fn detect_excludes_already_recurring_and_dismissed() {
        let months: Vec<String> = ["2026-03", "2026-04", "2026-05", "2026-06"].iter().map(|s| s.to_string()).collect();
        let mk = |c: &str| vec![
            obs("2026-03", c, dec!(100), FixedOrigin::Fatura),
            obs("2026-04", c, dec!(100), FixedOrigin::Fatura),
            obs("2026-05", c, dec!(100), FixedOrigin::Fatura),
        ];
        let mut o = mk("JáRecorrente");
        o.extend(mk("Ignorada"));
        let already = vec![RecurringCategory::ongoing("JáRecorrente")];
        let dismissed = vec!["Ignorada".to_string()];
        assert!(detect_suggestions(&o, &months, &already, &dismissed, 4, 3, 0.25).is_empty());
    }

    #[test]
    fn detect_excludes_outros() {
        let months: Vec<String> = ["2026-04", "2026-05", "2026-06"].iter().map(|s| s.to_string()).collect();
        let o = vec![
            obs("2026-04", "Outros", dec!(100), FixedOrigin::Fatura),
            obs("2026-05", "Outros", dec!(100), FixedOrigin::Fatura),
            obs("2026-06", "Outros", dec!(100), FixedOrigin::Fatura),
        ];
        assert!(detect_suggestions(&o, &months, &[], &[], 4, 3, 0.25).is_empty());
    }
}
