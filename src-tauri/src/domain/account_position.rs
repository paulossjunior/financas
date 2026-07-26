//! Domain: account positions (stock) and statement coverage (pure).
//!
//! First *stock* concept in the app — everything else models flows. A position is
//! the balance a statement printed for an account/product at a base date; coverage
//! is the period a statement import spans. All derived rules (current position,
//! partial months, gaps, chain check) are pure functions over these facts, so
//! re-imports and out-of-order imports can never desynchronize a materialized flag.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Which product of the account a balance refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Product {
    Corrente,
    Poupanca,
}

impl Product {
    pub fn as_str(&self) -> &'static str {
        match self {
            Product::Corrente => "corrente",
            Product::Poupanca => "poupanca",
        }
    }

    pub fn from_str(s: &str) -> Option<Product> {
        match s {
            "corrente" => Some(Product::Corrente),
            "poupanca" => Some(Product::Poupanca),
            _ => None,
        }
    }
}

/// Balance a statement printed for one account/product at a base date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountPosition {
    /// Deterministic: same (bank, account, product, as_of) ⇒ same id (idempotent upsert).
    pub id: String,
    pub bank: String,
    pub account: String,
    pub product: Product,
    pub balance: Decimal,
    /// Base date of the balance: end of the covered period (Banestes) or the date of
    /// the last printed daily balance (BTG best effort).
    pub as_of: NaiveDate,
    /// Statement file that produced it (traceability; removed with the statement data).
    pub source_file: String,
}

impl AccountPosition {
    pub fn new(
        bank: &str,
        account: &str,
        product: Product,
        balance: Decimal,
        as_of: NaiveDate,
        source_file: &str,
    ) -> Self {
        let key = format!("position:{bank}:{account}:{}:{as_of}", product.as_str());
        Self {
            id: Uuid::new_v5(&Uuid::NAMESPACE_OID, key.as_bytes()).to_string(),
            bank: bank.to_string(),
            account: account.to_string(),
            product,
            balance,
            as_of,
            source_file: source_file.to_string(),
        }
    }
}

/// Period one statement import covers for an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    pub id: String,
    pub bank: String,
    pub account: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub source_file: String,
}

impl Coverage {
    pub fn new(
        bank: &str,
        account: &str,
        start: NaiveDate,
        end: NaiveDate,
        source_file: &str,
    ) -> Self {
        let key = format!("coverage:{bank}:{account}:{start}:{end}");
        Self {
            id: Uuid::new_v5(&Uuid::NAMESPACE_OID, key.as_bytes()).to_string(),
            bank: bank.to_string(),
            account: account.to_string(),
            start,
            end,
            source_file: source_file.to_string(),
        }
    }
}

/// Current position per (bank, account, product): the greatest `as_of` wins,
/// regardless of import order (a late-arriving old statement never demotes it).
pub fn current_positions(all: &[AccountPosition]) -> Vec<AccountPosition> {
    let mut best: Vec<AccountPosition> = Vec::new();
    for p in all {
        match best
            .iter_mut()
            .find(|b| b.bank == p.bank && b.account == p.account && b.product == p.product)
        {
            Some(b) => {
                if p.as_of > b.as_of {
                    *b = p.clone();
                }
            }
            None => best.push(p.clone()),
        }
    }
    best.sort_by(|a, b| (&a.bank, &a.account, a.product.as_str()).cmp(&(&b.bank, &b.account, b.product.as_str())));
    best
}

/// How much of a civil month the merged coverages span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonthCoverage {
    Full,
    /// Covered sub-ranges inside the month (merged, sorted). The common case is one
    /// prefix range like 01..25 — shown as "dados até 25/07".
    Partial(Vec<(NaiveDate, NaiveDate)>),
    None,
}

fn month_bounds(month: &str) -> Option<(NaiveDate, NaiveDate)> {
    let (y, m) = month.split_once('-')?;
    let (y, m): (i32, u32) = (y.parse().ok()?, m.parse().ok()?);
    let first = NaiveDate::from_ymd_opt(y, m, 1)?;
    let next = if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)?
    };
    Some((first, next.pred_opt()?))
}

/// Merge intervals (union — overlaps never double-count).
fn merge_ranges(mut ranges: Vec<(NaiveDate, NaiveDate)>) -> Vec<(NaiveDate, NaiveDate)> {
    ranges.sort();
    let mut out: Vec<(NaiveDate, NaiveDate)> = Vec::new();
    for (s, e) in ranges {
        match out.last_mut() {
            // Adjacent (next day) counts as continuous.
            Some(last) if s <= last.1.succ_opt().unwrap_or(last.1) => {
                if e > last.1 {
                    last.1 = e;
                }
            }
            _ => out.push((s, e)),
        }
    }
    out
}

