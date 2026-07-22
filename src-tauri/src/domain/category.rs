//! Domain: category model and per-category aggregation of transactions
//! ([`Category`], [`TransactionSummary`], [`aggregate_by_category`]).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::transaction::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    pub id: String,
    pub date: String,
    pub description: String,
    pub amount: String,
    pub category: String,
}

impl TransactionSummary {
    pub fn from_transaction(tx: &Transaction) -> Self {
        Self {
            id: tx.id.to_string(),
            date: tx.date.format("%Y-%m-%d").to_string(),
            description: tx.description.clone(),
            amount: tx.amount.to_string(),
            category: tx.category.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub total: String,
    pub reversal_total: String,
    pub net_total: String,
    pub percentage: f64,
    pub transaction_count: u32,
    pub top_transactions: Vec<TransactionSummary>,
}

pub fn aggregate_by_category(transactions: &[Transaction], grand_total: Decimal) -> Vec<Category> {
    let mut map: std::collections::HashMap<String, (Decimal, Decimal, Vec<&Transaction>)> =
        std::collections::HashMap::new();

    for tx in transactions {
        let entry = map.entry(tx.category.clone()).or_insert((dec!(0), dec!(0), vec![]));
        if tx.is_reversal {
            entry.1 += tx.amount;
        } else {
            entry.0 += tx.amount;
        }
        entry.2.push(tx);
    }

    let mut categories: Vec<Category> = map
        .into_iter()
        .map(|(name, (total, reversal_total, txs))| {
            let net_total = total + reversal_total;
            let percentage = if grand_total > dec!(0) {
                ((net_total / grand_total) * dec!(100))
                    .to_f64()
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            let mut top: Vec<&Transaction> = txs.iter().filter(|t| !t.is_reversal).cloned().collect();
            top.sort_by_key(|b| std::cmp::Reverse(b.amount));
            let top_transactions = top
                .iter()
                .take(3)
                .map(|t| TransactionSummary::from_transaction(t))
                .collect();

            Category {
                name,
                total: total.to_string(),
                reversal_total: reversal_total.to_string(),
                net_total: net_total.to_string(),
                percentage,
                transaction_count: txs.len() as u32,
                top_transactions,
            }
        })
        .collect();

    categories.sort_by(|a, b| {
        let na: Decimal = a.net_total.parse().unwrap_or(dec!(0));
        let nb: Decimal = b.net_total.parse().unwrap_or(dec!(0));
        nb.cmp(&na)
    });

    categories
}

use rust_decimal::prelude::ToPrimitive;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::transaction::Transaction;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn make_tx(invoice_id: Uuid, idx: u32, amount: Decimal, category: &str) -> Transaction {
        Transaction::new(
            invoice_id,
            idx,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            format!("Desc {idx}"),
            amount,
            category.to_string(),
            None,
        )
    }

    #[test]
    fn test_aggregate_two_categories() {
        let id = Uuid::new_v4();
        let txs = vec![
            make_tx(id, 0, dec!(100.00), "Alimentação"),
            make_tx(id, 1, dec!(50.00), "Alimentação"),
            make_tx(id, 2, dec!(200.00), "Transporte"),
        ];
        let grand = dec!(350.00);
        let cats = aggregate_by_category(&txs, grand);
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].name, "Transporte");
        let net: Decimal = cats[0].net_total.parse().unwrap();
        assert_eq!(net, dec!(200.00));
    }

    #[test]
    fn test_reversals_reduce_net_total() {
        let id = Uuid::new_v4();
        let txs = vec![
            make_tx(id, 0, dec!(100.00), "Alimentação"),
            make_tx(id, 1, dec!(-30.00), "Alimentação"),
        ];
        let grand = dec!(70.00);
        let cats = aggregate_by_category(&txs, grand);
        assert_eq!(cats.len(), 1);
        let net: Decimal = cats[0].net_total.parse().unwrap();
        assert_eq!(net, dec!(70.00));
    }

    #[test]
    fn test_percentage_sums_to_100() {
        let id = Uuid::new_v4();
        let txs = vec![
            make_tx(id, 0, dec!(75.00), "A"),
            make_tx(id, 1, dec!(25.00), "B"),
        ];
        let grand = dec!(100.00);
        let cats = aggregate_by_category(&txs, grand);
        let sum: f64 = cats.iter().map(|c| c.percentage).sum();
        assert!((sum - 100.0).abs() < 0.01);
    }
}
