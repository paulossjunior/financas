use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::{
    category::{aggregate_by_category, Category, TransactionSummary},
    invoice::{Invoice, YearMonth},
    transaction::Transaction,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardFilter {
    pub invoice_ids: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySnapshot {
    pub name: String,
    pub net_total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySnapshot {
    pub month: String,
    pub net_total: String,
    pub categories: Vec<CategorySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPeriod {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub period: DashboardPeriod,
    pub total_charged: String,
    pub total_reversals: String,
    pub net_total: String,
    pub invoice_count: u32,
    pub categories: Vec<Category>,
    pub top_transactions: Vec<TransactionSummary>,
    pub monthly_trend: Vec<MonthlySnapshot>,
}

pub fn compute_dashboard(invoices: &[Invoice], _filter: &DashboardFilter) -> DashboardData {
    let all_transactions: Vec<&Transaction> = invoices
        .iter()
        .flat_map(|inv| inv.transactions.iter())
        .collect();

    let total_charged: Decimal = all_transactions
        .iter()
        .filter(|t| !t.is_reversal)
        .map(|t| t.amount)
        .fold(dec!(0), |acc, a| acc + a);

    let total_reversals: Decimal = all_transactions
        .iter()
        .filter(|t| t.is_reversal)
        .map(|t| t.amount)
        .fold(dec!(0), |acc, a| acc + a);

    let net_total = total_charged + total_reversals;

    let txs_owned: Vec<Transaction> = all_transactions.iter().map(|t| (*t).clone()).collect();
    let categories = aggregate_by_category(&txs_owned, net_total);

    let top_transactions = top_5_transactions(&txs_owned);

    let monthly_trend = if invoices.len() >= 2 {
        compute_monthly_trend(invoices)
    } else {
        vec![]
    };

    let months: Vec<&YearMonth> = invoices.iter().map(|i| &i.reference_month).collect();
    let from = months.iter().min().map(|m| m.to_string_iso()).unwrap_or_default();
    let to = months.iter().max().map(|m| m.to_string_iso()).unwrap_or_default();

    DashboardData {
        period: DashboardPeriod { from, to },
        total_charged: total_charged.to_string(),
        total_reversals: total_reversals.to_string(),
        net_total: net_total.to_string(),
        invoice_count: invoices.len() as u32,
        categories,
        top_transactions,
        monthly_trend,
    }
}

fn top_5_transactions(transactions: &[Transaction]) -> Vec<TransactionSummary> {
    let mut charges: Vec<&Transaction> = transactions.iter().filter(|t| !t.is_reversal).collect();
    charges.sort_by_key(|b| std::cmp::Reverse(b.amount));
    charges
        .iter()
        .take(5)
        .map(|t| TransactionSummary::from_transaction(t))
        .collect()
}

fn compute_monthly_trend(invoices: &[Invoice]) -> Vec<MonthlySnapshot> {
    let mut monthly: Vec<(&YearMonth, &[Transaction])> = invoices
        .iter()
        .map(|inv| (&inv.reference_month, inv.transactions.as_slice()))
        .collect();
    monthly.sort_by_key(|(ym, _)| (*ym).clone());

    monthly
        .into_iter()
        .map(|(ym, txs)| {
            let net: Decimal = txs.iter().map(|t| t.amount).fold(dec!(0), |a, b| a + b);
            let cats = aggregate_by_category(txs, net);
            let cat_snapshots: Vec<CategorySnapshot> = cats
                .into_iter()
                .map(|c| CategorySnapshot {
                    name: c.name,
                    net_total: c.net_total,
                })
                .collect();
            MonthlySnapshot {
                month: ym.to_string_iso(),
                net_total: net.to_string(),
                categories: cat_snapshots,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{invoice::Invoice, transaction::Transaction};
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn make_invoice(year: i32, month: u8, amounts: &[Decimal]) -> Invoice {
        let id = Uuid::new_v4();
        let txs: Vec<Transaction> = amounts
            .iter()
            .enumerate()
            .map(|(i, &amt)| {
                Transaction::new(
                    id,
                    i as u32,
                    NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap(),
                    format!("TX {i}"),
                    amt,
                    "Outros".to_string(),
                    None,
                )
            })
            .collect();
        Invoice::new(
            format!("{year}-{month:02}-fatura.xlsx"),
            YearMonth::new(year, month),
            None,
            txs,
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        )
    }

    #[test]
    fn test_top_transactions_returns_5_largest() {
        let amounts: Vec<Decimal> = vec![
            dec!(10), dec!(50), dec!(30), dec!(100), dec!(20), dec!(70), dec!(5),
        ];
        let txs: Vec<Transaction> = amounts
            .iter()
            .enumerate()
            .map(|(i, &amt)| {
                let inv_id = Uuid::new_v4();
                Transaction::new(inv_id, i as u32, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), "D".to_string(), amt, "Outros".to_string(), None)
            })
            .collect();
        let top = top_5_transactions(&txs);
        assert_eq!(top.len(), 5);
        let first: Decimal = top[0].amount.parse().unwrap();
        assert_eq!(first, dec!(100));
    }

    #[test]
    fn test_monthly_trend_empty_when_single_invoice() {
        let inv = make_invoice(2026, 6, &[dec!(100)]);
        let filter = DashboardFilter::default();
        let data = compute_dashboard(&[inv], &filter);
        assert!(data.monthly_trend.is_empty());
    }

    #[test]
    fn test_monthly_trend_two_months() {
        let inv1 = make_invoice(2026, 5, &[dec!(100)]);
        let inv2 = make_invoice(2026, 6, &[dec!(200)]);
        let filter = DashboardFilter::default();
        let data = compute_dashboard(&[inv1, inv2], &filter);
        assert_eq!(data.monthly_trend.len(), 2);
        assert_eq!(data.monthly_trend[0].month, "2026-05");
        assert_eq!(data.monthly_trend[1].month, "2026-06");
    }
}