/// Coverage status of `month` ("YYYY-MM") for one account's coverages.
pub fn month_coverage(covs: &[Coverage], month: &str) -> MonthCoverage {
    let Some((first, last)) = month_bounds(month) else { return MonthCoverage::None };
    let clipped: Vec<(NaiveDate, NaiveDate)> = covs
        .iter()
        .filter(|c| c.start <= last && c.end >= first)
        .map(|c| (c.start.max(first), c.end.min(last)))
        .collect();
    if clipped.is_empty() {
        return MonthCoverage::None;
    }
    let merged = merge_ranges(clipped);
    if merged.len() == 1 && merged[0] == (first, last) {
        MonthCoverage::Full
    } else {
        MonthCoverage::Partial(merged)
    }
}

/// Months ("YYYY-MM") with **no** coverage at all, between the month of the earliest
/// start and the month of the latest end. Partial months are not gaps.
pub fn coverage_gaps(covs: &[Coverage]) -> Vec<String> {
    let Some(first) = covs.iter().map(|c| c.start).min() else { return vec![] };
    let Some(last) = covs.iter().map(|c| c.end).max() else { return vec![] };
    let mut gaps = Vec::new();
    let (mut y, mut m) = (first.year(), first.month());
    loop {
        let month = format!("{y:04}-{m:02}");
        if month_coverage(covs, &month) == MonthCoverage::None {
            gaps.push(month);
        }
        if (y, m) >= (last.year(), last.month()) {
            break;
        }
        if m == 12 {
            y += 1;
            m = 1;
        } else {
            m += 1;
        }
    }
    gaps
}

/// Chain check: the new statement's printed "Saldo Anterior" must equal the balance
/// of the account's current position dated before `new_start` (Corrente product).
/// `positions` is the single account's positions (caller filters by account).
/// `None` = nothing to compare (first import) or it matches.
pub fn chain_warning(
    positions: &[AccountPosition],
    new_start: NaiveDate,
    saldo_anterior: Decimal,
) -> Option<String> {
    let prev = positions
        .iter()
        .filter(|p| p.product == Product::Corrente && p.as_of < new_start)
        .max_by_key(|p| p.as_of)?;
    if prev.balance == saldo_anterior {
        return None;
    }
    Some(format!(
        "O saldo anterior deste extrato (R$ {}) não bate com o saldo final do período \
         anterior (R$ {}, extrato até {}). Pode haver um extrato faltando entre eles.",
        format_brl(saldo_anterior),
        format_brl(prev.balance),
        prev.as_of.format("%d/%m/%Y"),
    ))
}

