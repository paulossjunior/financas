//! BTG bank-statement parsing + classification (pure domain).
//!
//! Turns the statement rows into credit/debit entries, drops what the app
//! already counts (card bill, salary when a payslip exists, internal transfers),
//! and categorizes the rest (app rules, BTG category as fallback).

use std::collections::HashSet;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::categorizer::Categorizer;

/// A raw statement line (before classification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEntry {
    pub date: String,  // "YYYY-MM-DD"
    pub month: String, // "YYYY-MM"
    pub btg_category: String,
    pub transaction: String,
    pub description: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal, // signed: negative = debit
}

/// A classified entry (what to save / show in the preview).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedEntry {
    pub id: String,
    pub date: String,
    pub month: String,
    pub description: String,
    pub btg_category: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub kind: String,     // "income" | "expense"
    pub category: String, // app category (BTG fallback)
    pub included: bool,
    pub reason: String, // "" | "fatura" | "salario" | "interno"
}

/// A persisted, included bank entry (feeds the dashboard as avulso/renda).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankEntry {
    pub id: String,
    pub bank: String,
    pub account: String,
    pub date: String,
    pub month: String,
    pub description: String,
    pub category: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal, // signed: negative = debit
    pub kind: String, // "income" | "expense"
}

impl BankEntry {
    pub fn from_classified(c: &ClassifiedEntry, bank: &str, account: &str) -> Self {
        Self {
            id: c.id.clone(),
            bank: bank.to_string(),
            account: account.to_string(),
            date: c.date.clone(),
            month: c.month.clone(),
            description: c.description.clone(),
            category: c.category.clone(),
            amount: c.amount,
            kind: c.kind.clone(),
        }
    }

    /// Convert to a ManualEntry (avulso expense / extra income) so it flows through
    /// the existing dashboard/year pipeline. Never salary (salary is excluded upstream).
    pub fn to_manual_entry(&self) -> super::manual_entry::ManualEntry {
        use super::manual_entry::{EntryKind, ManualEntry};
        let kind = if self.kind == "income" { EntryKind::Income } else { EntryKind::Expense };
        let mut m = ManualEntry::new(kind, self.description.clone(), self.amount.abs(), self.category.clone(), self.month.clone(), false);
        m.is_salary = false;
        m
    }
}

/// Result of parsing a statement file.
///
/// `bank` identifies which reader produced it ("BTG", "Banestes"), so the bank
/// travels with the data instead of being guessed again at each call site.
/// `positions`/`coverage` (feature 016) carry the stock data the statement prints —
/// final balances and the covered period — when the file provides them; the
/// `source_file` of each position is stamped by the layer that knows the filename.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedStatement {
    #[serde(default)]
    pub bank: String,
    pub holder: String,
    pub account: String,
    pub entries: Vec<RawEntry>,
    #[serde(default)]
    pub positions: Vec<crate::domain::account_position::AccountPosition>,
    #[serde(default)]
    pub coverage: Option<(chrono::NaiveDate, chrono::NaiveDate)>,
    /// Opening balance the statement printed ("Saldo Anterior"), used to check the
    /// chain against the previously imported period (016). `None` = not printed.
    #[serde(default)]
    pub previous_balance: Option<Decimal>,
}

/// Uppercase, strip accents, collapse whitespace — for robust name/keyword matching.
/// Shared with the per-bank readers so every statement normalizes identically.
pub(crate) fn norm(s: &str) -> String {
    let folded: String = s
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' | 'Ç' => 'C',
            other => other.to_ascii_uppercase(),
        })
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a money cell: plain decimal, or Brazilian format ("1.234,56").
pub(crate) fn parse_amount(s: &str) -> Option<Decimal> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    Decimal::from_str(t)
        .ok()
        .or_else(|| Decimal::from_str(&t.replace('.', "").replace(',', ".")).ok())
}

/// "01/01/2026 13:06" (or just "01/01/2026") → ("2026-01-01", "2026-01").
fn parse_date(s: &str) -> Option<(String, String)> {
    let d = s.split_whitespace().next()?; // date part
    let p: Vec<&str> = d.split('/').collect();
    if p.len() != 3 {
        return None;
    }
    let (dd, mm, yyyy) = (p[0], p[1], p[2]);
    if dd.len() != 2 || mm.len() != 2 || yyyy.len() != 4 || !yyyy.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((format!("{yyyy}-{mm}-{dd}"), format!("{yyyy}-{mm}")))
}

