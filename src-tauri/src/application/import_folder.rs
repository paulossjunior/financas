//! Application use-case (feature 013): scan a single folder and auto-import every
//! recognizable BTG invoice (`.xlsx`) and bank statement (`.xls`/`.xlsx`) found in it.
//!
//! Type detection reuses the existing parsers: an `.xls` is treated as a statement
//! (invoices are always `.xlsx`); an `.xlsx` is tried as an invoice first and, if the
//! invoice parser rejects it, as a statement. A file that fits neither — or a
//! password-protected invoice with no saved password — is skipped and reported in
//! `ignored`, never aborting the scan. Dedup is inherited from the underlying flows
//! (invoice id from filename; bank-entry id UNIQUE), so re-scanning is idempotent.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::application::import_invoice::{import_invoice, ImportError};
use crate::application::store::SharedStore;
use crate::domain::bank_statement::BankEntry;
use crate::domain::{classify_statement, AppConfig, Categorizer};
use crate::infrastructure::btg_statement::read_statement;
use crate::infrastructure::db::{persist, SharedDb};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredFile {
    pub name: String,
    /// `NOT_RECOGNIZED` | `ENCRYPTED_NO_PASSWORD` | `ERROR: <detail>`
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportSummary {
    pub faturas: usize,
    pub extratos: usize,
    pub entries: usize,
    pub ignored: Vec<IgnoredFile>,
    pub directory: String,
}

impl FolderImportSummary {
    /// True when the scan produced nothing worth telling the user about.
    pub fn is_empty(&self) -> bool {
        self.faturas == 0 && self.extratos == 0 && self.ignored.is_empty()
    }
}

enum FaturaOutcome {
    Imported,
    NotInvoice,
    EncryptedNoPassword,
    Error(String),
}

/// Scan `dir` and import every recognizable invoice/statement. Never panics and never
/// aborts on a single bad file; problems land in the returned summary.
pub fn import_from_folder(
    dir: &Path,
    db: &SharedDb,
    store: &SharedStore,
    cfg: &AppConfig,
    password: Option<&str>,
) -> FolderImportSummary {
    let mut summary = FolderImportSummary {
        directory: dir.display().to_string(),
        ..Default::default()
    };

    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) => {
            summary.ignored.push(IgnoredFile {
                name: dir.display().to_string(),
                reason: format!("ERROR: {e}"),
            });
            return summary;
        }
    };

    // Sorted for deterministic processing order (aggregation must be deterministic).
    let mut files: Vec<std::path::PathBuf> = read
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .collect();
    files.sort();

    let payslip_months = payslip_months(db);
    let mut imported_any_fatura = false;

    for path in files {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());

        match ext.as_deref() {
            Some("xlsx") => match try_import_fatura(&path, store, cfg, password) {
                FaturaOutcome::Imported => {
                    summary.faturas += 1;
                    imported_any_fatura = true;
                }
                FaturaOutcome::NotInvoice => {
                    // Maybe it's a statement saved as .xlsx.
                    match try_import_extrato(&path, db, cfg, &payslip_months) {
                        Ok(n) => {
                            summary.extratos += 1;
                            summary.entries += n;
                        }
                        Err(_) => summary.ignored.push(IgnoredFile {
                            name,
                            reason: "NOT_RECOGNIZED".to_string(),
                        }),
                    }
                }
                FaturaOutcome::EncryptedNoPassword => summary.ignored.push(IgnoredFile {
                    name,
                    reason: "ENCRYPTED_NO_PASSWORD".to_string(),
                }),
                FaturaOutcome::Error(e) => summary.ignored.push(IgnoredFile {
                    name,
                    reason: format!("ERROR: {e}"),
                }),
            },
            Some("xls") => match try_import_extrato(&path, db, cfg, &payslip_months) {
                Ok(n) => {
                    summary.extratos += 1;
                    summary.entries += n;
                }
                Err(_) => summary.ignored.push(IgnoredFile {
                    name,
                    reason: "NOT_RECOGNIZED".to_string(),
                }),
            },
            _ => {} // ignore non-spreadsheet files silently
        }
    }

    if imported_any_fatura {
        let snapshot = store.lock().map(|s| s.list_owned()).unwrap_or_default();
        persist(db, &snapshot);
    }

    summary
}

