//! Commands: bank statement (extrato) import at the Tauri boundary — preview, save,
//! list, remove and recategorize bank entries.

use std::collections::HashSet;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::domain::account_position::{
    chain_warning, coverage_gaps, current_positions, month_coverage, AccountPosition, Coverage,
    MonthCoverage,
};
use crate::domain::bank_statement::{classify_statement, BankEntry, ClassifiedEntry, ParsedStatement};
use crate::domain::{AppConfig, Categorizer};
use crate::infrastructure::statement_reader::{statement_reader_for, supported_formats};
use crate::infrastructure::db::SharedDb;

#[derive(Debug, Serialize)]
pub struct StatementPreview {
    /// Which bank the file came from — detected from the file, not asked of the user.
    pub bank: String,
    pub holder: String,
    pub account: String,
    pub included: Vec<ClassifiedEntry>,
    pub excluded: Vec<ClassifiedEntry>,
    /// Stock data the statement printed (016) — travels through the preview so
    /// confirming it persists exactly what was read, without re-reading the file.
    pub positions: Vec<AccountPosition>,
    pub coverage: Option<(NaiveDate, NaiveDate)>,
    pub previous_balance: Option<String>,
}

/// What an import/save produced: how many entries, plus the chain check outcome —
/// a warning, never a blocker (a missing statement in between is the common cause).
#[derive(Debug, Serialize)]
pub struct SaveStatementResult {
    pub saved: usize,
    pub chain_warning: Option<String>,
}

/// Persist the stock layer of one statement (positions + coverage) and return the
/// chain warning, if the opening balance disagrees with the previous period.
fn persist_stock(
    db: &State<'_, SharedDb>,
    bank: &str,
    account: &str,
    positions: &[AccountPosition],
    coverage: Option<(NaiveDate, NaiveDate)>,
    previous_balance: Option<Decimal>,
) -> Result<Option<String>, String> {
    let mut guard = db.lock().map_err(|e| e.to_string())?;

    // Compare against what was already stored BEFORE this statement's positions.
    let warning = match (coverage, previous_balance) {
        (Some((start, _)), Some(saldo_anterior)) => {
            let stored = guard.load_account_positions().unwrap_or_default();
            let same_account: Vec<AccountPosition> = stored
                .into_iter()
                .filter(|p| p.bank == bank && p.account == account)
                .collect();
            chain_warning(&same_account, start, saldo_anterior)
        }
        _ => None,
    };

    if !positions.is_empty() {
        guard.save_account_positions(positions)?;
    }
    if let Some((start, end)) = coverage {
        guard.save_statement_coverage(&[Coverage::new(bank, account, start, end, "")])?;
    }
    Ok(warning)
}

/// Strategy dispatch: the registry picks the reader for the file (Banestes ships
/// PDF, BTG a spreadsheet); each strategy fills `ParsedStatement.bank` itself.
fn read_statement(path: &str) -> Result<ParsedStatement, String> {
    match statement_reader_for(path) {
        Some(reader) => reader.read(path),
        None => Err(format!("Formato não suportado. Use {}.", supported_formats())),
    }
}

/// Read + classify a statement file (no persistence).
fn classify_all(
    path: &str,
    config: &State<'_, Mutex<AppConfig>>,
    db: &State<'_, SharedDb>,
) -> Result<(ParsedStatement, Vec<ClassifiedEntry>), String> {
    let parsed = read_statement(path)?;
    let rules = config.lock().map_err(|e| e.to_string())?.category_rules.clone();
    let cz = if rules.is_empty() { Categorizer::with_defaults() } else { Categorizer::new(rules) };
    let payslip_months: HashSet<String> = db
        .lock()
        .map_err(|e| e.to_string())?
        .load_payslips()
        .unwrap_or_default()
        .iter()
        .map(|p| p.month.clone())
        .collect();
    let classified = classify_statement(&parsed, &cz, &payslip_months);
    Ok((parsed, classified))
}

