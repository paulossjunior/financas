//! Application use-case (feature 013): scan a single folder and auto-import every
//! recognizable BTG invoice (`.xlsx`), BTG statement (`.xls`/`.xlsx`) and Banestes
//! statement (`.pdf`) found in it.
//!
//! Type detection reuses the existing parsers: an `.xls` is treated as a statement
//! (invoices are always `.xlsx`); an `.xlsx` is tried as an invoice first and, if the
//! invoice parser rejects it, as a statement; a `.pdf` is imported only when it is
//! recognizably a Banestes statement, so a payslip in the same folder is left alone
//! (feature 014).
//!
//! A **spreadsheet** that fits neither parser — or a password-protected invoice with no
//! saved password — is reported in `ignored`: the user put it here expecting it to be
//! imported, so silence would hide a real failure. A **PDF that is not a statement** is
//! skipped silently instead; payslips legitimately live in this folder and reporting each
//! one at every app start would look like an error. Nothing ever aborts the scan. Dedup is
//! inherited from the underlying flows (invoice id from filename; bank-entry id UNIQUE),
//! so re-scanning is idempotent.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::application::import_invoice::{import_invoice, ImportError};
use crate::application::store::SharedStore;
use crate::domain::bank_statement::BankEntry;
use crate::domain::{classify_statement, AppConfig, Categorizer};
use crate::infrastructure::db::{persist, SharedDb};
use crate::infrastructure::statement_reader::statement_reader_for;
use crate::infrastructure::{santander_invoice, secrets};

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
            // A PDF here is a bank statement, an (encrypted) Santander invoice, or
            // someone else's document — a payslip, most often. `pdf_route` decides
            // (see the function for the policy rationale); a PDF routed to silence
            // is not a problem to report, while a *recognized* document that fails
            // to import always is.
            Some("pdf") => {
                let Some(path_str) = path.to_str() else { continue };
                let is_statement = statement_reader_for(path_str)
                    .is_some_and(|r| r.recognizes(path_str));
                let is_encrypted = !is_statement && santander_invoice::is_encrypted_pdf(path_str);
                let santander_pw = secrets::get_password_for("Santander");
                match pdf_route(is_statement, is_encrypted, santander_pw.is_some()) {
                    PdfRoute::Extrato => match try_import_extrato(&path, db, cfg, &payslip_months) {
                        Ok(n) => {
                            summary.extratos += 1;
                            summary.entries += n;
                        }
                        Err(e) => summary.ignored.push(IgnoredFile {
                            name,
                            reason: format!("ERROR: {e}"),
                        }),
                    },
                    PdfRoute::Fatura => {
                        let outcome = (|| {
                            let mut guard = store.lock().map_err(|e| e.to_string())?;
                            import_invoice(&path, &mut guard, cfg, santander_pw.as_deref())
                                .map_err(|e| match e {
                                    // The *saved* password failing is worth naming —
                                    // the user fixes it in Settings, not in the folder.
                                    ImportError::WrongPassword => {
                                        "a senha salva do Santander não confere".to_string()
                                    }
                                    ImportError::Encrypted => "ENCRYPTED_NO_PASSWORD".to_string(),
                                    other => other.to_string(),
                                })
                        })();
                        match outcome {
                            Ok(_) => {
                                summary.faturas += 1;
                                imported_any_fatura = true;
                            }
                            Err(reason) if reason == "ENCRYPTED_NO_PASSWORD" => summary
                                .ignored
                                .push(IgnoredFile { name, reason }),
                            Err(reason) => summary.ignored.push(IgnoredFile {
                                name,
                                reason: format!("ERROR: {reason}"),
                            }),
                        }
                    }
                    PdfRoute::FaturaSemSenha => summary.ignored.push(IgnoredFile {
                        name,
                        reason: "ENCRYPTED_NO_PASSWORD".to_string(),
                    }),
                    PdfRoute::Silencio => {}
                }
            }
            _ => {} // ignore other files silently
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

/// Where a `.pdf` in the auto-import folder goes. Pure — the whole policy in one
/// testable place:
///
/// - a recognized bank statement imports as extrato (014);
/// - an **encrypted** PDF is a Santander-invoice candidate — it is the only
///   encrypted document in this app's universe (payslips and statements are open) —
///   imported with the saved password, or reported once as `ENCRYPTED_NO_PASSWORD`
///   so the user knows to save the password in Settings;
/// - any other PDF (a payslip) is skipped silently: this folder legitimately holds
///   them, and reporting each one at every app start would read as an error.
#[derive(Debug, PartialEq)]
enum PdfRoute {
    Extrato,
    Fatura,
    FaturaSemSenha,
    Silencio,
}

fn pdf_route(is_statement: bool, is_encrypted: bool, has_invoice_password: bool) -> PdfRoute {
    if is_statement {
        PdfRoute::Extrato
    } else if is_encrypted {
        if has_invoice_password {
            PdfRoute::Fatura
        } else {
            PdfRoute::FaturaSemSenha
        }
    } else {
        PdfRoute::Silencio
    }
}

