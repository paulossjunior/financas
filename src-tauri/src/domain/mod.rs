pub mod bank_statement;
pub mod category;
pub mod categorizer;
pub mod dashboard;
pub mod forecast;
pub mod inflation;
pub mod invoice;
pub mod manual_entry;
pub mod payslip;
pub mod transaction;
pub mod year;

pub use bank_statement::{classify_entry, entry_id, holder_key, parse_statement_rows, ClassifiedEntry, ParsedStatement, RawEntry};
pub use category::{aggregate_by_category, Category, TransactionSummary};
pub use categorizer::{CategoryRule, Categorizer};
pub use dashboard::{compute_dashboard, DashboardData, DashboardFilter};
pub use forecast::{compute_card_forecast, forecast_committed_total, forecast_last_month, ForecastItem, ForecastPoint};
pub use inflation::{compute_personal_inflation, InflationCache, InflationData, IpcaGroup, IpcaHeadline, IpcaPoint};
pub use invoice::{Invoice, YearMonth};
pub use manual_entry::{EntryKind, ManualEntry};
pub use payslip::{parse_payslip_text, Payslip, PayslipItem};
pub use transaction::{InstallmentInfo, Transaction};
pub use year::{compute_year_summary, YearSummary};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub faturas_directory: String,
    pub category_rules: Vec<CategoryRule>,
    #[serde(default)]
    pub transaction_overrides: HashMap<String, String>,
    /// Cash movements outside the credit card: fixed bills and income.
    #[serde(default)]
    pub manual_entries: Vec<ManualEntry>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            faturas_directory: "faturas".to_string(),
            category_rules: vec![],
            transaction_overrides: HashMap::new(),
            manual_entries: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appconfig_deserializes_without_overrides_field() {
        let json = r#"{"faturas_directory":"faturas","category_rules":[]}"#;
        let config: AppConfig = serde_json::from_str(json).expect("should deserialize");
        assert!(config.transaction_overrides.is_empty());
        assert!(config.manual_entries.is_empty());
    }

    #[test]
    fn appconfig_deserializes_with_overrides_field() {
        let json = r#"{"faturas_directory":"faturas","category_rules":[],"transaction_overrides":{"abc":"Educação"}}"#;
        let config: AppConfig = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(config.transaction_overrides.get("abc").map(String::as_str), Some("Educação"));
    }
}