fn cell(row: &[String], idx: Option<usize>) -> String {
    idx.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// Parse statement rows (already stringified) into holder + account + entries.
pub fn parse_statement_rows(rows: &[Vec<String>]) -> ParsedStatement {
    let mut out = ParsedStatement::default();
    // Metadata: "Cliente:" / "Conta:"
    for r in rows {
        let k = norm(r.first().map(|s| s.as_str()).unwrap_or(""));
        if k.starts_with("CLIENTE") && out.holder.is_empty() {
            out.holder = r.get(1).cloned().unwrap_or_default().trim().to_string();
        } else if k.starts_with("CONTA") && out.account.is_empty() {
            out.account = r.get(1).cloned().unwrap_or_default().trim().to_string();
        }
    }
    // Header row: contains "Data e hora" and "Valor".
    let mut cols = None;
    let mut header_idx = 0;
    for (i, r) in rows.iter().enumerate() {
        let norms: Vec<String> = r.iter().map(|c| norm(c)).collect();
        let find = |name: &str| norms.iter().position(|c| c == name);
        if norms.iter().any(|c| c == "DATA E HORA") && norms.iter().any(|c| c == "VALOR") {
            cols = Some((
                find("DATA E HORA"),
                find("CATEGORIA"),
                find("TRANSACAO"),
                find("DESCRICAO"),
                find("VALOR"),
            ));
            header_idx = i;
            break;
        }
    }
    let Some((c_date, c_cat, c_trans, c_desc, c_val)) = cols else {
        return out;
    };

    // Last printed "Saldo Diário" → a best-effort account position (016). The line
    // never becomes an entry; the grid prints it with a date and the day's balance.
    let mut last_daily: Option<(chrono::NaiveDate, Decimal)> = None;

    for r in rows.iter().skip(header_idx + 1) {
        let desc = cell(r, c_desc);
        if norm(&desc) == "SALDO DIARIO" {
            if let (Some((date, _)), Some(balance)) =
                (parse_date(&cell(r, c_date)), parse_amount(&cell(r, c_val)))
            {
                if let Ok(d) = date.parse::<chrono::NaiveDate>() {
                    if last_daily.is_none_or(|(prev, _)| d > prev) {
                        last_daily = Some((d, balance));
                    }
                }
            }
            continue;
        }
        let date_raw = cell(r, c_date);
        let Some((date, month)) = parse_date(&date_raw) else {
            continue;
        };
        let Some(amount) = parse_amount(&cell(r, c_val)) else {
            continue;
        };
        out.entries.push(RawEntry {
            date,
            month,
            btg_category: cell(r, c_cat),
            transaction: cell(r, c_trans),
            description: desc,
            amount,
        });
    }
    if let Some((as_of, balance)) = last_daily {
        use crate::domain::account_position::{AccountPosition, Product};
        // Bank is stamped by the reader after this returns; rebuilt there with it.
        out.positions =
            vec![AccountPosition::new("", &out.account, Product::Corrente, balance, as_of, "")];
    }
    out
}

/// Identity key of a statement line. **Do not change**: every bank entry already
/// persisted was hashed from this exact string, so any edit re-imports as new rows.
fn entry_key(account: &str, e: &RawEntry) -> String {
    format!("bank:{}:{}:{}:{}", account, e.date, norm(&e.description), e.amount)
}

/// Deterministic id so re-importing the same line does not duplicate it.
pub fn entry_id(account: &str, e: &RawEntry) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, entry_key(account, e).as_bytes()).to_string()
}

/// Ids for a whole statement, in order.
///
/// Two genuinely different lines can share date + description + amount (two equal
/// pix to the same payee on the same day). Hashing only that key would collapse
/// them into one row — money silently undercounted. Repeats therefore get an
/// occurrence suffix, while the **first** occurrence keeps the legacy key so no
/// already-imported entry changes id.
pub fn entry_ids(account: &str, entries: &[RawEntry]) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    entries
        .iter()
        .map(|e| {
            let base = entry_key(account, e);
            let n = seen.entry(base.clone()).or_insert(0);
            let key = if *n == 0 { base } else { format!("{base}#{n}") };
            *n += 1;
            Uuid::new_v5(&Uuid::NAMESPACE_OID, key.as_bytes()).to_string()
        })
        .collect()
}