/// Preview what will be imported (included) and what is dropped (excluded + reason).
#[tauri::command]
pub async fn preview_bank_statement(
    path: String,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<StatementPreview, String> {
    let (parsed, classified) = classify_all(&path, &config, &db)?;
    let (included, excluded): (Vec<_>, Vec<_>) = classified.into_iter().partition(|c| c.included);
    Ok(StatementPreview {
        bank: parsed.bank,
        holder: parsed.holder,
        account: parsed.account,
        included,
        excluded,
        positions: parsed.positions,
        coverage: parsed.coverage,
        previous_balance: parsed.previous_balance.map(|d| d.to_string()),
    })
}

/// Import: classify + persist the included entries (dedup). Returns how many were saved.
#[tauri::command]
pub async fn import_bank_statement(
    path: String,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<SaveStatementResult, String> {
    let (parsed, classified) = classify_all(&path, &config, &db)?;
    let entries: Vec<BankEntry> = classified
        .iter()
        .filter(|c| c.included)
        .map(|c| BankEntry::from_classified(c, &parsed.bank, &parsed.account))
        .collect();
    let n = entries.len();
    let chain_warning = persist_stock(
        &db,
        &parsed.bank,
        &parsed.account,
        &parsed.positions,
        parsed.coverage,
        parsed.previous_balance,
    )?;
    db.lock().map_err(|e| e.to_string())?.save_bank_entries(&entries)?;
    Ok(SaveStatementResult { saved: n, chain_warning })
}

/// Save the (possibly re-categorized) included entries from a preview. Dedup by id.
#[tauri::command]
pub async fn save_bank_statement(
    bank: String,
    account: String,
    entries: Vec<ClassifiedEntry>,
    positions: Option<Vec<AccountPosition>>,
    coverage: Option<(NaiveDate, NaiveDate)>,
    previous_balance: Option<String>,
    db: State<'_, SharedDb>,
) -> Result<SaveStatementResult, String> {
    let items: Vec<BankEntry> = entries
        .iter()
        .filter(|c| c.included)
        .map(|c| BankEntry::from_classified(c, &bank, &account))
        .collect();
    let n = items.len();
    let chain_warning = persist_stock(
        &db,
        &bank,
        &account,
        &positions.unwrap_or_default(),
        coverage,
        previous_balance.and_then(|s| s.parse().ok()),
    )?;
    db.lock().map_err(|e| e.to_string())?.save_bank_entries(&items)?;
    Ok(SaveStatementResult { saved: n, chain_warning })
}

/// Current balance per account/product, for the dashboard's "Saldo em conta" card.
#[tauri::command]
pub async fn list_account_positions(db: State<'_, SharedDb>) -> Result<Vec<AccountPosition>, String> {
    let all = db.lock().map_err(|e| e.to_string())?.load_account_positions()?;
    Ok(current_positions(&all))
}

/// Per-account data coverage: which months are partial (and until when) and which
/// months have no statement at all.
#[derive(Debug, Serialize)]
pub struct CoverageSummary {
    pub bank: String,
    pub account: String,
    /// Months (`YYYY-MM`) covered only in part, with the last covered day.
    pub partial_months: Vec<PartialMonth>,
    /// Months (`YYYY-MM`) with no coverage between the first and last statement.
    pub gaps: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PartialMonth {
    pub month: String,
    pub until: String,
}

#[tauri::command]
pub async fn coverage_summary(db: State<'_, SharedDb>) -> Result<Vec<CoverageSummary>, String> {
    let covs = db.lock().map_err(|e| e.to_string())?.load_statement_coverage()?;
    let mut accounts: Vec<(String, String)> =
        covs.iter().map(|c| (c.bank.clone(), c.account.clone())).collect();
    accounts.sort();
    accounts.dedup();

    let mut out = Vec::new();
    for (bank, account) in accounts {
        let mine: Vec<Coverage> = covs
            .iter()
            .filter(|c| c.bank == bank && c.account == account)
            .cloned()
            .collect();
        // Months touched by any coverage, so the UI can flag the partial ones.
        let mut months: Vec<String> = Vec::new();
        for c in &mine {
            let (mut y, mut m) = (c.start.year(), c.start.month());
            loop {
                let key = format!("{y:04}-{m:02}");
                if !months.contains(&key) {
                    months.push(key);
                }
                if (y, m) >= (c.end.year(), c.end.month()) {
                    break;
                }
                if m == 12 {
                    y += 1;
                    m = 1;
                } else {
                    m += 1;
                }
            }
        }
        months.sort();

        let partial_months = months
            .iter()
            .filter_map(|month| match month_coverage(&mine, month) {
                MonthCoverage::Partial(ranges) => ranges.last().map(|(_, until)| PartialMonth {
                    month: month.clone(),
                    until: until.format("%d/%m/%Y").to_string(),
                }),
                _ => None,
            })
            .collect();

        out.push(CoverageSummary {
            bank,
            account,
            partial_months,
            gaps: coverage_gaps(&mine),
        });
    }
    Ok(out)
}

/// Change the category of an already-imported entry.
#[tauri::command]
pub async fn set_bank_entry_category(id: String, category: String, db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.update_bank_entry_category(&id, &category)
}

#[tauri::command]
pub async fn list_bank_entries(db: State<'_, SharedDb>) -> Result<Vec<BankEntry>, String> {
    db.lock().map_err(|e| e.to_string())?.load_bank_entries()
}

#[tauri::command]
pub async fn remove_bank_entry(id: String, db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.remove_bank_entry(&id)
}

#[tauri::command]
pub async fn clear_bank_entries(db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.clear_bank_entries()
}
