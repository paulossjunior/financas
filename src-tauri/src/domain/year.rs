use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::category::{aggregate_by_category, Category};
use super::invoice::Invoice;
use super::manual_entry::{EntryKind, ManualEntry};
use super::transaction::Transaction;

/// One month of the year view. Amounts are decimal strings (exact money on the JS side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearMonthPoint {
    pub month: String, // ISO "YYYY-MM"
    pub income: String,
    pub card: String,
    pub fixed: String,
    pub expense: String, // card + fixed
    pub balance: String, // income − expense
}

/// Whole-period ("year") view: everything the annual dashboard needs in one shot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearSummary {
    pub months: Vec<YearMonthPoint>,
    pub income_total: String,
    pub expense_total: String,
    pub card_total: String,
    pub fixed_total: String,
    pub balance_total: String,
    pub avg_expense: String,
    pub biggest_month: String,
    pub biggest_month_value: String,
    /// balance_total / income_total (0.0 when there is no income).
    pub savings_rate: f64,
    pub active_months: u32,
    pub tx_count: u32,
    pub categories: Vec<Category>,
    /// All calendar years present in the data (desc), for the year filter — never affected by `year`.
    pub available_years: Vec<i32>,
}

/// Build the annual view. Unlike the monthly dashboard (grouped by invoice reference
/// month), card spending here is grouped by **transaction date** — the month the money
/// was actually spent — so installments spread across the year the way they were charged.
/// Recurring manual entries count once for every month in scope; one-offs only for their own.
pub fn compute_year_summary(
    invoices: &[Invoice],
    manual: &[ManualEntry],
    year: Option<i32>,
) -> YearSummary {
    // Every calendar year in the data — computed BEFORE filtering so the dropdown is stable.
    let mut years: BTreeSet<i32> = BTreeSet::new();
    for inv in invoices {
        for t in &inv.transactions {
            years.insert(t.date.year());
        }
    }
    for e in manual {
        if let Some(y) = year_of(&e.month) {
            years.insert(y);
        }
    }
    let available_years: Vec<i32> = years.into_iter().rev().collect();

    let in_year = |ym: &str| match year {
        Some(y) => year_of(ym) == Some(y),
        None => true,
    };

    // Card grouped by transaction-date month, net of reversals (filtered to `year`).
    let mut card_by: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut card_txs: Vec<Transaction> = Vec::new();
    for inv in invoices {
        for t in &inv.transactions {
            let m = t.date.format("%Y-%m").to_string();
            if !in_year(&m) {
                continue;
            }
            let signed = if t.is_reversal { -t.amount } else { t.amount };
            *card_by.entry(m).or_insert(dec!(0)) += signed;
            card_txs.push(t.clone());
        }
    }

    // Scope = every month with card activity, plus each in-year manual entry's own month.
    let mut scope: BTreeSet<String> = card_by.keys().cloned().collect();
    for e in manual {
        if in_year(&e.month) {
            scope.insert(e.month.clone());
        }
    }

    // Expand manual entries over the scope.
    let mut income_by: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut fixed_by: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut manual_expense_txs: Vec<Transaction> = Vec::new();
    for e in manual {
        let months: Vec<String> = if e.recurring {
            scope.iter().cloned().collect()
        } else if scope.contains(&e.month) {
            vec![e.month.clone()]
        } else {
            vec![]
        };
        for m in months {
            match e.kind {
                EntryKind::Income => *income_by.entry(m).or_insert(dec!(0)) += e.amount,
                EntryKind::Expense => {
                    *fixed_by.entry(m.clone()).or_insert(dec!(0)) += e.amount;
                    manual_expense_txs.push(e.to_transaction(&m));
                }
            }
        }
    }

    // Per-month points, chronological (BTreeSet iterates sorted).
    let mut months: Vec<YearMonthPoint> = Vec::new();
    let mut card_total = dec!(0);
    let mut fixed_total = dec!(0);
    let mut income_total = dec!(0);
    let mut biggest = (String::new(), dec!(0));
    for m in &scope {
        let card = card_by.get(m).copied().unwrap_or(dec!(0));
        let fixed = fixed_by.get(m).copied().unwrap_or(dec!(0));
        let income = income_by.get(m).copied().unwrap_or(dec!(0));
        let expense = card + fixed;
        card_total += card;
        fixed_total += fixed;
        income_total += income;
        if expense > biggest.1 {
            biggest = (m.clone(), expense);
        }
        months.push(YearMonthPoint {
            month: m.clone(),
            income: income.to_string(),
            card: card.to_string(),
            fixed: fixed.to_string(),
            expense: expense.to_string(),
            balance: (income - expense).to_string(),
        });
    }

    let expense_total = card_total + fixed_total;
    let balance_total = income_total - expense_total;
    let active = scope.len().max(1) as i64;
    let avg_expense = (expense_total / Decimal::from(active)).round_dp(2);
    let savings_rate = if income_total > dec!(0) {
        (balance_total / income_total).to_f64().unwrap_or(0.0)
    } else {
        0.0
    };

    // Category ranking across the whole period (card charges + synthetic manual expenses).
    let mut cat_txs = card_txs.clone();
    cat_txs.extend(manual_expense_txs);
    let categories = aggregate_by_category(&cat_txs, expense_total);

    YearSummary {
        months,
        income_total: income_total.to_string(),
        expense_total: expense_total.to_string(),
        card_total: card_total.to_string(),
        fixed_total: fixed_total.to_string(),
        balance_total: balance_total.to_string(),
        avg_expense: avg_expense.to_string(),
        biggest_month: biggest.0,
        biggest_month_value: biggest.1.to_string(),
        savings_rate,
        active_months: scope.len() as u32,
        tx_count: card_txs.len() as u32,
        categories,
        available_years,
    }
}

