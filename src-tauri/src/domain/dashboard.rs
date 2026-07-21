use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::{
    category::{aggregate_by_category, Category, TransactionSummary},
    invoice::Invoice,
    manual_entry::EntryKind,
    transaction::Transaction,
};

/// A manual entry already expanded for the dashboard scope: exactly one item per
/// (entry × month it counts for). `tx` is a synthetic transaction used for expense
/// category aggregation (unused for income).
#[derive(Debug, Clone)]
pub struct ManualAgg {
    pub kind: EntryKind,
    pub month: String,
    pub amount: Decimal,
    pub category: String,
    pub tx: Transaction,
    /// Income only: whether this is salary (used to let a payslip supersede it).
    pub is_salary: bool,
    /// Expense only: true for payroll deductions (kept out of "contas fixas").
    pub is_payroll: bool,
    /// Whether this comes from a recurring entry (fixo) vs a one-off (avulso).
    pub recurring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardFilter {
    pub invoice_ids: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySnapshot {
    pub name: String,
    pub net_total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySnapshot {
    pub month: String,
    pub net_total: String,
    pub categories: Vec<CategorySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPeriod {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentSummary {
    pub description: String,
    pub current: u8,
    pub total: u8,
    pub amount: String,
    /// Installments still to be charged after this one: total − current.
    pub remaining: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionSummary {
    pub name: String,
    pub total: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub period: DashboardPeriod,
    pub total_charged: String,
    pub total_reversals: String,
    /// Grand total of EXPENSES (card net + manual fixed expenses). Categories sum to this.
    pub net_total: String,
    /// Card net only (total_charged + total_reversals).
    pub total_card_net: String,
    /// Sum of recurring manual fixed expenses in scope (contas fixas — excludes payroll).
    pub total_manual_expense: String,
    /// Sum of one-off (avulso) manual expenses in scope.
    pub total_variable_expense: String,
    /// Sum of payroll deductions (folha) in scope.
    pub total_payroll_deductions: String,
    /// Sum of manual income (crédito) in scope.
    pub total_income: String,
    /// total_income − net_total (positive = sobra, negative = déficit).
    pub balance: String,
    pub invoice_count: u32,
    pub categories: Vec<Category>,
    pub top_transactions: Vec<TransactionSummary>,
    pub monthly_trend: Vec<MonthlySnapshot>,
    /// Card spending by weekday, Monday..Sunday (7 decimal strings).
    pub weekday_spending: Vec<String>,
    /// Card installment line items in scope, largest first.
    pub installments: Vec<InstallmentSummary>,
    /// Sum of installment charges in this month.
    pub installments_month_total: String,
    /// Sum of remaining installments already committed (remaining × amount).
    pub installments_future_total: String,
    /// Detected recurring subscriptions (streaming, SaaS…) on the card.
    pub subscriptions: Vec<SubscriptionSummary>,
    pub subscriptions_total: String,
}

pub fn compute_dashboard(
    invoices: &[Invoice],
    manual: &[ManualAgg],
    _filter: &DashboardFilter,
) -> DashboardData {
    let all_transactions: Vec<&Transaction> = invoices
        .iter()
        .flat_map(|inv| inv.transactions.iter())
        .collect();

    let total_charged: Decimal = all_transactions
        .iter()
        .filter(|t| !t.is_reversal)
        .map(|t| t.amount)
        .fold(dec!(0), |acc, a| acc + a);

    let total_reversals: Decimal = all_transactions
        .iter()
        .filter(|t| t.is_reversal)
        .map(|t| t.amount)
        .fold(dec!(0), |acc, a| acc + a);

    let total_card_net = total_charged + total_reversals;

    // Manual fixed expenses ("contas fixas") — recurring, excludes payroll deductions.
    let total_manual_expense: Decimal = manual
        .iter()
        .filter(|m| m.kind == EntryKind::Expense && !m.is_payroll && m.recurring)
        .map(|m| m.amount)
        .fold(dec!(0), |acc, a| acc + a);

    // One-off (avulso) manual expenses — counted in the grand total, shown apart.
    let total_variable_expense: Decimal = manual
        .iter()
        .filter(|m| m.kind == EntryKind::Expense && !m.is_payroll && !m.recurring)
        .map(|m| m.amount)
        .fold(dec!(0), |acc, a| acc + a);

    // Payroll deductions (folha) — counted in the grand total, but shown apart.
    let total_payroll_deductions: Decimal = manual
        .iter()
        .filter(|m| m.kind == EntryKind::Expense && m.is_payroll)
        .map(|m| m.amount)
        .fold(dec!(0), |acc, a| acc + a);

    let total_income: Decimal = manual
        .iter()
        .filter(|m| m.kind == EntryKind::Income)
        .map(|m| m.amount)
        .fold(dec!(0), |acc, a| acc + a);

    // Expense grand total = card + fixed + one-off manual expenses + payroll deductions.
    let net_total = total_card_net + total_manual_expense + total_variable_expense + total_payroll_deductions;
    let balance = total_income - net_total;

    // Category aggregation over card charges + synthetic manual-expense transactions.
    let mut txs_owned: Vec<Transaction> = all_transactions.iter().map(|t| (*t).clone()).collect();
    for m in manual.iter().filter(|m| m.kind == EntryKind::Expense) {
        txs_owned.push(m.tx.clone());
    }
    let categories = aggregate_by_category(&txs_owned, net_total);

    // Top transactions stay card-only (the real invoice line items).
    let card_owned: Vec<Transaction> = all_transactions.iter().map(|t| (*t).clone()).collect();
    let top_transactions = top_5_transactions(&card_owned);

    // Card spending by weekday (Mon..Sun) and installment load — card charges only.
    let (weekday_spending, installments, installments_month_total, installments_future_total) =
        compute_weekday_and_installments(&all_transactions);

    let (subscriptions, subscriptions_total) = compute_subscriptions(&all_transactions);

    let month_count = distinct_months(invoices, manual);
    let monthly_trend = if month_count >= 2 {
        compute_monthly_trend(invoices, manual)
    } else {
        vec![]
    };

    let mut all_months: Vec<String> = invoices
        .iter()
        .map(|i| i.reference_month.to_string_iso())
        .collect();
    all_months.extend(manual.iter().map(|m| m.month.clone()));
    let from = all_months.iter().min().cloned().unwrap_or_default();
    let to = all_months.iter().max().cloned().unwrap_or_default();

    DashboardData {
        period: DashboardPeriod { from, to },
        total_charged: total_charged.to_string(),
        total_reversals: total_reversals.to_string(),
        net_total: net_total.to_string(),
        total_card_net: total_card_net.to_string(),
        total_manual_expense: total_manual_expense.to_string(),
        total_variable_expense: total_variable_expense.to_string(),
        total_payroll_deductions: total_payroll_deductions.to_string(),
        total_income: total_income.to_string(),
        balance: balance.to_string(),
        invoice_count: invoices.len() as u32,
        categories,
        top_transactions,
        monthly_trend,
        weekday_spending,
        installments,
        installments_month_total: installments_month_total.to_string(),
        installments_future_total: installments_future_total.to_string(),
        subscriptions,
        subscriptions_total: subscriptions_total.to_string(),
    }
}

/// Detect recurring subscriptions on the card by matching known brand keywords
/// in the transaction description. Returns (summaries desc by total, grand total).
fn compute_subscriptions(card_txs: &[&Transaction]) -> (Vec<SubscriptionSummary>, Decimal) {
    // (keyword, canonical name) — keyword matched against uppercased description.
    const RULES: &[(&str, &str)] = &[
        ("NETFLIX", "Netflix"),
        ("SPOTIFY", "Spotify"),
        ("AMAZON PRIME", "Amazon Prime"),
        ("PRIME VIDEO", "Amazon Prime"),
        ("AMAZONPRIME", "Amazon Prime"),
        ("PRIME", "Amazon Prime"),
        ("YOUTUBE", "YouTube"),
        ("DISNEY", "Disney+"),
        ("HBO", "HBO Max"),
        ("PARAMOUNT", "Paramount+"),
        ("GLOBOPLAY", "Globoplay"),
        ("DEEZER", "Deezer"),
        ("APPLE.COM", "Apple"),
        ("APPLE TV", "Apple TV+"),
        ("ICLOUD", "iCloud"),
        ("GOOGLE", "Google"),
        ("OPENAI", "OpenAI"),
        ("CHATGPT", "OpenAI"),
        ("CANVA", "Canva"),
        ("MICROSOFT", "Microsoft"),
        ("LINKEDIN", "LinkedIn"),
        ("DROPBOX", "Dropbox"),
        ("STEAM", "Steam"),
    ];

    use std::collections::HashMap;
    let mut map: HashMap<&str, (Decimal, u32)> = HashMap::new();
    for t in card_txs.iter().filter(|t| !t.is_reversal) {
        let up = t.description.to_uppercase();
        for (kw, name) in RULES {
            if up.contains(kw) {
                let e = map.entry(*name).or_insert((dec!(0), 0));
                e.0 += t.amount;
                e.1 += 1;
                break; // first matching rule wins (order = priority)
            }
        }
    }

    let total: Decimal = map.values().map(|(v, _)| *v).fold(dec!(0), |a, b| a + b);
    let mut subs: Vec<SubscriptionSummary> = map
        .into_iter()
        .map(|(name, (v, c))| SubscriptionSummary {
            name: name.to_string(),
            total: v.to_string(),
            count: c,
        })
        .collect();
    subs.sort_by(|a, b| {
        let aa: Decimal = a.total.parse().unwrap_or(dec!(0));
        let bb: Decimal = b.total.parse().unwrap_or(dec!(0));
        bb.cmp(&aa)
    });
    (subs, total)
}

/// Returns (weekday totals Mon..Sun, installment summaries desc, month total, future committed total).
fn compute_weekday_and_installments(
    card_txs: &[&Transaction],
) -> (Vec<String>, Vec<InstallmentSummary>, Decimal, Decimal) {
    use chrono::Datelike;

    let mut weekday = [dec!(0); 7];
    let mut installments: Vec<InstallmentSummary> = Vec::new();
    let mut month_total = dec!(0);
    let mut future_total = dec!(0);

    for t in card_txs.iter().filter(|t| !t.is_reversal) {
        let idx = t.date.weekday().num_days_from_monday() as usize;
        weekday[idx] += t.amount;

        if let Some(inst) = &t.installment {
            let remaining = inst.total.saturating_sub(inst.current);
            month_total += t.amount;
            future_total += t.amount * Decimal::from(remaining);
            installments.push(InstallmentSummary {
                description: t.description.clone(),
                current: inst.current,
                total: inst.total,
                amount: t.amount.to_string(),
                remaining,
            });
        }
    }

    installments.sort_by(|a, b| {
        let aa: Decimal = a.amount.parse().unwrap_or(dec!(0));
        let bb: Decimal = b.amount.parse().unwrap_or(dec!(0));
        bb.cmp(&aa)
    });

    let weekday_spending = weekday.iter().map(|d| d.to_string()).collect();
    (weekday_spending, installments, month_total, future_total)
}

fn distinct_months(invoices: &[Invoice], manual: &[ManualAgg]) -> usize {
    let mut set = std::collections::BTreeSet::new();
    for i in invoices {
        set.insert(i.reference_month.to_string_iso());
    }
    for m in manual {
        set.insert(m.month.clone());
    }
    set.len()
}

fn top_5_transactions(transactions: &[Transaction]) -> Vec<TransactionSummary> {
    let mut charges: Vec<&Transaction> = transactions.iter().filter(|t| !t.is_reversal).collect();
    charges.sort_by_key(|b| std::cmp::Reverse(b.amount));
    charges
        .iter()
        .take(5)
        .map(|t| TransactionSummary::from_transaction(t))
        .collect()
}

fn compute_monthly_trend(invoices: &[Invoice], manual: &[ManualAgg]) -> Vec<MonthlySnapshot> {
    use std::collections::BTreeMap;

    // Group card + synthetic manual-expense transactions by ISO month.
    let mut by_month: BTreeMap<String, Vec<Transaction>> = BTreeMap::new();
    for inv in invoices {
        by_month
            .entry(inv.reference_month.to_string_iso())
            .or_default()
            .extend(inv.transactions.iter().cloned());
    }
    for m in manual.iter().filter(|m| m.kind == EntryKind::Expense) {
        by_month.entry(m.month.clone()).or_default().push(m.tx.clone());
    }

    by_month
        .into_iter()
        .map(|(month, txs)| {
            let net: Decimal = txs.iter().map(|t| t.amount).fold(dec!(0), |a, b| a + b);
            let cats = aggregate_by_category(&txs, net);
            let cat_snapshots: Vec<CategorySnapshot> = cats
                .into_iter()
                .map(|c| CategorySnapshot {
                    name: c.name,
                    net_total: c.net_total,
                })
                .collect();
            MonthlySnapshot {
                month,
                net_total: net.to_string(),
                categories: cat_snapshots,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::invoice::YearMonth;
    use crate::domain::manual_entry::ManualEntry;
    use crate::domain::{invoice::Invoice, transaction::Transaction};
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn make_invoice(year: i32, month: u8, amounts: &[Decimal]) -> Invoice {
        let id = Uuid::new_v4();
        let txs: Vec<Transaction> = amounts
            .iter()
            .enumerate()
            .map(|(i, &amt)| {
                Transaction::new(
                    id,
                    i as u32,
                    NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap(),
                    format!("TX {i}"),
                    amt,
                    "Outros".to_string(),
                    None,
                )
            })
            .collect();
        Invoice::new(
            format!("{year}-{month:02}-fatura.xlsx"),
            YearMonth::new(year, month),
            None,
            txs,
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        )
    }

    fn agg(kind: EntryKind, month: &str, amount: Decimal, category: &str) -> ManualAgg {
        let entry = ManualEntry::new(
            kind,
            "manual".into(),
            amount,
            category.into(),
            month.into(),
            true,
        );
        ManualAgg {
            kind,
            month: month.to_string(),
            amount,
            category: category.to_string(),
            tx: entry.to_transaction(month),
            is_salary: entry.is_salary,
            is_payroll: false,
            recurring: entry.recurring,
        }
    }

    #[test]
    fn test_top_transactions_returns_5_largest() {
        let amounts: Vec<Decimal> = vec![
            dec!(10), dec!(50), dec!(30), dec!(100), dec!(20), dec!(70), dec!(5),
        ];
        let txs: Vec<Transaction> = amounts
            .iter()
            .enumerate()
            .map(|(i, &amt)| {
                let inv_id = Uuid::new_v4();
                Transaction::new(inv_id, i as u32, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), "D".to_string(), amt, "Outros".to_string(), None)
            })
            .collect();
        let top = top_5_transactions(&txs);
        assert_eq!(top.len(), 5);
        let first: Decimal = top[0].amount.parse().unwrap();
        assert_eq!(first, dec!(100));
    }

    #[test]
    fn test_monthly_trend_empty_when_single_invoice() {
        let inv = make_invoice(2026, 6, &[dec!(100)]);
        let filter = DashboardFilter::default();
        let data = compute_dashboard(&[inv], &[], &filter);
        assert!(data.monthly_trend.is_empty());
    }

    #[test]
    fn test_monthly_trend_two_months() {
        let inv1 = make_invoice(2026, 5, &[dec!(100)]);
        let inv2 = make_invoice(2026, 6, &[dec!(200)]);
        let filter = DashboardFilter::default();
        let data = compute_dashboard(&[inv1, inv2], &[], &filter);
        assert_eq!(data.monthly_trend.len(), 2);
        assert_eq!(data.monthly_trend[0].month, "2026-05");
        assert_eq!(data.monthly_trend[1].month, "2026-06");
    }

    #[test]
    fn test_manual_income_and_expense_totals() {
        let inv = make_invoice(2026, 6, &[dec!(1000)]);
        let manual = vec![
            agg(EntryKind::Expense, "2026-06", dec!(2950), "Moradia & Serviços"),
            agg(EntryKind::Income, "2026-06", dec!(8000), "Salário"),
        ];
        let data = compute_dashboard(&[inv], &manual, &DashboardFilter::default());
        assert_eq!(data.total_card_net, "1000");
        assert_eq!(data.total_manual_expense, "2950");
        assert_eq!(data.total_income, "8000");
        // net expenses = 1000 + 2950 = 3950; balance = 8000 - 3950 = 4050
        assert_eq!(data.net_total, "3950");
        assert_eq!(data.balance, "4050");
    }

    #[test]
    fn test_manual_expense_enters_category_aggregation() {
        let inv = make_invoice(2026, 6, &[dec!(500)]); // category "Outros"
        let manual = vec![agg(EntryKind::Expense, "2026-06", dec!(2950), "Moradia & Serviços")];
        let data = compute_dashboard(&[inv], &manual, &DashboardFilter::default());
        let moradia = data.categories.iter().find(|c| c.name == "Moradia & Serviços");
        assert!(moradia.is_some(), "manual expense must create/join a category");
        assert_eq!(moradia.unwrap().net_total, "2950");
        // income never appears as an expense category
        assert!(data.categories.iter().all(|c| c.name != "Salário"));
    }

    #[test]
    fn test_negative_balance_when_expenses_exceed_income() {
        let inv = make_invoice(2026, 6, &[dec!(5000)]);
        let manual = vec![agg(EntryKind::Income, "2026-06", dec!(3000), "Salário")];
        let data = compute_dashboard(&[inv], &manual, &DashboardFilter::default());
        assert_eq!(data.balance, "-2000");
    }
}
