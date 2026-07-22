//! Domain: the [`Transaction`] model — one card-invoice line item, with optional
//! [`InstallmentInfo`] (parcela x/N).

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentInfo {
    pub current: u8,
    pub total: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub category: String,
    pub installment: Option<InstallmentInfo>,
    pub is_reversal: bool,
}

impl Transaction {
    pub fn new(
        invoice_id: Uuid,
        row_index: u32,
        date: NaiveDate,
        description: String,
        amount: Decimal,
        category: String,
        installment: Option<InstallmentInfo>,
    ) -> Self {
        let is_reversal = amount < Decimal::ZERO;
        let id = Uuid::new_v5(
            &invoice_id,
            format!("{row_index}").as_bytes(),
        );
        Self {
            id,
            invoice_id,
            date,
            description,
            amount,
            category,
            installment,
            is_reversal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_negative_amount_marks_as_reversal() {
        let invoice_id = Uuid::new_v4();
        let tx = Transaction::new(
            invoice_id,
            0,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            "Estorno IFOOD".to_string(),
            dec!(-50.00),
            "Alimentação".to_string(),
            None,
        );
        assert!(tx.is_reversal);
        assert_eq!(tx.amount, dec!(-50.00));
    }

    #[test]
    fn test_positive_amount_not_reversal() {
        let invoice_id = Uuid::new_v4();
        let tx = Transaction::new(
            invoice_id,
            0,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            "IFOOD".to_string(),
            dec!(42.50),
            "Alimentação".to_string(),
            None,
        );
        assert!(!tx.is_reversal);
    }

    #[test]
    fn test_deterministic_id_same_inputs() {
        let invoice_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let tx1 = Transaction::new(invoice_id, 5, date, "DESC".to_string(), dec!(10.00), "Outros".to_string(), None);
        let tx2 = Transaction::new(invoice_id, 5, date, "DESC".to_string(), dec!(10.00), "Outros".to_string(), None);
        assert_eq!(tx1.id, tx2.id);
    }
}
