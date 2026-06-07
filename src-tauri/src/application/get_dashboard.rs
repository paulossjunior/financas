use crate::domain::{compute_dashboard, DashboardData, DashboardFilter, invoice::Invoice};
use super::store::InvoiceStore;

pub fn get_dashboard(store: &InvoiceStore, filter: DashboardFilter) -> Result<DashboardData, String> {
    let invoices: Vec<&Invoice> = store.list();
    if invoices.is_empty() {
        return Err("NO_DATA".into());
    }
    let owned: Vec<Invoice> = invoices.into_iter().cloned().collect();
    Ok(compute_dashboard(&owned, &filter))
}
