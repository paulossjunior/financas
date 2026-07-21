use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::invoice::{Invoice, YearMonth};
use crate::domain::manual_entry::{EntryKind, ManualEntry};
use crate::domain::payslip::{Payslip, PayslipItem};
use crate::domain::transaction::{InstallmentInfo, Transaction};
use crate::domain::{AppConfig, CategoryRule};

/// SQLite-backed persistence for invoices and their transactions.
/// Config (rules, overrides, manual entries) stays in config.json.
pub struct Database {
    conn: Connection,
}

pub type SharedDb = Arc<Mutex<Database>>;

const DATE_FMT: &str = "%Y-%m-%d";
const DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS invoices (
                    id             TEXT PRIMARY KEY,
                    filename       TEXT NOT NULL,
                    reference_year INTEGER NOT NULL,
                    reference_month INTEGER NOT NULL,
                    due_date       TEXT,
                    imported_at    TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS transactions (
                    id                 TEXT PRIMARY KEY,
                    invoice_id         TEXT NOT NULL,
                    date               TEXT NOT NULL,
                    description        TEXT NOT NULL,
                    amount             TEXT NOT NULL,
                    category           TEXT NOT NULL,
                    installment_current INTEGER,
                    installment_total   INTEGER,
                    is_reversal        INTEGER NOT NULL,
                    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_tx_invoice ON transactions(invoice_id);

                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS category_rules (
                    category TEXT NOT NULL,
                    keyword  TEXT NOT NULL,
                    priority INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS transaction_overrides (
                    transaction_id TEXT PRIMARY KEY,
                    category       TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS manual_entries (
                    id          TEXT PRIMARY KEY,
                    kind        TEXT NOT NULL,
                    description TEXT NOT NULL,
                    amount      TEXT NOT NULL,
                    category    TEXT NOT NULL,
                    month       TEXT NOT NULL,
                    recurring   INTEGER NOT NULL,
                    is_salary   INTEGER NOT NULL DEFAULT 1
                );
                CREATE TABLE IF NOT EXISTS payslips (
                    id          TEXT PRIMARY KEY,
                    month       TEXT NOT NULL UNIQUE,
                    gross       TEXT NOT NULL,
                    real_gross  TEXT NOT NULL,
                    deductions  TEXT NOT NULL,
                    net         TEXT NOT NULL,
                    salary_liq  TEXT NOT NULL,
                    bonus_liq   TEXT NOT NULL,
                    ir_base     TEXT NOT NULL,
                    fgts        TEXT NOT NULL,
                    source_file TEXT NOT NULL,
                    imported_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS payslip_items (
                    payslip_id  TEXT NOT NULL,
                    kind        TEXT NOT NULL,
                    class       TEXT NOT NULL,
                    description TEXT NOT NULL,
                    amount      TEXT NOT NULL,
                    net_share   TEXT NOT NULL,
                    offsetting  INTEGER NOT NULL,
                    FOREIGN KEY (payslip_id) REFERENCES payslips(id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_pitems_payslip ON payslip_items(payslip_id);
                ",
            )
            .map_err(|e| e.to_string())?;

        // Migrate pre-existing databases created before is_salary existed.
        // ADD COLUMN errors with "duplicate column name" when already present — ignore that.
        if let Err(e) = self
            .conn
            .execute("ALTER TABLE manual_entries ADD COLUMN is_salary INTEGER NOT NULL DEFAULT 1", [])
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(msg);
            }
        }
        // Salary now comes from the payslip; manual income is always EXTRA (bolsa, rendimentos).
        // Normalize any legacy salary-flagged income so it is never superseded by a payslip.
        self.conn
            .execute("UPDATE manual_entries SET is_salary = 0 WHERE kind = 'income'", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// True when no config data is stored yet (fresh DB — triggers config.json migration).
    pub fn config_is_empty(&self) -> bool {
        let rules: i64 = self
            .conn
            .query_row("SELECT count(*) FROM category_rules", [], |r| r.get(0))
            .unwrap_or(0);
        let entries: i64 = self
            .conn
            .query_row("SELECT count(*) FROM manual_entries", [], |r| r.get(0))
            .unwrap_or(0);
        let overrides: i64 = self
            .conn
            .query_row("SELECT count(*) FROM transaction_overrides", [], |r| r.get(0))
            .unwrap_or(0);
        let settings: i64 = self
            .conn
            .query_row("SELECT count(*) FROM settings", [], |r| r.get(0))
            .unwrap_or(0);
        rules == 0 && entries == 0 && overrides == 0 && settings == 0
    }

    /// Replace all persisted config (rules, overrides, manual entries, settings).
    pub fn save_config(&mut self, cfg: &AppConfig) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM category_rules", []).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM transaction_overrides", []).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM manual_entries", []).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM settings", []).map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('faturas_directory', ?1)",
            params![cfg.faturas_directory],
        )
        .map_err(|e| e.to_string())?;

        for rule in &cfg.category_rules {
            for kw in &rule.keywords {
                tx.execute(
                    "INSERT INTO category_rules (category, keyword, priority) VALUES (?1, ?2, ?3)",
                    params![rule.category, kw, rule.priority as i64],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        for (tx_id, category) in &cfg.transaction_overrides {
            tx.execute(
                "INSERT INTO transaction_overrides (transaction_id, category) VALUES (?1, ?2)",
                params![tx_id, category],
            )
            .map_err(|e| e.to_string())?;
        }

        for e in &cfg.manual_entries {
            tx.execute(
                "INSERT INTO manual_entries (id, kind, description, amount, category, month, recurring, is_salary)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    e.id.to_string(),
                    entry_kind_str(e.kind),
                    e.description,
                    e.amount.to_string(),
                    e.category,
                    e.month,
                    e.recurring as i64,
                    e.is_salary as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())
    }

    /// Load full config from the database.
    pub fn load_config(&self) -> Result<AppConfig, String> {
        let faturas_directory: String = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'faturas_directory'",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "faturas".to_string());

        // category_rules grouped by category
        let mut stmt = self
            .conn
            .prepare("SELECT category, keyword, priority FROM category_rules")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut grouped: HashMap<String, (u8, Vec<String>)> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for r in rows {
            let (category, keyword, priority) = r.map_err(|e| e.to_string())?;
            let entry = grouped.entry(category.clone()).or_insert_with(|| {
                order.push(category.clone());
                (priority as u8, Vec::new())
            });
            entry.1.push(keyword);
        }
        let category_rules: Vec<CategoryRule> = order
            .into_iter()
            .map(|cat| {
                let (priority, keywords) = grouped.remove(&cat).unwrap();
                CategoryRule { keywords, category: cat, priority }
            })
            .collect();

        // transaction_overrides
        let mut stmt = self
            .conn
            .prepare("SELECT transaction_id, category FROM transaction_overrides")
            .map_err(|e| e.to_string())?;
        let ov_rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut transaction_overrides = HashMap::new();
        for r in ov_rows {
            let (k, v) = r.map_err(|e| e.to_string())?;
            transaction_overrides.insert(k, v);
        }

        // manual_entries
        let mut stmt = self
            .conn
            .prepare("SELECT id, kind, description, amount, category, month, recurring, is_salary FROM manual_entries")
            .map_err(|e| e.to_string())?;
        let me_rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut manual_entries = Vec::new();
        for r in me_rows {
            let (id, kind, description, amount, category, month, recurring, is_salary) =
                r.map_err(|e| e.to_string())?;
            manual_entries.push(ManualEntry {
                id: Uuid::parse_str(&id).map_err(|e| e.to_string())?,
                kind: entry_kind_from_str(&kind),
                description,
                amount: Decimal::from_str(&amount).unwrap_or_default(),
                category,
                month,
                recurring: recurring != 0,
                is_salary: is_salary != 0,
            });
        }

        Ok(AppConfig {
            faturas_directory,
            category_rules,
            transaction_overrides,
            manual_entries,
        })
    }

    /// Replace all persisted invoices/transactions with the given snapshot.
    pub fn save_all(&mut self, invoices: &[Invoice]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM transactions", []).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM invoices", []).map_err(|e| e.to_string())?;

        for inv in invoices {
            tx.execute(
                "INSERT INTO invoices (id, filename, reference_year, reference_month, due_date, imported_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    inv.id.to_string(),
                    inv.filename,
                    inv.reference_month.year,
                    inv.reference_month.month as i64,
                    inv.due_date.map(|d| d.format(DATE_FMT).to_string()),
                    inv.imported_at.format(DATETIME_FMT).to_string(),
                ],
            )
            .map_err(|e| e.to_string())?;

            for t in &inv.transactions {
                tx.execute(
                    "INSERT INTO transactions
                       (id, invoice_id, date, description, amount, category,
                        installment_current, installment_total, is_reversal)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        t.id.to_string(),
                        t.invoice_id.to_string(),
                        t.date.format(DATE_FMT).to_string(),
                        t.description,
                        t.amount.to_string(),
                        t.category,
                        t.installment.as_ref().map(|i| i.current as i64),
                        t.installment.as_ref().map(|i| i.total as i64),
                        t.is_reversal as i64,
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        tx.commit().map_err(|e| e.to_string())
    }

    /// Load all invoices (with transactions) from the database.
    pub fn load_invoices(&self) -> Result<Vec<Invoice>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, filename, reference_year, reference_month, due_date, imported_at FROM invoices")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let filename: String = row.get(1)?;
                let year: i32 = row.get(2)?;
                let month: i64 = row.get(3)?;
                let due: Option<String> = row.get(4)?;
                let imported: String = row.get(5)?;
                Ok((id, filename, year, month, due, imported))
            })
            .map_err(|e| e.to_string())?;

        let mut invoices = Vec::new();
        for r in rows {
            let (id, filename, year, month, due, imported) = r.map_err(|e| e.to_string())?;
            let invoice_id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
            let transactions = self.load_transactions(&id)?;
            invoices.push(Invoice {
                id: invoice_id,
                filename,
                reference_month: YearMonth::new(year, month as u8),
                due_date: due.and_then(|s| NaiveDate::parse_from_str(&s, DATE_FMT).ok()),
                transactions,
                imported_at: NaiveDateTime::parse_from_str(&imported, DATETIME_FMT)
                    .unwrap_or_else(|_| default_datetime()),
            });
        }
        Ok(invoices)
    }

    fn load_transactions(&self, invoice_id: &str) -> Result<Vec<Transaction>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, invoice_id, date, description, amount, category,
                        installment_current, installment_total, is_reversal
                 FROM transactions WHERE invoice_id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![invoice_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut txs = Vec::new();
        for r in rows {
            let (id, inv, date, desc, amount, category, cur, tot, rev) =
                r.map_err(|e| e.to_string())?;
            let installment = match (cur, tot) {
                (Some(c), Some(t)) => Some(InstallmentInfo {
                    current: c as u8,
                    total: t as u8,
                }),
                _ => None,
            };
            txs.push(Transaction {
                id: Uuid::parse_str(&id).map_err(|e| e.to_string())?,
                invoice_id: Uuid::parse_str(&inv).map_err(|e| e.to_string())?,
                date: NaiveDate::parse_from_str(&date, DATE_FMT).map_err(|e| e.to_string())?,
                description: desc,
                amount: Decimal::from_str(&amount).unwrap_or_default(),
                category,
                installment,
                is_reversal: rev != 0,
            });
        }
        Ok(txs)
    }

    /// Insert or replace a payslip (keyed by its deterministic id / month) and its items.
    pub fn save_payslip(&mut self, p: &Payslip) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let id = p.id.to_string();
        tx.execute("DELETE FROM payslip_items WHERE payslip_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        // Same month may have a different id only in theory; clear by month too.
        tx.execute("DELETE FROM payslips WHERE id = ?1 OR month = ?2", params![id, p.month])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO payslips (id, month, gross, real_gross, deductions, net, salary_liq, bonus_liq, ir_base, fgts, source_file, imported_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                id, p.month, p.gross.to_string(), p.real_gross.to_string(), p.deductions.to_string(),
                p.net.to_string(), p.salary_liq.to_string(), p.bonus_liq.to_string(),
                p.ir_base.to_string(), p.fgts.to_string(), p.source_file, p.imported_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        for it in &p.items {
            tx.execute(
                "INSERT INTO payslip_items (payslip_id, kind, class, description, amount, net_share, offsetting)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![id, it.kind, it.class, it.description, it.amount.to_string(), it.net_share.to_string(), it.offsetting as i64],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// Load all payslips (with items), most recent month first.
    pub fn load_payslips(&self) -> Result<Vec<Payslip>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, month, gross, real_gross, deductions, net, salary_liq, bonus_liq, ir_base, fgts, source_file, imported_at FROM payslips ORDER BY month DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?, r.get::<_, String>(10)?, r.get::<_, String>(11)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let dnum = |s: &str| Decimal::from_str(s).unwrap_or_default();
        let mut out = Vec::new();
        for row in rows {
            let (id, month, gross, real_gross, deductions, net, salary_liq, bonus_liq, ir_base, fgts, source_file, imported_at) =
                row.map_err(|e| e.to_string())?;
            let items = self.load_payslip_items(&id)?;
            out.push(Payslip {
                id: Uuid::parse_str(&id).map_err(|e| e.to_string())?,
                month,
                gross: dnum(&gross), real_gross: dnum(&real_gross), deductions: dnum(&deductions),
                net: dnum(&net), salary_liq: dnum(&salary_liq), bonus_liq: dnum(&bonus_liq),
                ir_base: dnum(&ir_base), fgts: dnum(&fgts),
                items, source_file, imported_at,
            });
        }
        Ok(out)
    }

    fn load_payslip_items(&self, payslip_id: &str) -> Result<Vec<PayslipItem>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT kind, class, description, amount, net_share, offsetting FROM payslip_items WHERE payslip_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![payslip_id], |r| {
                Ok((
                    r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, i64>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for r in rows {
            let (kind, class, description, amount, net_share, offsetting) = r.map_err(|e| e.to_string())?;
            items.push(PayslipItem {
                kind, class, description,
                amount: Decimal::from_str(&amount).unwrap_or_default(),
                net_share: Decimal::from_str(&net_share).unwrap_or_default(),
                offsetting: offsetting != 0,
            });
        }
        Ok(items)
    }

    /// Remove a payslip by month.
    pub fn remove_payslip(&mut self, month: &str) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM payslip_items WHERE payslip_id IN (SELECT id FROM payslips WHERE month = ?1)",
            params![month],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM payslips WHERE month = ?1", params![month]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}

fn entry_kind_str(k: EntryKind) -> &'static str {
    match k {
        EntryKind::Income => "income",
        EntryKind::Expense => "expense",
    }
}

fn entry_kind_from_str(s: &str) -> EntryKind {
    match s {
        "income" => EntryKind::Income,
        _ => EntryKind::Expense,
    }
}

fn default_datetime() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

pub fn new_shared_db(db: Database) -> SharedDb {
    Arc::new(Mutex::new(db))
}

/// Persist an invoice snapshot to the database. On failure the error is logged
/// but not propagated — persistence must never break an otherwise-successful command.
pub fn persist(db: &SharedDb, invoices: &[Invoice]) {
    match db.lock() {
        Ok(mut guard) => {
            if let Err(e) = guard.save_all(invoices) {
                eprintln!("[db] falha ao persistir faturas: {e}");
            }
        }
        Err(e) => eprintln!("[db] mutex envenenado: {e}"),
    }
}

/// Persist config (rules, overrides, manual entries, settings) to the database.
pub fn persist_config(db: &SharedDb, cfg: &AppConfig) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.save_config(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_invoice() -> Invoice {
        let filename = "2026-06-fatura.xlsx";
        // Transactions must reference the invoice's own id (as import does).
        let invoice_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, filename.as_bytes());
        let txs = vec![Transaction::new(
            invoice_id,
            1,
            NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            "Ifood (2/3)".to_string(),
            dec!(42.90),
            "Alimentação".to_string(),
            Some(InstallmentInfo { current: 2, total: 3 }),
        )];
        Invoice::new(
            filename.to_string(),
            YearMonth::new(2026, 6),
            NaiveDate::from_ymd_opt(2026, 6, 15),
            txs,
            default_datetime(),
        )
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        let inv = make_invoice();
        db.save_all(&[inv.clone()]).unwrap();

        let loaded = db.load_invoices().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, inv.id);
        assert_eq!(loaded[0].filename, inv.filename);
        assert_eq!(loaded[0].transactions.len(), 1);
        let t = &loaded[0].transactions[0];
        assert_eq!(t.amount, dec!(42.90));
        assert_eq!(t.category, "Alimentação");
        assert_eq!(t.installment.as_ref().unwrap().current, 2);
        assert_eq!(t.installment.as_ref().unwrap().total, 3);
    }

    #[test]
    fn save_all_replaces_previous() {
        let mut db = Database::open_in_memory().unwrap();
        db.save_all(&[make_invoice()]).unwrap();
        db.save_all(&[]).unwrap();
        assert_eq!(db.load_invoices().unwrap().len(), 0);
    }

    #[test]
    fn config_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        assert!(db.config_is_empty());

        let cfg = AppConfig {
            faturas_directory: "faturas".into(),
            category_rules: vec![CategoryRule {
                keywords: vec!["IFOOD".into(), "RESTAURANTE".into()],
                category: "Alimentação".into(),
                priority: 10,
            }],
            transaction_overrides: HashMap::from([("abc".to_string(), "Cerveja".to_string())]),
            manual_entries: vec![ManualEntry::new(
                EntryKind::Income,
                "Salário".into(),
                dec!(8000),
                "Salário".into(),
                "2026-06".into(),
                true,
            )],
        };
        db.save_config(&cfg).unwrap();
        assert!(!db.config_is_empty());

        let loaded = db.load_config().unwrap();
        assert_eq!(loaded.faturas_directory, "faturas");
        assert_eq!(loaded.category_rules.len(), 1);
        assert_eq!(loaded.category_rules[0].category, "Alimentação");
        assert_eq!(loaded.category_rules[0].keywords.len(), 2);
        assert_eq!(loaded.transaction_overrides.get("abc").map(String::as_str), Some("Cerveja"));
        assert_eq!(loaded.manual_entries.len(), 1);
        assert_eq!(loaded.manual_entries[0].kind, EntryKind::Income);
        assert_eq!(loaded.manual_entries[0].amount, dec!(8000));
    }
}
