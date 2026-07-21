//! Card payment forecast from existing installments.
//!
//! Projects, month by month, how much of the card is already committed by
//! purchases the user has split into installments. Pure domain logic (no I/O),
//! deterministic (anchored to the most recent invoice month, not the clock).

use std::collections::{BTreeMap, HashMap};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::invoice::Invoice;

/// One installment that lands in a given future month (composition of a point).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastItem {
    pub description: String,
    pub parcela: String, // "3/5"
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

/// One projected future month.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub month: String, // ISO "YYYY-MM"
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub items: Vec<ForecastItem>,
}

/// A deduplicated installment purchase (most recent known parcela).
#[derive(Debug, Clone)]
struct Purchase {
    desc: String,
    current: u8,
    total: u8,
    amount: Decimal,
    ref_idx: i32, // month index of the `current` parcela
}

fn ym_index(year: i32, month: u8) -> i32 {
    year * 12 + (month as i32 - 1)
}
fn ym_from_index(idx: i32) -> (i32, u8) {
    (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u8)
}
fn ym_str(idx: i32) -> String {
    let (y, m) = ym_from_index(idx);
    format!("{y:04}-{m:02}")
}

/// True for tokens like "2/3" or "(2/3)" — installment markers to drop from keys/labels.
fn is_parcela_token(tok: &str) -> bool {
    let t = tok.trim_matches(|c| c == '(' || c == ')');
    match t.split_once('/') {
        Some((a, b)) => {
            !a.is_empty()
                && !b.is_empty()
                && a.chars().all(|c| c.is_ascii_digit())
                && b.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// Human description without the "(x/y)" marker (kept in original case).
fn strip_parcela(desc: &str) -> String {
    desc.split_whitespace()
        .filter(|t| !is_parcela_token(t))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Dedup key: the purchase identity, independent of which parcela was seen.
fn purchase_key(desc: &str, total: u8, amount: Decimal) -> String {
    format!("{}|{}|{}", strip_parcela(desc).to_uppercase(), total, amount)
}

/// Project future card payments from the installments in `invoices`.
///
/// The most recent invoice month is the anchor; the projection covers a
/// continuous series from anchor+1 up to the last pending parcela. Purchases
/// are deduplicated (keeping the most recent parcela) so the same purchase seen
/// across several invoices is not counted twice. Reversals are ignored.
pub fn compute_card_forecast(invoices: &[Invoice]) -> Vec<ForecastPoint> {
    let mut best: HashMap<String, Purchase> = HashMap::new();
    let mut anchor = i32::MIN;

    for inv in invoices {
        let ref_idx = ym_index(inv.reference_month.year, inv.reference_month.month);
        anchor = anchor.max(ref_idx);
        for t in &inv.transactions {
            if t.is_reversal {
                continue;
            }
            let Some(inst) = &t.installment else { continue };
            if inst.total == 0 || inst.current == 0 || inst.current > inst.total {
                continue;
            }
            let key = purchase_key(&t.description, inst.total, t.amount);
            let cand = Purchase {
                desc: strip_parcela(&t.description),
                current: inst.current,
                total: inst.total,
                amount: t.amount,
                ref_idx,
            };
            best.entry(key)
                .and_modify(|p| {
                    if cand.current > p.current || (cand.current == p.current && cand.ref_idx > p.ref_idx) {
                        *p = cand.clone();
                    }
                })
                .or_insert(cand);
        }
    }

    if anchor == i32::MIN {
        return Vec::new();
    }

    let mut sums: BTreeMap<i32, Decimal> = BTreeMap::new();
    let mut items: BTreeMap<i32, Vec<ForecastItem>> = BTreeMap::new();
    let mut last = anchor;

    for p in best.values() {
        let remaining = p.total.saturating_sub(p.current);
        for k in 1..=remaining {
            let midx = p.ref_idx + k as i32;
            if midx <= anchor {
                continue; // already billed at/before the latest invoice
            }
            last = last.max(midx);
            *sums.entry(midx).or_insert(dec!(0)) += p.amount;
            items.entry(midx).or_default().push(ForecastItem {
                description: p.desc.clone(),
                parcela: format!("{}/{}", p.current + k, p.total),
                amount: p.amount,
            });
        }
    }

    if sums.is_empty() {
        return Vec::new();
    }

    // Continuous series anchor+1 ..= last (zero-fill gaps so the timeline has no holes).
    let mut out = Vec::new();
    for idx in (anchor + 1)..=last {
        let amount = sums.get(&idx).copied().unwrap_or(dec!(0));
        let mut its = items.remove(&idx).unwrap_or_default();
        its.sort_by_key(|i| std::cmp::Reverse(i.amount));
        out.push(ForecastPoint {
            month: ym_str(idx),
            amount,
            items: its,
        });
    }
    out
}

/// Total still to be paid across all future parcelas (sum of the series).
pub fn forecast_committed_total(points: &[ForecastPoint]) -> Decimal {
    points.iter().fold(dec!(0), |a, p| a + p.amount)
}

/// The month the commitment ends (last point), or "" when there is none.
pub fn forecast_last_month(points: &[ForecastPoint]) -> String {
    points.last().map(|p| p.month.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::invoice::{Invoice, YearMonth};
    use crate::domain::transaction::{InstallmentInfo, Transaction};
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn dt() -> chrono::NaiveDateTime {
        chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc()
    }
    fn tx(desc: &str, amount: Decimal, inst: Option<(u8, u8)>, reversal: bool) -> Transaction {
        let mut t = Transaction::new(
            Uuid::new_v4(),
            0,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            desc.into(),
            amount,
            "Compras".into(),
            inst.map(|(current, total)| InstallmentInfo { current, total }),
        );
        t.is_reversal = reversal;
        t
    }
    fn inv(year: i32, month: u8, txs: Vec<Transaction>) -> Invoice {
        Invoice::new("f.xlsx".into(), YearMonth::new(year, month), None, txs, dt())
    }

    #[test]
    fn single_purchase_spreads_over_future_months() {
        let invs = vec![inv(2026, 6, vec![tx("Loja (1/3)", dec!(100), Some((1, 3)), false)])];
        let f = compute_card_forecast(&invs);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].month, "2026-07");
        assert_eq!(f[0].amount, dec!(100));
        assert_eq!(f[0].items[0].parcela, "2/3");
        assert_eq!(f[1].month, "2026-08");
        assert_eq!(f[1].items[0].parcela, "3/3");
    }

    #[test]
    fn two_purchases_same_month_sum() {
        let invs = vec![inv(
            2026,
            6,
            vec![
                tx("A (1/2)", dec!(100), Some((1, 2)), false),
                tx("B (1/2)", dec!(50), Some((1, 2)), false),
            ],
        )];
        let f = compute_card_forecast(&invs);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].month, "2026-07");
        assert_eq!(f[0].amount, dec!(150));
        assert_eq!(f[0].items.len(), 2);
        // sorted desc by amount
        assert_eq!(f[0].items[0].amount, dec!(100));
    }

    #[test]
    fn dedup_same_purchase_across_invoices() {
        // Same purchase seen as 1/3 (May) and 2/3 (June). Only 3/3 remains.
        let invs = vec![
            inv(2026, 5, vec![tx("Loja X (1/3)", dec!(100), Some((1, 3)), false)]),
            inv(2026, 6, vec![tx("Loja X (2/3)", dec!(100), Some((2, 3)), false)]),
        ];
        let f = compute_card_forecast(&invs);
        assert_eq!(f.len(), 1, "só resta 3/3");
        assert_eq!(f[0].month, "2026-07");
        assert_eq!(f[0].amount, dec!(100), "não conta em dobro");
        assert_eq!(f[0].items[0].parcela, "3/3");
    }

    #[test]
    fn last_installment_has_no_future() {
        let invs = vec![inv(2026, 6, vec![tx("Loja (3/3)", dec!(100), Some((3, 3)), false)])];
        assert!(compute_card_forecast(&invs).is_empty());
    }

    #[test]
    fn no_installments_is_empty() {
        let invs = vec![inv(2026, 6, vec![tx("À vista", dec!(100), None, false)])];
        assert!(compute_card_forecast(&invs).is_empty());
    }

    #[test]
    fn reversal_is_ignored() {
        let invs = vec![inv(2026, 6, vec![tx("Estorno (1/3)", dec!(100), Some((1, 3)), true)])];
        assert!(compute_card_forecast(&invs).is_empty());
    }

    #[test]
    fn invariant_sum_matches_remaining_no_duplicates() {
        // One fatura, distinct purchases → forecast sum == Σ remaining×amount.
        let invs = vec![inv(
            2026,
            6,
            vec![
                tx("A (1/3)", dec!(100), Some((1, 3)), false), // remaining 2 → 200
                tx("B (2/4)", dec!(50), Some((2, 4)), false),  // remaining 2 → 100
            ],
        )];
        let f = compute_card_forecast(&invs);
        assert_eq!(forecast_committed_total(&f), dec!(300));
        // A: 2/3 jul, 3/3 ago · B: 3/4 jul, 4/4 ago → last = ago/2026
        assert_eq!(forecast_last_month(&f), "2026-08");
    }

    #[test]
    fn series_is_contiguous() {
        let invs = vec![inv(2026, 6, vec![tx("Longo (1/4)", dec!(10), Some((1, 4)), false)])];
        let f = compute_card_forecast(&invs);
        let months: Vec<&str> = f.iter().map(|p| p.month.as_str()).collect();
        assert_eq!(months, vec!["2026-07", "2026-08", "2026-09"]);
    }
}
