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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub faturas_directory: String,
    pub category_rules: Vec<CategoryRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            faturas_directory: "faturas".to_string(),
            category_rules: vec![],
        }
    }
}
