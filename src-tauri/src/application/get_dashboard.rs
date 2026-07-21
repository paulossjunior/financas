use std::collections::BTreeSet;

use crate::domain::dashboard::ManualAgg;
use crate::domain::manual_entry::ManualEntry;
use crate::domain::{compute_dashboard, invoice::Invoice, DashboardData, DashboardFilter};

use super::store::InvoiceStore;

pub fn get_dashboard(
    store: &InvoiceStore,
    manual_entries: &[ManualEntry],
    filter: DashboardFilter,
) -> Result<DashboardData, String> {
    let all: Vec<&Invoice> = store.list();

    // Apply invoice_ids filter (month scoping resolves to a set of invoice ids on the frontend).
    let selected: Vec<Invoice> = match &filter.invoice_ids {
        Some(ids) => all
            .iter()
            .filter(|i| ids.contains(&i.id.to_string()))
            .map(|i| (*i).clone())
            .collect(),
        None => all.iter().map(|i| (*i).clone()).collect(),
    };

    if selected.is_empty() && manual_entries.is_empty() {
        return Err("NO_DATA".into());
    }

    let scope_months: BTreeSet<String> = selected
        .iter()
        .map(|i| i.reference_month.to_string_iso())
        .collect();

    let manual_agg = expand_manual(manual_entries, &scope_months);

    Ok(compute_dashboard(&selected, &manual_agg, &filter))
}

/// Expand each manual entry into one ManualAgg per month it counts for.
/// Recurring entries count for every month in scope (or their own month when scope is empty);
/// one-off entries count only for their own month when it is in scope.
fn expand_manual(entries: &[ManualEntry], scope_months: &BTreeSet<String>) -> Vec<ManualAgg> {
    let mut out = Vec::new();
    for e in entries {
        let months: Vec<String> = if e.recurring {
            if scope_months.is_empty() {
                vec![e.month.clone()]
            } else {
                scope_months.iter().cloned().collect()
            }
        } else if scope_months.is_empty() || scope_months.contains(&e.month) {
            vec![e.month.clone()]
        } else {
            vec![]
        };

        for m in months {
            out.push(ManualAgg {
                kind: e.kind,
                month: m.clone(),
                amount: e.amount,
                category: e.category.clone(),
                tx: e.to_transaction(&m),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::manual_entry::{EntryKind, ManualEntry};
    use rust_decimal_macros::dec;

    #[test]
    fn recurring_counts_once_when_no_invoices() {
        let entries = vec![ManualEntry::new(
            EntryKind::Expense,
            "Aluguel".into(),
            dec!(2950),
            "Moradia & Serviços".into(),
            "2026-06".into(),
            true,
        )];
        let store = InvoiceStore::new();
        let data = get_dashboard(&store, &entries, DashboardFilter::default()).unwrap();
        assert_eq!(data.total_manual_expense, "2950");
        assert_eq!(data.net_total, "2950");
    }

    #[test]
    fn empty_everything_is_no_data() {
        let store = InvoiceStore::new();
        let err = get_dashboard(&store, &[], DashboardFilter::default());
        assert!(err.is_err());
    }
}
