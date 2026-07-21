use std::collections::BTreeSet;

use crate::domain::dashboard::ManualAgg;
use crate::domain::manual_entry::{EntryKind, ManualEntry};
use crate::domain::payslip::Payslip;
use crate::domain::transaction::Transaction;
use crate::domain::{compute_dashboard, invoice::Invoice, DashboardData, DashboardFilter};

use super::store::InvoiceStore;

pub fn get_dashboard(
    store: &InvoiceStore,
    manual_entries: &[ManualEntry],
    payslips: &[Payslip],
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

    if selected.is_empty() && manual_entries.is_empty() && payslips.is_empty() {
        return Err("NO_DATA".into());
    }

    let scope_months: BTreeSet<String> = selected
        .iter()
        .map(|i| i.reference_month.to_string_iso())
        .collect();

    let mut manual_agg = expand_manual(manual_entries, &scope_months);

    // A payslip is the real net income for its month: it supersedes the manual *salary*
    // income (avoid double counting) while non-salary manual income (e.g. bolsa) still counts.
    let in_scope = |m: &str| scope_months.is_empty() || scope_months.contains(m);
    let payslip_months: BTreeSet<String> = payslips
        .iter()
        .map(|p| p.month.clone())
        .filter(|m| in_scope(m))
        .collect();
    manual_agg.retain(|a| !(a.kind == EntryKind::Income && a.is_salary && payslip_months.contains(&a.month)));
    for p in payslips.iter().filter(|p| in_scope(&p.month)) {
        manual_agg.push(payslip_income_agg(p));
    }

    Ok(compute_dashboard(&selected, &manual_agg, &filter))
}

/// Synthetic income aggregate carrying a payslip's net líquido for its month.
fn payslip_income_agg(p: &Payslip) -> ManualAgg {
    let date = crate::domain::manual_entry::parse_month_start(&p.month)
        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let inv = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, format!("payslip:{}", p.month).as_bytes());
    let tx = Transaction::new(inv, 0, date, "Salário (contracheque)".into(), p.net, "Salário".into(), None);
    ManualAgg {
        kind: EntryKind::Income,
        month: p.month.clone(),
        amount: p.net,
        category: "Salário".into(),
        tx,
        is_salary: true,
    }
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
                is_salary: e.is_salary,
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
        let data = get_dashboard(&store, &entries, &[], DashboardFilter::default()).unwrap();
        assert_eq!(data.total_manual_expense, "2950");
        assert_eq!(data.net_total, "2950");
    }

    #[test]
    fn empty_everything_is_no_data() {
        let store = InvoiceStore::new();
        let err = get_dashboard(&store, &[], &[], DashboardFilter::default());
        assert!(err.is_err());
    }
}