/// Parse the year out of an ISO "YYYY-MM" string.
fn year_of(ym: &str) -> Option<i32> {
    ym.get(0..4).and_then(|s| s.parse::<i32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::invoice::YearMonth;
    use chrono::{NaiveDate, NaiveDateTime};
    use uuid::Uuid;

    fn invoice_with(dates_amounts: &[(&str, Decimal)]) -> Invoice {
        let id = Uuid::new_v4();
        let txs: Vec<Transaction> = dates_amounts
            .iter()
            .enumerate()
            .map(|(i, (d, amt))| {
                Transaction::new(
                    id,
                    i as u32,
                    NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap(),
                    format!("TX {i}"),
                    *amt,
                    "Outros".to_string(),
                    None,
                )
            })
            .collect();
        Invoice::new(
            "f.xlsx".into(),
            YearMonth::new(2026, 6),
            None,
            txs,
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        )
    }

    fn entry(kind: EntryKind, amount: Decimal, cat: &str, month: &str, recurring: bool) -> ManualEntry {
        ManualEntry::new(kind, "m".into(), amount, cat.into(), month.into(), recurring)
    }

    #[test]
    fn groups_card_by_transaction_date() {
        // Two charges in different months, even from one invoice.
        let inv = invoice_with(&[("2026-05-10", dec!(100)), ("2026-06-02", dec!(200))]);
        let y = compute_year_summary(&[inv], &[], None);
        assert_eq!(y.months.len(), 2);
        assert_eq!(y.months[0].month, "2026-05");
        assert_eq!(y.months[0].card, "100");
        assert_eq!(y.months[1].month, "2026-06");
        assert_eq!(y.card_total, "300");
    }

    #[test]
    fn recurring_fixed_and_income_span_all_scope_months() {
        let inv = invoice_with(&[("2026-05-10", dec!(1000)), ("2026-06-10", dec!(2000))]);
        let manual = vec![
            entry(EntryKind::Expense, dec!(2950), "Moradia & Serviços", "2026-06", true),
            entry(EntryKind::Income, dec!(8000), "Salário", "2026-06", true),
        ];
        let y = compute_year_summary(&[inv], &manual, None);
        // 2 scope months → recurring counts twice.
        assert_eq!(y.fixed_total, "5900"); // 2950 * 2
        assert_eq!(y.income_total, "16000"); // 8000 * 2
        assert_eq!(y.expense_total, "8900"); // card 3000 + fixed 5900
        assert_eq!(y.balance_total, "7100"); // 16000 - 8900
        assert_eq!(y.active_months, 2);
        // May: card 1000 + fixed 2950 = 3950; June: 2000 + 2950 = 4950 → biggest June.
        assert_eq!(y.biggest_month, "2026-06");
        assert_eq!(y.biggest_month_value, "4950");
        // savings 7100/16000 ≈ 0.44
        assert!((y.savings_rate - 0.44375).abs() < 1e-6);
    }

    #[test]
    fn income_appears_per_month_not_as_category() {
        let inv = invoice_with(&[("2026-06-10", dec!(500))]);
        let manual = vec![entry(EntryKind::Income, dec!(9000), "Salário", "2026-06", true)];
        let y = compute_year_summary(&[inv], &manual, None);
        assert_eq!(y.months[0].income, "9000");
        assert!(y.categories.iter().all(|c| c.name != "Salário"));
    }

    #[test]
    fn no_income_gives_zero_savings_rate() {
        let inv = invoice_with(&[("2026-06-10", dec!(500))]);
        let y = compute_year_summary(&[inv], &[], None);
        assert_eq!(y.savings_rate, 0.0);
    }

    #[test]
    fn year_filter_restricts_to_that_year_and_lists_all_years() {
        let inv = invoice_with(&[("2025-12-10", dec!(200)), ("2026-03-10", dec!(500))]);
        let all = compute_year_summary(&[inv.clone()], &[], None);
        assert_eq!(all.available_years, vec![2026, 2025]); // desc, unaffected by filter
        assert_eq!(all.months.len(), 2);

        let only26 = compute_year_summary(&[inv], &[], Some(2026));
        assert_eq!(only26.months.len(), 1);
        assert_eq!(only26.months[0].month, "2026-03");
        assert_eq!(only26.card_total, "500");
        assert_eq!(only26.available_years, vec![2026, 2025]); // still lists both
    }
}
