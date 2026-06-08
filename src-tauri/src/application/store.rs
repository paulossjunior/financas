use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::domain::Invoice;

#[derive(Debug, Default)]
pub struct InvoiceStore {
    invoices: HashMap<Uuid, Invoice>,
}

impl InvoiceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, invoice: Invoice) -> bool {
        let existing = self.invoices.values().find(|i| i.filename == invoice.filename);
        let is_replace = existing.is_some();
        if let Some(old) = existing {
            let old_id = old.id;
            self.invoices.remove(&old_id);
        }
        self.invoices.insert(invoice.id, invoice);
        is_replace
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &Uuid) -> Option<&Invoice> {
        self.invoices.get(id)
    }

    pub fn remove(&mut self, id: &Uuid) -> bool {
        self.invoices.remove(id).is_some()
    }

    pub fn list(&self) -> Vec<&Invoice> {
        let mut list: Vec<&Invoice> = self.invoices.values().collect();
        list.sort_by_key(|i| &i.reference_month);
        list
    }

    pub fn for_each_transaction_mut<F>(&mut self, mut f: F) -> usize
    where
        F: FnMut(&mut crate::domain::Transaction) -> bool,
    {
        let mut changed = 0usize;
        for invoice in self.invoices.values_mut() {
            for tx in invoice.transactions.iter_mut() {
                if f(tx) {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn update_transaction_category(&mut self, tx_id: &str, category: &str) -> bool {
        for invoice in self.invoices.values_mut() {
            for tx in invoice.transactions.iter_mut() {
                if tx.id.to_string() == tx_id {
                    tx.category = category.to_string();
                    return true;
                }
            }
        }
        false
    }
}

pub type SharedStore = Arc<Mutex<InvoiceStore>>;

pub fn new_shared_store() -> SharedStore {
    Arc::new(Mutex::new(InvoiceStore::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{invoice::{Invoice, YearMonth}, transaction::Transaction};
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal_macros::dec;

    fn make_invoice(filename: &str) -> Invoice {
        Invoice::new(
            filename.to_string(),
            YearMonth::new(2026, 6),
            None,
            vec![],
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        )
    }

    #[test]
    fn test_add_and_list() {
        let mut store = InvoiceStore::new();
        let inv = make_invoice("fatura.xlsx");
        let id = inv.id;
        store.add(inv);
        assert_eq!(store.list().len(), 1);
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn test_remove() {
        let mut store = InvoiceStore::new();
        let inv = make_invoice("fatura.xlsx");
        let id = inv.id;
        store.add(inv);
        assert!(store.remove(&id));
        assert!(store.get(&id).is_none());
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_duplicate_filename_replaced() {
        let mut store = InvoiceStore::new();
        store.add(make_invoice("fatura.xlsx"));
        store.add(make_invoice("fatura.xlsx"));
        assert_eq!(store.list().len(), 1);
    }
}
