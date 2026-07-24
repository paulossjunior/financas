//! Application layer: [`InvoiceStore`], the in-memory store of imported invoices
//! (keyed by UUID), shared behind a mutex as `SharedStore`.

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

    /// Replace the whole store with a fresh set of invoices (used after a database
    /// restore, so in-memory state matches the swapped-in database).
    pub fn replace_all(&mut self, invoices: Vec<Invoice>) {
        self.invoices.clear();
        for inv in invoices {
            self.invoices.insert(inv.id, inv);
        }
    }

    pub fn list(&self) -> Vec<&Invoice> {
        let mut list: Vec<&Invoice> = self.invoices.values().collect();
        list.sort_by_key(|i| &i.reference_month);
        list
    }

    /// Owned, sorted snapshot of all invoices — used to persist to the database.
    pub fn list_owned(&self) -> Vec<Invoice> {
        self.list().into_iter().cloned().collect()
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

    pub fn list_all_transactions(&self) -> Vec<crate::domain::Transaction> {
        let mut txs: Vec<crate::domain::Transaction> = self
            .invoices
            .values()
            .flat_map(|inv| inv.transactions.iter().cloned())
            .collect();
        txs.sort_by_key(|t| std::cmp::Reverse(t.date));
        txs
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

/// Empty shared store — used by tests.
#[cfg(test)]
pub fn new_shared_store() -> SharedStore {
    Arc::new(Mutex::new(InvoiceStore::new()))
}

/// Build a store preloaded with invoices (e.g. loaded from the database on startup).
pub fn shared_store_with(invoices: Vec<Invoice>) -> SharedStore {
    let mut store = InvoiceStore::new();
    for inv in invoices {
        store.invoices.insert(inv.id, inv);
    }
    Arc::new(Mutex::new(store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::invoice::{Invoice, YearMonth};


    fn make_invoice(filename: &str) -> Invoice {
        Invoice::new(
            filename.to_string(),
            YearMonth::new(2026, 6),
            None,
            vec![],
            chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
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