/// Classify one entry: kind, category, and whether it is excluded (and why).
pub fn classify_entry(
    e: &RawEntry,
    account: &str,
    holder_norm: &str,
    has_payslip_month: bool,
    categorizer: &Categorizer,
) -> ClassifiedEntry {
    let kind = if e.amount < Decimal::ZERO { "expense" } else { "income" };
    let ntrans = norm(&e.transaction);
    let ndesc = norm(&e.description);
    let ncat = norm(&e.btg_category);

    // Card-bill payment. Wording differs per bank ("Pagamento de fatura do cartão"
    // at BTG, "Pagamento Fatura Cartao" at Banestes), so match the two tokens.
    let has_card_bill = |s: &str| s.contains("FATURA") && s.contains("CART");
    let is_card = has_card_bill(&ntrans) || has_card_bill(&ndesc);
    let is_salary = (ncat == "SALARIO" || ntrans.contains("SALARIO")) && has_payslip_month;
    // Internal transfer = the description carries the account holder's name. The holder
    // metadata may come with a broken byte (U+FFFD), so match by tokens: every "clean"
    // holder token (len ≥ 3, no replacement char) must appear in the description.
    let holder_tokens: Vec<&str> = holder_norm
        .split_whitespace()
        .filter(|t| t.chars().count() >= 3 && !t.contains('\u{FFFD}'))
        .collect();
    let desc_tokens: std::collections::HashSet<&str> = ndesc.split_whitespace().collect();
    let is_internal = holder_tokens.len() >= 2 && holder_tokens.iter().all(|t| desc_tokens.contains(t));

    let (included, reason) = if is_card {
        (false, "fatura")
    } else if is_salary {
        (false, "salario")
    } else if is_internal {
        (false, "interno")
    } else {
        (true, "")
    };

    // Category: app rules on the description; fall back to the BTG category.
    let mut category = categorizer.categorize(&e.description);
    if category == "Outros" && !e.btg_category.trim().is_empty() {
        category = e.btg_category.trim().to_string();
    }

    ClassifiedEntry {
        id: entry_id(account, e),
        date: e.date.clone(),
        month: e.month.clone(),
        description: e.description.clone(),
        btg_category: e.btg_category.clone(),
        amount: e.amount,
        kind: kind.to_string(),
        category,
        included,
        reason: reason.to_string(),
    }
}

/// Normalized holder name for matching (exposed for the command layer).
pub fn holder_key(holder: &str) -> String {
    norm(holder)
}