/// Read (via the strategy registry), classify and persist one statement file.
fn try_import_extrato(
    path: &Path,
    db: &SharedDb,
    cfg: &AppConfig,
    payslip_months: &HashSet<String>,
) -> Result<usize, String> {
    let path_str = path.to_str().ok_or("caminho inválido")?;
    let reader = statement_reader_for(path_str).ok_or("formato sem leitor de extrato")?;
    let parsed = reader.read(path_str)?;
    save_entries(db, &classify_for_persist(&parsed, cfg, payslip_months))
}

/// Persist bank entries (dedup is the id UNIQUE constraint) and report how many.
fn save_entries(db: &SharedDb, entries: &[BankEntry]) -> Result<usize, String> {
    db.lock().map_err(|e| e.to_string())?.save_bank_entries(entries)?;
    Ok(entries.len())
}

fn classify_for_persist(
    parsed: &crate::domain::ParsedStatement,
    cfg: &AppConfig,
    payslip_months: &HashSet<String>,
) -> Vec<BankEntry> {
    let rules = cfg.category_rules.clone();
    let cz = if rules.is_empty() {
        Categorizer::with_defaults()
    } else {
        Categorizer::new(rules)
    };
    classify_statement(parsed, &cz, payslip_months)
        .iter()
        .filter(|c| c.included)
        .map(|c| BankEntry::from_classified(c, &parsed.bank, &parsed.account))
        .collect()
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

    fn extrato_text(name: &str) -> String {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("fixture {p:?}: {e}"))
    }

    /// Test seam: the statement branch from already-extracted text, so the folder
    /// behaviour is testable without a real (personal-data) PDF in the repository.
    /// Production goes through the reader strategy instead (`try_import_extrato`);
    /// both funnel into the same `classify_for_persist`.
    fn import_extrato_text(
        text: &str,
        cfg: &AppConfig,
        payslip_months: &HashSet<String>,
    ) -> Result<Vec<BankEntry>, String> {
        let parsed = crate::domain::parse_banestes_text(text)?;
        Ok(classify_for_persist(&parsed, cfg, payslip_months))
    }

    // T039 — the PDF branch, exercised from the extracted text (no real statement
    // PDF in the repository; the pdf_extract call itself is the untested shell).
    #[test]
    fn imports_banestes_statement_text_from_folder() {
        let entries = import_extrato_text(
            &extrato_text("banestes_extrato.txt"),
            &AppConfig::default(),
            &HashSet::new(),
        )
        .unwrap();
        assert_eq!(entries.len(), 9);
        assert!(entries.iter().all(|e| e.bank == "Banestes"));
        assert!(entries.iter().all(|e| e.account == "12/1234567-8"));
    }

    // T040
    #[test]
    fn payslip_pdf_is_not_treated_as_a_statement() {
        let payslip = "Comprovante de Rendimentos\nSouGov.br\nJUL 2026\nLíquido a Receber 10.345,67";
        assert!(!crate::domain::is_banestes_statement(payslip));
        assert!(import_extrato_text(payslip, &AppConfig::default(), &HashSet::new()).is_err());
    }

    // T040
    #[test]
    fn statement_that_does_not_reconcile_is_reported_not_imported() {
        let err = import_extrato_text(
            &extrato_text("banestes_extrato_quebrado.txt"),
            &AppConfig::default(),
            &HashSet::new(),
        )
        .unwrap_err();
        assert!(err.contains("não fech"), "{err}");
    }

    // T041 — re-importing the same statement text must not add rows.
    #[test]
    fn rescanning_a_statement_does_not_duplicate() {
        let db = new_shared_db(Database::open_in_memory().unwrap());
        let cfg = AppConfig::default();
        for _ in 0..2 {
            let entries =
                import_extrato_text(&extrato_text("banestes_extrato.txt"), &cfg, &HashSet::new())
                    .unwrap();
            db.lock().unwrap().save_bank_entries(&entries).unwrap();
        }
        assert_eq!(db.lock().unwrap().load_bank_entries().unwrap().len(), 9);
    }

    // T035 — both banks coexist in the same table without id collision.
    #[test]
    fn entries_from_both_banks_coexist() {
        let db = new_shared_db(Database::open_in_memory().unwrap());
        let cfg = AppConfig::default();
        let banestes =
            import_extrato_text(&extrato_text("banestes_extrato.txt"), &cfg, &HashSet::new()).unwrap();
        db.lock().unwrap().save_bank_entries(&banestes).unwrap();

        let mut btg = banestes.clone();
        for e in &mut btg {
            e.bank = "BTG".into();
            e.account = "286969-2".into();
            e.id = format!("btg-{}", e.id);
        }
        db.lock().unwrap().save_bank_entries(&btg).unwrap();

        let all = db.lock().unwrap().load_bank_entries().unwrap();
        assert_eq!(all.len(), 18);
        assert_eq!(all.iter().filter(|e| e.bank == "Banestes").count(), 9);
        assert_eq!(all.iter().filter(|e| e.bank == "BTG").count(), 9);
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

    // T030 — the whole `.pdf` routing policy, in one table (research R8).
    #[test]
    fn pdf_route_policy_table() {
        use PdfRoute::*;
        // (is_statement, is_encrypted, has_password) → route
        assert_eq!(pdf_route(true, false, false), Extrato, "extrato reconhecido");
        assert_eq!(pdf_route(true, true, true), Extrato, "extrato vence qualquer outra rota");
        assert_eq!(pdf_route(false, true, true), Fatura, "cifrado + senha salva importa");
        assert_eq!(pdf_route(false, true, false), FaturaSemSenha, "cifrado sem senha é reportado");
        assert_eq!(pdf_route(false, false, true), Silencio, "PDF aberto que não é extrato = contracheque");
        assert_eq!(pdf_route(false, false, false), Silencio);
    }

    /// Test seam for the Santander-invoice folder branch: import from already-
    /// extracted text (no personal-data PDF in the repository). Mirrors what
    /// `import_invoice` does after `extract_text`; production goes through the
    /// reader strategy — both funnel into the same store/dedup path.
    fn import_fatura_santander_text(
        text: &str,
        filename: &str,
        store: &SharedStore,
        cfg: &AppConfig,
    ) -> Result<usize, String> {
        use crate::domain::santander_invoice::FaturaSantander;
        use crate::domain::{Categorizer, Invoice};
        use chrono::DateTime;
        use uuid::Uuid;

        let fatura = FaturaSantander::parse(text)?;
        fatura.conferir().exigir()?;
        let invoice_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, filename.as_bytes());
        let rules = cfg.category_rules.clone();
        let cz = if rules.is_empty() { Categorizer::with_defaults() } else { Categorizer::new(rules) };
        let month = fatura.reference_month(filename);
        let (txs, _) = fatura.into_transactions(invoice_id, &cz);
        let n = txs.len();
        let mut invoice = Invoice::new(
            filename.to_string(),
            month,
            None,
            txs,
            DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        );
        invoice.bank = "Santander".to_string();
        store.lock().map_err(|e| e.to_string())?.add(invoice);
        Ok(n)
    }

    // T030 — a Santander invoice (as text) lands in the store with its bank and month.
    #[test]
    fn imports_santander_invoice_text_from_folder() {
        let store = new_shared_store();
        let n = import_fatura_santander_text(
            &extrato_text("santander_fatura.txt"),
            "Fatura_072026_MARIA_1111_VISA_000_SANTANDER.PDF",
            &store,
            &AppConfig::default(),
        )
        .unwrap();
        assert_eq!(n, 15, "14 despesas + 1 cashback");
        let guard = store.lock().unwrap();
        let list = guard.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].bank, "Santander");
        assert_eq!(list[0].reference_month.to_string_iso(), "2026-07");
    }

    // T030 — a tampered invoice must not touch the store.
    #[test]
    fn santander_invoice_that_does_not_reconcile_is_not_stored() {
        let store = new_shared_store();
        let err = import_fatura_santander_text(
            &extrato_text("santander_fatura_quebrada.txt"),
            "Fatura_072026_MARIA_1111_VISA_000_SANTANDER.PDF",
            &store,
            &AppConfig::default(),
        )
        .unwrap_err();
        assert!(err.contains("não fechou"), "{err}");
        assert_eq!(store.lock().unwrap().list().len(), 0, "nada gravado");
    }

    // T031 — re-scanning the same invoice replaces it (same filename identity).
    #[test]
    fn rescanning_a_santander_invoice_does_not_duplicate() {
        let store = new_shared_store();
        let cfg = AppConfig::default();
        for _ in 0..2 {
            import_fatura_santander_text(
                &extrato_text("santander_fatura.txt"),
                "Fatura_072026_MARIA_1111_VISA_000_SANTANDER.PDF",
                &store,
                &cfg,
            )
            .unwrap();
        }
        let guard = store.lock().unwrap();
        assert_eq!(guard.list().len(), 1, "substitui, não duplica");
        assert_eq!(guard.list()[0].transactions.len(), 15);
    }

    // A PDF that is not a statement (a payslip, or any unreadable one) must leave no
    // trace in the summary: this folder is scanned at every app start, and reporting the
    // user's payslips as "ignorado" every time would read as a recurring error.
    #[test]
    fn pdf_that_is_not_a_statement_is_skipped_silently() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(fatura_fixture(), dir.path().join("ok.xlsx")).unwrap();
        std::fs::write(dir.path().join("contracheque_7_2026.pdf"), b"%PDF-1.4 not a statement").unwrap();

        let db = new_shared_db(Database::open_in_memory().unwrap());
        let store = new_shared_store();
        let summary = import_from_folder(dir.path(), &db, &store, &AppConfig::default(), None);

        assert_eq!(summary.faturas, 1, "the valid invoice must still import");
        assert_eq!(summary.extratos, 0);
        assert!(summary.ignored.is_empty(), "ignorados: {:?}", summary.ignored);
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
