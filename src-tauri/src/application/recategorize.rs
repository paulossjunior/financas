//! Application use-case: re-run categorization over every stored transaction using
//! the current keyword rules and per-transaction overrides.

use crate::domain::AppConfig;
use super::store::SharedStore;

pub fn recategorize_invoices(store: &SharedStore, config: &AppConfig) -> usize {
    use crate::domain::categorizer::Categorizer;

    let categorizer = if config.category_rules.is_empty() {
        Categorizer::with_defaults()
    } else {
        Categorizer::new(config.category_rules.clone())
    };

    let overrides = &config.transaction_overrides;
    let mut store = store.lock().unwrap();

    store.for_each_transaction_mut(|tx| {
        let new_cat = overrides
            .get(&tx.id.to_string())
            .cloned()
            .unwrap_or_else(|| categorizer.categorize(&tx.description));
        if tx.category != new_cat {
            tx.category = new_cat;
            true
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::application::store::new_shared_store;
    use crate::domain::{
        AppConfig, CategoryRule,
        invoice::{Invoice, YearMonth},
        transaction::Transaction,
    };
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn make_transaction(invoice_id: uuid::Uuid, row: u32, description: &str, category: &str) -> Transaction {
        Transaction::new(
            invoice_id,
            row,
            NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            description.to_string(),
            dec!(100.00),
            category.to_string(),
            None,
        )
    }

    fn make_invoice_with_txs(filename: &str, txs: Vec<Transaction>) -> Invoice {
        Invoice::new(
            filename.to_string(),
            YearMonth::new(2026, 5),
            None,
            txs,
            chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        )
    }

    #[test]
    fn recategorize_applies_rules_and_returns_change_count() {
        let store = new_shared_store();
        let invoice_id = uuid::Uuid::NAMESPACE_URL; // placeholder for test

        let tx1 = make_transaction(invoice_id, 0, "IFOOD DELIVERY", "Outros");
        let tx2 = make_transaction(invoice_id, 1, "NETFLIX BRASIL", "Outros");
        let invoice = make_invoice_with_txs("2026-05-test.xlsx", vec![tx1, tx2]);
        store.lock().unwrap().add(invoice);

        let config = AppConfig {
            faturas_directory: "faturas".into(),
            category_rules: vec![
                CategoryRule { keywords: vec!["IFOOD".into()], category: "Alimentação".into(), priority: 10 },
                CategoryRule { keywords: vec!["NETFLIX".into()], category: "Lazer".into(), priority: 20 },
            ],
            transaction_overrides: HashMap::new(),
            manual_entries: vec![],
        };

        let changed = recategorize_invoices(&store, &config);
        // stub returns 0, real impl must return 2
        assert_eq!(changed, 2, "both transactions should be recategorized");
    }

    #[test]
    fn recategorize_applies_override_over_rule() {
        let store = new_shared_store();
        let filename = "2026-05-override-test.xlsx";
        let invoice_id = uuid::Uuid::NAMESPACE_URL;

        let tx = make_transaction(invoice_id, 0, "AMAZON PRIME", "Outros");
        let tx_id = tx.id.to_string();
        let invoice = make_invoice_with_txs(filename, vec![tx]);
        store.lock().unwrap().add(invoice);

        let mut overrides = HashMap::new();
        overrides.insert(tx_id.clone(), "Educação".to_string());

        let config = AppConfig {
            faturas_directory: "faturas".into(),
            category_rules: vec![
                CategoryRule { keywords: vec!["AMAZON".into()], category: "Compras".into(), priority: 10 },
            ],
            transaction_overrides: overrides,
            manual_entries: vec![],
        };

        let changed = recategorize_invoices(&store, &config);
        // override must win over rule
        assert_eq!(changed, 1);

        let store_locked = store.lock().unwrap();
        let invoices = store_locked.list();
        let final_category = &invoices[0].transactions[0].category;
        assert_eq!(final_category, "Educação", "override must take priority over rule");
    }
}
