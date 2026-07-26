//! Infrastructure: SQLite persistence (rusqlite) — the app's source of truth. Owns
//! the schema and migrations, plus load/persist for invoices, config, manual entries,
//! payslips, bank entries and recurring categories.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use chrono::{Local, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::bank_statement::BankEntry;
use crate::domain::invoice::{Invoice, YearMonth};
use crate::domain::manual_entry::{EntryKind, ManualEntry};
use crate::domain::payslip::{Payslip, PayslipItem};
use crate::domain::recurring::RecurringCategory;
use crate::domain::transaction::{InstallmentInfo, Transaction};
use crate::domain::categorizer::Categorizer;
use crate::domain::{AppConfig, CategoryRule};

/// Parse a money value stored as text. The app always writes valid `Decimal`
/// strings, so a failure means the row was corrupted or hand-edited — log it
/// loudly instead of silently reading it as 0 (which would hide data loss).
fn parse_money(field: &str, s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|e| {
        eprintln!("[financas] valor decimal inválido em {field}: {s:?} ({e}); usando 0");
        Decimal::ZERO
    })
}

/// SQLite-backed persistence for invoices and their transactions.
/// Config (rules, overrides, manual entries) stays in config.json.
pub struct Database {
    conn: Connection,
    /// Absolute path of the SQLite file backing `conn`. Empty for in-memory DBs (tests).
    /// Backup/restore need it to know the source/target file without an `AppHandle`.
    path: PathBuf,
}

pub type SharedDb = Arc<Mutex<Database>>;