fn payslip_months(db: &SharedDb) -> HashSet<String> {
    db.lock()
        .ok()
        .and_then(|d| d.load_payslips().ok())
        .unwrap_or_default()
        .iter()
        .map(|p| p.month.clone())
        .collect()
}

fn try_import_fatura(
    path: &Path,
    store: &SharedStore,
    cfg: &AppConfig,
    password: Option<&str>,
) -> FaturaOutcome {
    let mut guard = match store.lock() {
        Ok(g) => g,
        Err(e) => return FaturaOutcome::Error(e.to_string()),
    };
    match import_invoice(path, &mut guard, cfg, password) {
        Ok(_) => FaturaOutcome::Imported,
        // Not a BTG invoice sheet → let the caller try the statement parser.
        Err(ImportError::InvalidFormat(_)) | Err(ImportError::ParseError(_)) => {
            FaturaOutcome::NotInvoice
        }
        Err(ImportError::Encrypted) | Err(ImportError::WrongPassword) => {
            FaturaOutcome::EncryptedNoPassword
        }
        Err(ImportError::FileNotFound) => FaturaOutcome::Error("arquivo não encontrado".into()),
    }
}

fn try_import_extrato(
    path: &Path,
    db: &SharedDb,
    cfg: &AppConfig,
    payslip_months: &HashSet<String>,
) -> Result<usize, String> {
    let path_str = path.to_str().ok_or("caminho inválido")?;
    let parsed = read_statement(path_str)?;
    let rules = cfg.category_rules.clone();
    let cz = if rules.is_empty() {
        Categorizer::with_defaults()
    } else {
        Categorizer::new(rules)
    };
    let entries: Vec<BankEntry> = classify_statement(&parsed, &cz, payslip_months)
        .iter()
        .filter(|c| c.included)
        .map(|c| BankEntry::from_classified(c, "BTG", &parsed.account))
        .collect();
    let n = entries.len();
    db.lock().map_err(|e| e.to_string())?.save_bank_entries(&entries)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::store::new_shared_store;
    use crate::infrastructure::db::{new_shared_db, Database};

    fn fatura_fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/fixtures/sample_fatura.xlsx")
    }

    #[test]
    fn imports_fatura_from_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(fatura_fixture(), dir.path().join("fatura1.xlsx")).unwrap();

        let db = new_shared_db(Database::open_in_memory().unwrap());
        let store = new_shared_store();
        let summary = import_from_folder(dir.path(), &db, &store, &AppConfig::default(), None);

        assert_eq!(summary.faturas, 1);
        assert_eq!(store.lock().unwrap().list().len(), 1);
    }

    #[test]
    fn junk_file_is_ignored_without_aborting() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(fatura_fixture(), dir.path().join("ok.xlsx")).unwrap();
        std::fs::write(dir.path().join("junk.xlsx"), b"not a spreadsheet at all").unwrap();
        std::fs::write(dir.path().join("garbage.xls"), b"neither is this").unwrap();

        let db = new_shared_db(Database::open_in_memory().unwrap());
        let store = new_shared_store();
        let summary = import_from_folder(dir.path(), &db, &store, &AppConfig::default(), None);

        assert_eq!(summary.faturas, 1, "the valid invoice must still import");
        assert!(summary.ignored.iter().any(|f| f.name == "junk.xlsx"));
        assert!(summary.ignored.iter().any(|f| f.name == "garbage.xls"));
    }

    #[test]
    fn rescan_does_not_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(fatura_fixture(), dir.path().join("fatura1.xlsx")).unwrap();

        let db = new_shared_db(Database::open_in_memory().unwrap());
        let store = new_shared_store();
        let cfg = AppConfig::default();
        import_from_folder(dir.path(), &db, &store, &cfg, None);
        import_from_folder(dir.path(), &db, &store, &cfg, None);

        assert_eq!(store.lock().unwrap().list().len(), 1);
    }

    #[test]
    fn missing_folder_yields_error_not_panic() {
        let db = new_shared_db(Database::open_in_memory().unwrap());
        let store = new_shared_store();
        let summary = import_from_folder(
            Path::new("/no/such/folder/xyz"),
            &db,
            &store,
            &AppConfig::default(),
            None,
        );
        assert_eq!(summary.faturas, 0);
        assert_eq!(summary.ignored.len(), 1);
    }
}
