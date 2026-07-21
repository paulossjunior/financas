use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::transaction::Transaction;

/// Whether a manual entry adds to income (crédito) or expense (débito).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Income,
    Expense,
}

/// A cash movement that does NOT come from a credit-card invoice:
/// fixed bills (aluguel, energia, água…) and income (salário, bolsa, rendimentos).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualEntry {
    pub id: Uuid,
    pub kind: EntryKind,
    pub description: String,
    /// Always stored positive; `kind` carries the sign meaning.
    /// Serialized as a string to preserve exact money values on the JS side.
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub category: String,
    /// Reference month, ISO "YYYY-MM".
    pub month: String,
    /// When true the entry counts once for every month in the dashboard scope
    /// (a fixed monthly cost / recurring income). When false it counts only for `month`.
    pub recurring: bool,
}

impl ManualEntry {
    pub fn new(
        kind: EntryKind,
        description: String,
        amount: Decimal,
        category: String,
        month: String,
        recurring: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            description,
            amount: amount.abs(),
            category,
            month,
            recurring,
        }
    }

    /// Build a synthetic transaction for aggregation, dated the first day of `month`.
    /// Expense → positive amount (like a charge); Income → not represented here
    /// (income never enters the expense category aggregation).
    pub fn to_transaction(&self, month: &str) -> Transaction {
        let date = parse_month_start(month).unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()
        });
        // Deterministic-enough synthetic invoice id namespaced by the entry id.
        let synth_invoice = Uuid::new_v5(&Uuid::NAMESPACE_OID, self.id.as_bytes());
        Transaction::new(
            synth_invoice,
            0,
            date,
            self.description.clone(),
            self.amount,
            self.category.clone(),
            None,
        )
    }
}

/// Parse "YYYY-MM" into the first day of that month.
pub fn parse_month_start(month: &str) -> Option<NaiveDate> {
    let mut parts = month.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    NaiveDate::from_ymd_opt(y, m, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn amount_stored_positive() {
        let e = ManualEntry::new(
            EntryKind::Expense,
            "Aluguel".into(),
            dec!(-2950.00),
            "Moradia & Serviços".into(),
            "2026-06".into(),
            true,
        );
        assert_eq!(e.amount, dec!(2950.00));
    }

    #[test]
    fn kind_serializes_lowercase() {
        let json = serde_json::to_string(&EntryKind::Income).unwrap();
        assert_eq!(json, "\"income\"");
    }

    #[test]
    fn to_transaction_dates_first_of_month() {
        let e = ManualEntry::new(
            EntryKind::Expense,
            "Energia".into(),
            dec!(1000),
            "Moradia & Serviços".into(),
            "2026-06".into(),
            true,
        );
        let tx = e.to_transaction("2026-06");
        assert_eq!(tx.date, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        assert_eq!(tx.category, "Moradia & Serviços");
        assert!(!tx.is_reversal);
    }

    #[test]
    fn parse_month_start_ok() {
        assert_eq!(parse_month_start("2026-06"), NaiveDate::from_ymd_opt(2026, 6, 1));
        assert_eq!(parse_month_start("bad"), None);
    }
}