/// Classify every entry of a parsed statement (app rules + BTG fallback), marking
/// which are included vs dropped. Shared by the manual-preview command and the
/// folder auto-importer so both apply identical rules.
pub fn classify_statement(
    parsed: &ParsedStatement,
    categorizer: &Categorizer,
    payslip_months: &HashSet<String>,
) -> Vec<ClassifiedEntry> {
    let hk = holder_key(&parsed.holder);
    let ids = entry_ids(&parsed.account, &parsed.entries);
    parsed
        .entries
        .iter()
        .zip(ids)
        .map(|(e, id)| {
            let mut c =
                classify_entry(e, &parsed.account, &hk, payslip_months.contains(&e.month), categorizer);
            c.id = id;
            c
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Vec<String>> {
        let r = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        vec![
            r(&["Cliente:", "Paulo Sérgio Dos Santos Júnior"]),
            r(&["Conta:", "286969-2"]),
            r(&["Data e hora", "Categoria", "Transação", "", "", "Descrição", "", "", "", "Valor"]),
            r(&["01/01/2026 13:06", "Transferência", "Transferência recebida", "", "", "Paulo Sergio Dos Santos Junior", "", "", "", "6000"]),
            r(&["01/01/2026 13:32", "Cuidados Pessoais", "Pix enviado", "", "", "Ln Sports", "", "", "", "-340"]),
            r(&["02/01/2026 10:20", "Salário", "Portabilidade de salário", "", "", "Pagamento recebido", "", "", "", "41659.08"]),
            r(&["02/01/2026 14:46", "Contas", "Pagamento de fatura do cartão", "", "", "Fatura do cartão BTG Pactual", "", "", "", "-10803.36"]),
            r(&["01/01/2026 23:59", "", "", "", "", "Saldo Diário", "", "", "", "5160"]),
            r(&["", "", "", "", "", "", "", "", "", ""]),
        ]
    }

    #[test]
    fn parses_holder_account_and_skips_noise() {
        let p = parse_statement_rows(&rows());
        assert_eq!(p.account, "286969-2");
        assert!(p.holder.contains("Paulo"));
        // 4 real entries (saldo diário + blank skipped)
        assert_eq!(p.entries.len(), 4);
        assert_eq!(p.entries[0].month, "2026-01");
        assert_eq!(p.entries[1].description, "Ln Sports");
    }

    #[test]
    fn classifies_and_excludes() {
        let p = parse_statement_rows(&rows());
        let cz = Categorizer::with_defaults();
        let hk = holder_key(&p.holder);
        let cl: Vec<_> = p.entries.iter().map(|e| classify_entry(e, &p.account, &hk, true, &cz)).collect();

        // internal transfer (desc = holder, no accents) → excluded
        assert!(!cl[0].included && cl[0].reason == "interno");
        // Ln Sports debit → included, expense, BTG fallback category
        assert!(cl[1].included && cl[1].kind == "expense");
        assert_eq!(cl[1].category, "Cuidados Pessoais");
        // salary with payslip → excluded
        assert!(!cl[2].included && cl[2].reason == "salario");
        // card bill → excluded
        assert!(!cl[3].included && cl[3].reason == "fatura");
    }

    #[test]
    fn salary_kept_when_no_payslip() {
        let p = parse_statement_rows(&rows());
        let cz = Categorizer::with_defaults();
        let hk = holder_key(&p.holder);
        let salary = classify_entry(&p.entries[2], &p.account, &hk, false, &cz);
        assert!(salary.included && salary.kind == "income");
    }

    #[test]
    fn dedup_id_is_stable() {
        let p = parse_statement_rows(&rows());
        let a = entry_id(&p.account, &p.entries[1]);
        let b = entry_id(&p.account, &p.entries[1]);
        assert_eq!(a, b);
        assert_ne!(a, entry_id(&p.account, &p.entries[2]));
    }

    // T009 (016) — the last "Saldo Diário" line becomes a best-effort position.
    #[test]
    fn btg_last_daily_balance_becomes_a_position() {
        use crate::domain::account_position::Product;
        let p = parse_statement_rows(&rows());
        assert_eq!(p.positions.len(), 1);
        assert_eq!(p.positions[0].product, Product::Corrente);
        assert_eq!(p.positions[0].balance, Decimal::from_str("5160").unwrap());
        assert_eq!(
            p.positions[0].as_of,
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert!(p.coverage.is_none(), "grid BTG não imprime período — cobertura nunca");

        // A grid without the line yields no position (nothing invented).
        let no_daily: Vec<Vec<String>> =
            rows().into_iter().filter(|r| !r.iter().any(|c| norm(c) == "SALDO DIARIO")).collect();
        assert!(parse_statement_rows(&no_daily).positions.is_empty());
    }

    /// Regression lock: the id key must stay byte-identical, or every bank entry
    /// already in the user's database re-imports as a new row.
    #[test]
    fn entry_id_key_format_is_frozen() {
        let p = parse_statement_rows(&rows());
        let e = &p.entries[1]; // Ln Sports, -340, 01/01/2026
        let legacy = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            b"bank:286969-2:2026-01-01:LN SPORTS:-340",
        )
        .to_string();
        assert_eq!(entry_id(&p.account, e), legacy);
        // …and the first occurrence in a batch keeps that same id.
        assert_eq!(entry_ids(&p.account, &p.entries)[1], legacy);
    }

    #[test]
    fn identical_lines_get_distinct_ids() {
        let e = RawEntry {
            date: "2026-03-04".into(),
            month: "2026-03".into(),
            btg_category: String::new(),
            transaction: "Pix enviado".into(),
            description: "Padaria Central".into(),
            amount: Decimal::from_str("-25.50").unwrap(),
        };
        let ids = entry_ids("286969-2", &[e.clone(), e.clone(), e.clone()]);
        assert_eq!(ids[0], entry_id("286969-2", &e), "first keeps the legacy id");
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
    }

    #[test]
    fn card_bill_is_excluded_in_both_bank_wordings() {
        let cz = Categorizer::with_defaults();
        let mk = |transaction: &str| RawEntry {
            date: "2026-03-04".into(),
            month: "2026-03".into(),
            btg_category: String::new(),
            transaction: transaction.into(),
            description: "Cartao".into(),
            amount: Decimal::from_str("-100").unwrap(),
        };
        for t in ["Pagamento de fatura do cartão", "Pagamento Fatura Cartao"] {
            let c = classify_entry(&mk(t), "1234567-8", "", false, &cz);
            assert!(!c.included && c.reason == "fatura", "não excluiu: {t}");
        }
    }
}
