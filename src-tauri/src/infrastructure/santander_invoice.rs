//! Read a Santander card invoice (encrypted PDF) and parse it.
//!
//! Thin I/O shell: PDF (password-protected — the holder's CPF) → text → domain. All
//! grammar and integrity rules live in `domain::santander_invoice`, so they are
//! testable without a binary fixture (real invoices carry personal data and never
//! enter the repository).

use std::path::Path;

use uuid::Uuid;

use crate::domain::categorizer::Categorizer;
use crate::domain::santander_invoice::{is_santander_invoice, FaturaSantander};
use crate::infrastructure::invoice_reader::{InvoiceRead, InvoiceReadError, InvoiceReader};

/// True when pdf_extract's error means "this document is encrypted".
fn is_encryption_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("password") || m.contains("encrypt")
}

/// Extract the text of an invoice PDF, decrypting when a password is given.
/// Errors map to the invoice pipeline: encrypted-without-password → `Encrypted`
/// (front asks for the password), wrong password → `WrongPassword`.
pub fn extract_text(path: &str, password: Option<&str>) -> Result<String, InvoiceReadError> {
    if !Path::new(path).exists() {
        return Err(InvoiceReadError::Io("Arquivo não encontrado.".into()));
    }
    let open_plain = || {
        pdf_extract::extract_text(path).map_err(|e| {
            let msg = e.to_string();
            if is_encryption_error(&msg) {
                InvoiceReadError::Encrypted
            } else {
                InvoiceReadError::Io(format!("Não consegui ler o PDF: {msg}"))
            }
        })
    };
    let attempt = match password {
        Some(p) => match pdf_extract::extract_text_encrypted(path, p) {
            Ok(t) => Ok(t),
            Err(e) => {
                let msg = e.to_string();
                if msg.to_lowercase().contains("password is incorrect") {
                    Err(InvoiceReadError::WrongPassword)
                } else {
                    // Decrypting an *unencrypted* PDF fails on a missing crypt
                    // dictionary — the file just doesn't need the password. Retry
                    // plain so a saved password never breaks an open PDF.
                    open_plain()
                }
            }
        },
        None => open_plain(),
    }?;
    if attempt.trim().is_empty() {
        return Err(InvoiceReadError::InvalidFormat(
            "Este PDF não tem texto para ler (pode ser digitalizado ou protegido).".into(),
        ));
    }
    Ok(attempt)
}

/// True when the PDF at `path` is encrypted. In this app's document universe only
/// Santander invoices are encrypted PDFs, so the auto-import folder uses this as the
/// cheap "candidate invoice" sniff before it has a password (research R8).
pub fn is_encrypted_pdf(path: &str) -> bool {
    match pdf_extract::extract_text(path) {
        Ok(_) => false,
        Err(e) => is_encryption_error(&e.to_string()),
    }
}

/// Strategy: the Santander card invoice — an encrypted PDF whose extracted text
/// carries the whole grammar (see `domain::santander_invoice`).
pub struct SantanderInvoiceReader;

impl InvoiceReader for SantanderInvoiceReader {
    fn bank(&self) -> &'static str {
        "Santander"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
    }

    fn read(
        &self,
        path: &Path,
        password: Option<&str>,
        invoice_id: Uuid,
        categorizer: &Categorizer,
    ) -> Result<InvoiceRead, InvoiceReadError> {
        let path_str = path.to_str().ok_or_else(|| InvoiceReadError::Io("caminho inválido".into()))?;
        let text = extract_text(path_str, password)?;
        if !is_santander_invoice(&text) {
            return Err(InvoiceReadError::InvalidFormat(
                "Este PDF não é uma fatura do Santander.".into(),
            ));
        }
        let fatura = FaturaSantander::parse(&text).map_err(InvoiceReadError::InvalidFormat)?;
        fatura.conferir().exigir().map_err(InvoiceReadError::InvalidFormat)?;

        // The reader owns the month (Fatura_MMYYYY filename / printed due date —
        // research R10); BTG's YYYY-MM inference does not apply to these files.
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or(path_str);
        let reference_month = Some(fatura.reference_month(filename));
        let (transactions, warnings) = fatura.into_transactions(invoice_id, categorizer);
        Ok(InvoiceRead { transactions, warnings, reference_month })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T026 — the I/O shell fails loudly and in Portuguese, never panics.
    #[test]
    fn missing_file_and_non_pdf_fail_with_clear_errors() {
        match extract_text("/no/such/dir/fatura.pdf", None) {
            Err(InvoiceReadError::Io(msg)) => assert!(msg.contains("não encontrado"), "{msg}"),
            other => panic!("esperava Io(não encontrado), veio {other:?}"),
        }

        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("nao-e-pdf.pdf");
        std::fs::write(&fake, b"apenas texto, nao um PDF").unwrap();
        match extract_text(fake.to_str().unwrap(), None) {
            Err(InvoiceReadError::Io(msg) | InvoiceReadError::InvalidFormat(msg)) => {
                assert!(!msg.is_empty())
            }
            Err(InvoiceReadError::Encrypted | InvoiceReadError::WrongPassword) => {
                panic!("arquivo de texto não é 'cifrado'")
            }
            Ok(_) => panic!("texto puro não pode passar por PDF"),
            Err(InvoiceReadError::Empty) => {}
        }
    }
}