const DATE_FMT: &str = "%Y-%m-%d";
const DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self { conn, path: path.to_path_buf() };
        db.init()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self { conn, path: PathBuf::new() };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS invoices (
                    id             TEXT PRIMARY KEY,
                    bank           TEXT NOT NULL DEFAULT 'BTG',
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
                CREATE TABLE IF NOT EXISTS inflation_cache (
                    id         INTEGER PRIMARY KEY CHECK (id = 1),
                    payload    TEXT NOT NULL,
                    fetched_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS bank_entries (
                    id           TEXT PRIMARY KEY,
                    bank         TEXT NOT NULL,
                    account      TEXT NOT NULL,
                    date         TEXT NOT NULL,
                    month        TEXT NOT NULL,
                    description  TEXT NOT NULL,
                    category     TEXT NOT NULL,
                    amount       TEXT NOT NULL,
                    kind         TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS categories (
                    name TEXT PRIMARY KEY
                );
                CREATE TABLE IF NOT EXISTS recurring_categories (
                    category    TEXT PRIMARY KEY,
                    start_month TEXT,
                    end_month   TEXT
                );
                CREATE TABLE IF NOT EXISTS dismissed_recurring_suggestions (
                    target TEXT PRIMARY KEY
                );
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
        // Migrate recurring_categories created before base_amount existed.
        if let Err(e) = self
            .conn
            .execute("ALTER TABLE recurring_categories ADD COLUMN base_amount TEXT", [])
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(msg);
            }
        }
        // Migrate bank_entries created before user_categorized existed. When true, the
        // user set the category by hand → keyword recategorization must not overwrite it.
        if let Err(e) = self
            .conn
            .execute("ALTER TABLE bank_entries ADD COLUMN user_categorized INTEGER NOT NULL DEFAULT 0", [])
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(msg);
            }
        }
        // Migrate invoices created before multi-bank support (feature 014): the table
        // is bank-generic; every pre-existing invoice was a BTG one.
        if let Err(e) = self
            .conn
            .execute("ALTER TABLE invoices ADD COLUMN bank TEXT NOT NULL DEFAULT 'BTG'", [])
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
        tx.execute("DELETE FROM categories", []).map_err(|e| e.to_string())?;

        // Persist every category name (even ones with no keywords yet) so a freshly
        // created category survives a reload.
        for rule in &cfg.category_rules {
            tx.execute(
                "INSERT OR IGNORE INTO categories (name) VALUES (?1)",
                params![rule.category],
            )
            .map_err(|e| e.to_string())?;
        }

        if let Some(dir) = cfg.import_directory.as_deref().filter(|s| !s.is_empty()) {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES ('import_directory', ?1)",
                params![dir],
            )
            .map_err(|e| e.to_string())?;
        }

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
        let import_directory: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'import_directory'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .filter(|s| !s.is_empty());

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
        let mut grouped: HashMap<String, (u32, Vec<String>)> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for r in rows {
            let (category, keyword, priority) = r.map_err(|e| e.to_string())?;
            let entry = grouped.entry(category.clone()).or_insert_with(|| {
                order.push(category.clone());
                (priority as u32, Vec::new())
            });
            entry.1.push(keyword);
        }
        let mut category_rules: Vec<CategoryRule> = order
            .into_iter()
            .map(|cat| {
                let (priority, keywords) = grouped.remove(&cat).unwrap();
                CategoryRule { keywords, category: cat, priority }
            })
            .collect();

        // Categories with no keywords yet (persisted separately) → keep them as empty rules
        // so a freshly created category doesn't vanish on reload.
        let mut have: std::collections::HashSet<String> =
            category_rules.iter().map(|r| r.category.clone()).collect();
        let mut next_priority = category_rules.iter().map(|r| r.priority).max().unwrap_or(0);
        let mut cstmt = self.conn.prepare("SELECT name FROM categories ORDER BY name").map_err(|e| e.to_string())?;
        let cat_rows = cstmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        for r in cat_rows {
            let name = r.map_err(|e| e.to_string())?;
            if have.insert(name.clone()) {
                next_priority += 10;
                category_rules.push(CategoryRule { keywords: Vec::new(), category: name, priority: next_priority });
            }
        }

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
                amount: parse_money("manual_entries.amount", &amount),
                category,
                month,
                recurring: recurring != 0,
                is_salary: is_salary != 0,
            });
        }

        Ok(AppConfig {
            category_rules,
            transaction_overrides,
            manual_entries,
            import_directory,
        })
    }

    /// Replace all persisted invoices/transactions with the given snapshot.
    pub fn save_all(&mut self, invoices: &[Invoice]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM transactions", []).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM invoices", []).map_err(|e| e.to_string())?;

        for inv in invoices {
            tx.execute(
                "INSERT INTO invoices (id, bank, filename, reference_year, reference_month, due_date, imported_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    inv.id.to_string(),
                    inv.bank,
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
            .prepare("SELECT id, bank, filename, reference_year, reference_month, due_date, imported_at FROM invoices")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let bank: String = row.get(1)?;
                let filename: String = row.get(2)?;
                let year: i32 = row.get(3)?;
                let month: i64 = row.get(4)?;
                let due: Option<String> = row.get(5)?;
                let imported: String = row.get(6)?;
                Ok((id, bank, filename, year, month, due, imported))
            })
            .map_err(|e| e.to_string())?;

        let mut invoices = Vec::new();
        for r in rows {
            let (id, bank, filename, year, month, due, imported) = r.map_err(|e| e.to_string())?;
            let invoice_id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
            let transactions = self.load_transactions(&id)?;
            invoices.push(Invoice {
                id: invoice_id,
                bank,
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
                amount: parse_money("transactions.amount", &amount),
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
        let dnum = |s: &str| parse_money("payslips", s);
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
                amount: parse_money("payslip_items.amount", &amount),
                net_share: parse_money("payslip_items.net_share", &net_share),
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

    /// Upsert the single-row inflation index cache (JSON payload + fetch timestamp).
    pub fn save_inflation_cache(&self, payload: &str, fetched_at: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO inflation_cache (id, payload, fetched_at) VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET payload = ?1, fetched_at = ?2",
                params![payload, fetched_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load the cached inflation payload (JSON), or None if never fetched.
    pub fn load_inflation_cache(&self) -> Result<Option<String>, String> {
        self.conn
            .query_row("SELECT payload FROM inflation_cache WHERE id = 1", [], |r| r.get::<_, String>(0))
            .optional()
            .map_err(|e| e.to_string())
    }

    /// Upsert bank-statement entries (dedup by deterministic id → no re-import dupes).
    pub fn save_bank_entries(&mut self, entries: &[BankEntry]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for e in entries {
            tx.execute(
                "INSERT INTO bank_entries (id, bank, account, date, month, description, category, amount, kind)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
                 ON CONFLICT(id) DO UPDATE SET category = ?7, amount = ?8, kind = ?9",
                params![e.id, e.bank, e.account, e.date, e.month, e.description, e.category, e.amount.to_string(), e.kind],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn load_bank_entries(&self) -> Result<Vec<BankEntry>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, bank, account, date, month, description, category, amount, kind FROM bank_entries ORDER BY date")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(BankEntry {
                    id: r.get(0)?,
                    bank: r.get(1)?,
                    account: r.get(2)?,
                    date: r.get(3)?,
                    month: r.get(4)?,
                    description: r.get(5)?,
                    category: r.get(6)?,
                    amount: parse_money("bank_entries.amount", &r.get::<_, String>(7)?),
                    kind: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn update_bank_entry_category(&mut self, id: &str, category: &str) -> Result<(), String> {
        // A hand-set category is a per-entry override: mark it so keyword recategorization
        // leaves it alone.
        self.conn
            .execute(
                "UPDATE bank_entries SET category = ?1, user_categorized = 1 WHERE id = ?2",
                params![category, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Re-run keyword rules over bank statement entries (both credit and debit), so a
    /// keyword categorizes card AND extrato uniformly. A keyword match wins; when no
    /// keyword matches, the existing category (BTG fallback / prior) is kept. Entries the
    /// user categorized by hand (`user_categorized = 1`) are never touched. Returns the
    /// number of entries whose category changed.
    pub fn recategorize_bank_entries(&mut self, categorizer: &Categorizer) -> Result<usize, String> {
        let rows: Vec<(String, String, String)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, description, category FROM bank_entries WHERE user_categorized = 0")
                .map_err(|e| e.to_string())?;
            let mapped = stmt
                .query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
                })
                .map_err(|e| e.to_string())?;
            let mut v = Vec::new();
            for r in mapped {
                v.push(r.map_err(|e| e.to_string())?);
            }
            v
        };
        let mut changed = 0usize;
        for (id, description, category) in rows {
            let matched = categorizer.categorize(&description);
            if matched != "Outros" && matched != category {
                self.conn
                    .execute("UPDATE bank_entries SET category = ?1 WHERE id = ?2", params![matched, id])
                    .map_err(|e| e.to_string())?;
                changed += 1;
            }
        }
        Ok(changed)
    }

    pub fn remove_bank_entry(&mut self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM bank_entries WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_bank_entries(&mut self) -> Result<(), String> {
        self.conn.execute("DELETE FROM bank_entries", []).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Recurring categories + dismissed suggestions (feature 010) ──

    pub fn load_recurring_categories(&self) -> Result<Vec<RecurringCategory>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT category, start_month, end_month, base_amount FROM recurring_categories ORDER BY category")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(RecurringCategory {
                    category: r.get(0)?,
                    start_month: r.get(1)?,
                    end_month: r.get(2)?,
                    base_amount: r
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| Decimal::from_str(&s).ok()),
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// Upsert (recurring=true) or remove (recurring=false) a category's recurrence.
    /// `start_month`/`end_month` are "YYYY-MM" or None (open-ended vigência).
    pub fn set_recurring_category(
        &mut self,
        category: &str,
        recurring: bool,
        start_month: Option<&str>,
        end_month: Option<&str>,
    ) -> Result<(), String> {
        if recurring {
            self.conn
                .execute(
                    "INSERT INTO recurring_categories (category, start_month, end_month) VALUES (?1, ?2, ?3)
                     ON CONFLICT(category) DO UPDATE SET start_month = ?2, end_month = ?3",
                    params![category, start_month, end_month],
                )
                .map_err(|e| e.to_string())?;
        } else {
            self.conn
                .execute("DELETE FROM recurring_categories WHERE category = ?1", params![category])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Set (or clear, with None) the user's base value for a recurring category.
    pub fn set_recurring_base(&mut self, category: &str, amount: Option<&str>) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE recurring_categories SET base_amount = ?1 WHERE category = ?2",
                params![amount, category],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Distinct category names in use anywhere: config rules, card transactions,
    /// bank entries, manual entries and payslip deduction categories.
    pub fn all_category_names(&self) -> Result<Vec<String>, String> {
        let sql = "
            SELECT name AS category FROM categories
            UNION SELECT category FROM category_rules
            UNION SELECT category FROM transactions
            UNION SELECT category FROM bank_entries
            UNION SELECT category FROM manual_entries
            ORDER BY category";
        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            let c: String = r.map_err(|e| e.to_string())?;
            if !c.trim().is_empty() {
                out.push(c);
            }
        }
        Ok(out)
    }

    pub fn load_dismissed_suggestions(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT target FROM dismissed_recurring_suggestions")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn dismiss_suggestion(&mut self, target: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO dismissed_recurring_suggestions (target) VALUES (?1)",
                params![target],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Backup & restore (feature 012) ──

    /// Write a consistent snapshot of the live database to `dest` via `VACUUM INTO`.
    /// Safe with the connection open — unlike a raw file copy, it never captures a
    /// half-written page. `dest` must not already exist (VACUUM INTO refuses to overwrite).
    fn vacuum_into(&self, dest: &Path) -> Result<(), String> {
        self.conn
            .execute("VACUUM INTO ?1", params![dest.to_string_lossy()])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Back up the whole database into `dest_dir` as
    /// `financas-backup-<YYYYMMDD-HHMMSS>.db` (suffixed `-N` if that name is taken,
    /// so successive backups never overwrite each other). Returns the file written.
    pub fn backup_to(&self, dest_dir: &Path) -> Result<PathBuf, String> {
        if !dest_dir.is_dir() {
            return Err("BACKUP_DIR_INVALID".to_string());
        }
        let stem = format!("financas-backup-{}", Local::now().format("%Y%m%d-%H%M%S"));
        let dest = unique_path(dest_dir, &stem);
        self.vacuum_into(&dest).map_err(|e| format!("BACKUP_FAILED: {e}"))?;
        Ok(dest)
    }

    /// Validate that `path` is a restorable app database: passes SQLite integrity check
    /// and contains the app's core tables. Never mutates anything. Associated fn so the
    /// caller can validate a candidate before touching the live DB.
    pub fn validate_backup(path: &Path) -> Result<(), String> {
        if !path.is_file() {
            return Err("FILE_NOT_FOUND".to_string());
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| "INVALID_BACKUP".to_string())?;
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|_| "INVALID_BACKUP".to_string())?;
        if integrity != "ok" {
            return Err("INVALID_BACKUP".to_string());
        }
        for table in ["invoices", "transactions", "settings"] {
            let found: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |r| r.get(0),
                )
                .map_err(|_| "INVALID_BACKUP".to_string())?;
            if found == 0 {
                return Err("INVALID_BACKUP".to_string());
            }
        }
        Ok(())
    }

    /// Replace the live database with the one at `src`, after validating it and saving a
    /// `financas-pre-restore-<ts>.db` copy of the current data next to it (so the user can
    /// revert). Returns the path of that safety copy. On validation failure the live DB is
    /// left untouched. Reopens the connection on the restored file and re-runs migrations.
    pub fn restore_from(&mut self, src: &Path) -> Result<PathBuf, String> {
        Self::validate_backup(src)?;

        // Safety copy of the CURRENT database before we overwrite it.
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let stem = format!("financas-pre-restore-{}", Local::now().format("%Y%m%d-%H%M%S"));
        let safety = unique_path(dir, &stem);
        self.vacuum_into(&safety).map_err(|e| format!("RESTORE_FAILED: {e}"))?;

        // Close the current connection (drop it) so the file can be overwritten (Windows).
        let tmp = Connection::open_in_memory().map_err(|e| format!("RESTORE_FAILED: {e}"))?;
        let old = std::mem::replace(&mut self.conn, tmp);
        drop(old);

        std::fs::copy(src, &self.path).map_err(|e| format!("RESTORE_FAILED: {e}"))?;

        // Reopen on the restored file; init() migrations bring an older schema up to date.
        self.conn = Connection::open(&self.path).map_err(|e| format!("RESTORE_FAILED: {e}"))?;
        self.init().map_err(|e| format!("RESTORE_FAILED: {e}"))?;
        Ok(safety)
    }
}

/// First free path of the form `<dir>/<stem>.db`, then `<stem>-1.db`, `<stem>-2.db`, …
/// Avoids clobbering an existing backup (e.g. two backups in the same second).
fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{stem}.db"));
    let mut n = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}-{n}.db"));
        n += 1;
    }
    candidate
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

    /// The invoices table is bank-generic: whatever bank the reader strategy stamps
    /// must survive the roundtrip untouched (no BTG hardcoded in persistence).
    #[test]
    fn invoice_bank_roundtrips_for_any_bank() {
        let mut db = Database::open_in_memory().unwrap();
        let mut inv = make_invoice();
        assert_eq!(inv.bank, "BTG", "default histórico");
        inv.bank = "NovoBanco".to_string();
        db.save_all(std::slice::from_ref(&inv)).unwrap();
        assert_eq!(db.load_invoices().unwrap()[0].bank, "NovoBanco");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        let inv = make_invoice();
        db.save_all(std::slice::from_ref(&inv)).unwrap();

        let loaded = db.load_invoices().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, inv.id);
        assert_eq!(loaded[0].bank, "BTG");
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
            import_directory: None,
        };
        db.save_config(&cfg).unwrap();
        assert!(!db.config_is_empty());

        let loaded = db.load_config().unwrap();
        assert_eq!(loaded.category_rules.len(), 1);
        assert_eq!(loaded.category_rules[0].category, "Alimentação");
        assert_eq!(loaded.category_rules[0].keywords.len(), 2);
        assert_eq!(loaded.transaction_overrides.get("abc").map(String::as_str), Some("Cerveja"));
        assert_eq!(loaded.manual_entries.len(), 1);
        assert_eq!(loaded.manual_entries[0].kind, EntryKind::Income);
        assert_eq!(loaded.manual_entries[0].amount, dec!(8000));
    }

    // ── Backup & restore (feature 012) ──

    #[test]
    fn backup_to_creates_valid_copy() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(&dir.path().join("financas.db")).unwrap();
        db.save_all(&[make_invoice()]).unwrap();

        let dest = dir.path().join("backups");
        std::fs::create_dir_all(&dest).unwrap();
        let backup = db.backup_to(&dest).unwrap();

        assert!(backup.exists());
        assert!(backup.starts_with(&dest));
        // The copy opens as a database and holds the same data.
        let restored = Database::open(&backup).unwrap();
        let invs = restored.load_invoices().unwrap();
        assert_eq!(invs.len(), 1);
        assert_eq!(invs[0].transactions.len(), 1);
    }

    #[test]
    fn backup_to_rejects_non_directory() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.backup_to(Path::new("/no/such/dir")), Err("BACKUP_DIR_INVALID".into()));
    }

    #[test]
    fn backup_to_never_overwrites_previous() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("financas.db")).unwrap();
        let b1 = db.backup_to(dir.path()).unwrap();
        let b2 = db.backup_to(dir.path()).unwrap();
        assert_ne!(b1, b2);
        assert!(b1.exists() && b2.exists());
    }

    #[test]
    fn validate_backup_accepts_app_db_and_rejects_others() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("financas.db");
        drop(Database::open(&db_path).unwrap()); // create app schema
        assert!(Database::validate_backup(&db_path).is_ok());

        let missing = dir.path().join("nope.db");
        assert_eq!(Database::validate_backup(&missing), Err("FILE_NOT_FOUND".into()));

        let junk = dir.path().join("junk.db");
        std::fs::write(&junk, b"this is not a sqlite database").unwrap();
        assert_eq!(Database::validate_backup(&junk), Err("INVALID_BACKUP".into()));

        let empty = dir.path().join("empty.db");
        {
            let c = Connection::open(&empty).unwrap();
            c.execute("CREATE TABLE foo (x)", []).unwrap();
        }
        assert_eq!(Database::validate_backup(&empty), Err("INVALID_BACKUP".into()));
    }

    #[test]
    fn restore_from_swaps_db_and_preserves_previous() {
        let dir = tempfile::tempdir().unwrap();
        let cur_path = dir.path().join("financas.db");
        let mut cur = Database::open(&cur_path).unwrap();
        cur.save_all(&[make_invoice()]).unwrap(); // current state: 1 invoice

        // Source backup with a distinct (empty) state.
        let src_path = dir.path().join("source.db");
        drop(Database::open(&src_path).unwrap());

        assert_eq!(cur.load_invoices().unwrap().len(), 1);
        let safety = cur.restore_from(&src_path).unwrap();

        // Live DB now reflects the source (0 invoices).
        assert!(safety.exists());
        assert_eq!(cur.load_invoices().unwrap().len(), 0);
        // The safety copy still holds the previous state (1 invoice) → revertible.
        let prev = Database::open(&safety).unwrap();
        assert_eq!(prev.load_invoices().unwrap().len(), 1);
    }

    #[test]
    fn restore_from_rejects_invalid_source_without_touching_db() {
        let dir = tempfile::tempdir().unwrap();
        let cur_path = dir.path().join("financas.db");
        let mut cur = Database::open(&cur_path).unwrap();
        cur.save_all(&[make_invoice()]).unwrap();

        let junk = dir.path().join("junk.db");
        std::fs::write(&junk, b"garbage").unwrap();
        assert_eq!(cur.restore_from(&junk), Err("INVALID_BACKUP".into()));
        // Live DB untouched.
        assert_eq!(cur.load_invoices().unwrap().len(), 1);
    }
}