/// `1234.5` → `1.234,50` (same rendering as the statement reconciliation errors).
fn format_brl(v: Decimal) -> String {
    let s = format!("{:.2}", v);
    let (int, dec) = s.split_once('.').unwrap_or((s.as_str(), "00"));
    let neg = int.starts_with('-');
    let digits = int.trim_start_matches('-');
    let mut grouped = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let int: String = grouped.chars().rev().collect();
    format!("{}{},{}", if neg { "-" } else { "" }, int, dec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::from_str(s).unwrap()
    }
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    fn pos(product: Product, balance: &str, as_of: &str) -> AccountPosition {
        AccountPosition::new("Banestes", "44/123-4", product, dec(balance), d(as_of), "x.pdf")
    }
    fn cov(start: &str, end: &str) -> Coverage {
        Coverage::new("Banestes", "44/123-4", d(start), d(end), "x.pdf")
    }

    // T003 — deterministic identity
    #[test]
    fn ids_are_deterministic_per_tuple() {
        let a = pos(Product::Corrente, "231.30", "2026-07-25");
        let b = pos(Product::Corrente, "999.99", "2026-07-25"); // same tuple, other balance
        assert_eq!(a.id, b.id, "id depende da tupla, não do valor (upsert substitui)");
        assert_ne!(a.id, pos(Product::Poupanca, "231.30", "2026-07-25").id);
        assert_ne!(a.id, pos(Product::Corrente, "231.30", "2026-08-31").id);
        assert_eq!(cov("2026-07-01", "2026-07-25").id, cov("2026-07-01", "2026-07-25").id);
    }

    // T003 — current position: greatest as_of, import order irrelevant, products apart
    #[test]
    fn current_position_is_the_latest_base_date() {
        let all = vec![
            pos(Product::Corrente, "500.00", "2026-08-31"), // newest first (out of order)
            pos(Product::Corrente, "231.30", "2026-07-25"),
            pos(Product::Poupanca, "5000.00", "2026-07-25"),
        ];
        let current = current_positions(&all);
        assert_eq!(current.len(), 2, "um por produto");
        let corrente = current.iter().find(|p| p.product == Product::Corrente).unwrap();
        assert_eq!(corrente.balance, dec("500.00"), "vence o de maior as_of");
        let poupanca = current.iter().find(|p| p.product == Product::Poupanca).unwrap();
        assert_eq!(poupanca.balance, dec("5000.00"));
    }

    // T003 — month coverage: full / prefix partial / none / union without double count
    #[test]
    fn month_coverage_full_partial_none() {
        assert_eq!(
            month_coverage(&[cov("2026-07-01", "2026-07-31")], "2026-07"),
            MonthCoverage::Full
        );
        assert_eq!(
            month_coverage(&[cov("2026-07-01", "2026-07-25")], "2026-07"),
            MonthCoverage::Partial(vec![(d("2026-07-01"), d("2026-07-25"))])
        );
        assert_eq!(month_coverage(&[cov("2026-07-01", "2026-07-25")], "2026-06"), MonthCoverage::None);
    }

    #[test]
    fn overlapping_coverages_merge_into_full() {
        let covs = [cov("2026-07-01", "2026-07-25"), cov("2026-07-20", "2026-07-31")];
        assert_eq!(month_coverage(&covs, "2026-07"), MonthCoverage::Full);
    }

    #[test]
    fn cross_month_coverage_marks_both_months() {
        let covs = [cov("2026-07-20", "2026-08-10")];
        assert_eq!(
            month_coverage(&covs, "2026-07"),
            MonthCoverage::Partial(vec![(d("2026-07-20"), d("2026-07-31"))]),
            "julho coberto só no fim"
        );
        assert_eq!(
            month_coverage(&covs, "2026-08"),
            MonthCoverage::Partial(vec![(d("2026-08-01"), d("2026-08-10"))])
        );
    }

    // T003 — gaps: whole months with zero coverage; partials are not gaps
    #[test]
    fn gaps_are_wholly_uncovered_months_between_first_and_last() {
        let covs = [cov("2026-05-01", "2026-05-31"), cov("2026-07-01", "2026-07-25")];
        assert_eq!(coverage_gaps(&covs), vec!["2026-06".to_string()]);
        assert!(coverage_gaps(&[cov("2026-07-01", "2026-07-25")]).is_empty());
        assert!(coverage_gaps(&[]).is_empty());
    }

    // T003 — chain warning
    #[test]
    fn chain_warning_fires_only_on_divergence_with_a_previous_position() {
        let positions = vec![pos(Product::Corrente, "231.30", "2026-07-25")];

        // Matches → silence.
        assert!(chain_warning(&positions, d("2026-08-01"), dec("231.30")).is_none());
        // First import ever → nothing to compare.
        assert!(chain_warning(&[], d("2026-08-01"), dec("100.00")).is_none());
        // Position is AFTER the new start → not "previous", no warning.
        assert!(chain_warning(&positions, d("2026-07-01"), dec("100.00")).is_none());

        // Diverges → warning naming both values and the previous statement date.
        let w = chain_warning(&positions, d("2026-08-01"), dec("500.00")).unwrap();
        assert!(w.contains("500,00") && w.contains("231,30") && w.contains("25/07/2026"), "{w}");

        // Savings positions never anchor the chain (statement flows are Corrente).
        let only_savings = vec![pos(Product::Poupanca, "5000.00", "2026-07-25")];
        assert!(chain_warning(&only_savings, d("2026-08-01"), dec("1.00")).is_none());
    }

    #[test]
    fn chain_uses_the_closest_previous_position() {
        let positions = vec![
            pos(Product::Corrente, "100.00", "2026-05-31"),
            pos(Product::Corrente, "231.30", "2026-07-25"),
        ];
        // New statement starting 2026-08: compares against 25/07 (closest), not 31/05.
        assert!(chain_warning(&positions, d("2026-08-01"), dec("231.30")).is_none());
        let w = chain_warning(&positions, d("2026-08-01"), dec("100.00")).unwrap();
        assert!(w.contains("231,30"), "{w}");
    }
}
