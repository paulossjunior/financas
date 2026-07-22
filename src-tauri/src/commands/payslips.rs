//! Commands: payslips (contracheque) at the Tauri boundary — import/preview a PDF,
//! save, list and remove.

use std::path::PathBuf;
use tauri::State;

use crate::domain::payslip::{parse_payslip_text, Payslip};
use crate::infrastructure::db::SharedDb;

/// Parse a payslip PDF and return the extracted+classified data WITHOUT saving.
/// The UI shows this for confirmation before `save_payslip`.
#[tauri::command]
pub async fn import_payslip(path: String) -> Result<Payslip, String> {
    let pb = PathBuf::from(&path);
    if !pb.exists() {
        return Err("FILE_NOT_FOUND".into());
    }
    let text = pdf_extract::extract_text(&pb)
        .map_err(|e| format!("Falha ao ler o PDF: {e}"))?;
    let file = pb
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let mut p = parse_payslip_text(&text, &file)?;
    p.imported_at = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    Ok(p)
}

/// Persist a (possibly user-corrected) payslip. Re-importing the same month replaces it.
#[tauri::command]
pub async fn save_payslip(payslip: Payslip, db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.save_payslip(&payslip)
}

#[tauri::command]
pub async fn list_payslips(db: State<'_, SharedDb>) -> Result<Vec<Payslip>, String> {
    db.lock().map_err(|e| e.to_string())?.load_payslips()
}

#[tauri::command]
pub async fn remove_payslip(month: String, db: State<'_, SharedDb>) -> Result<(), String> {
    db.lock().map_err(|e| e.to_string())?.remove_payslip(&month)
}
