use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::transaction::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
    pub fn new(year: i32, month: u8) -> Self {
        Self { year, month }
    }

    pub fn from_date(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month() as u8,
        }
    }

    pub fn to_string_iso(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }
}

use chrono::Datelike;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub filename: String,
    pub reference_month: YearMonth,
    pub due_date: Option<NaiveDate>,
    pub transactions: Vec<Transaction>,
    pub imported_at: NaiveDateTime,
}

impl Invoice {
    pub fn new(
        filename: String,
        reference_month: YearMonth,
        due_date: Option<NaiveDate>,
        transactions: Vec<Transaction>,
        imported_at: NaiveDateTime,
    ) -> Self {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, filename.as_bytes());
        Self {
            id,
            filename,
            reference_month,
            due_date,
            transactions,
            imported_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn make_invoice(filename: &str) -> Invoice {
        Invoice::new(
            filename.to_string(),
            YearMonth::new(2026, 5),
            None,
            vec![],
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        )
    }

    #[test]
    fn invoice_id_deterministic_from_filename() {
        let a = make_invoice("2026-05-fatura.xlsx");
        let b = make_invoice("2026-05-fatura.xlsx");
        // This test MUST FAIL until we switch to Uuid::new_v5
        assert_eq!(a.id, b.id, "same filename must produce same Invoice ID");
    }

    #[test]
    fn invoice_id_differs_for_different_filenames() {
        let a = make_invoice("2026-05-fatura-btg.xlsx");
        let b = make_invoice("2026-05-fatura-dep.xlsx");
        assert_ne!(a.id, b.id, "different filenames must produce different IDs");
    }
}
