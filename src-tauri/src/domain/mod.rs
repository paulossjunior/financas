pub mod category;
pub mod categorizer;
pub mod dashboard;
pub mod invoice;
pub mod transaction;

pub use category::{aggregate_by_category, Category, TransactionSummary};
pub use categorizer::{CategoryRule, Categorizer};
pub use dashboard::{compute_dashboard, DashboardData, DashboardFilter};
pub use invoice::{Invoice, YearMonth};
pub use transaction::{InstallmentInfo, Transaction};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub faturas_directory: String,
    pub category_rules: Vec<CategoryRule>,
    #[serde(default)]
    pub transaction_overrides: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            faturas_directory: "faturas".to_string(),
            category_rules: vec![],
            transaction_overrides: HashMap::new(),
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
    }

    #[test]
    fn appconfig_deserializes_with_overrides_field() {
        let json = r#"{"faturas_directory":"faturas","category_rules":[],"transaction_overrides":{"abc":"Educação"}}"#;
        let config: AppConfig = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(config.transaction_overrides.get("abc").map(String::as_str), Some("Educação"));
    }
}
