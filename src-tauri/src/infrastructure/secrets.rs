//! OS-keychain-backed storage for invoice decryption passwords, one credential per
//! bank.
//!
//! Each bank's invoice password is a distinct reusable credential (BTG: the `.xlsx`
//! file password; Santander: the PDF password), so each lives under its own USER in
//! the OS keychain — encrypted at rest and guarded by the OS — never in
//! `financas.db` in plaintext. BTG keeps the pre-015 USER (`invoice-password`) so a
//! password the user saved before multi-bank support keeps working.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use keyring::Entry;

const SERVICE: &str = "com.financas.app";
const LEGACY_BTG_USER: &str = "invoice-password";

/// Keychain USER for a bank's invoice password. "BTG" maps to the legacy name.
fn user_for(bank: &str) -> String {
    if bank.eq_ignore_ascii_case("btg") {
        LEGACY_BTG_USER.to_string()
    } else {
        format!("invoice-password-{}", bank.to_ascii_lowercase())
    }
}

/// One cached handle per (service, user) credential. Caching keeps every operation
/// pointed at the same credential — which is what the mock store in tests needs, and
/// is a harmless optimization in prod. Entries are leaked: one per bank, tiny.
fn entry_for(bank: &str) -> Result<&'static Entry, String> {
    static ENTRIES: OnceLock<Mutex<HashMap<String, &'static Entry>>> = OnceLock::new();
    let map = ENTRIES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().map_err(|e| e.to_string())?;
    let user = user_for(bank);
    if let Some(e) = guard.get(&user) {
        return Ok(e);
    }
    let entry = Entry::new(SERVICE, &user).map_err(|_| "keychain indisponível".to_string())?;
    let leaked: &'static Entry = Box::leak(Box::new(entry));
    guard.insert(user, leaked);
    Ok(leaked)
}

/// Store (or replace) the saved invoice password for a bank.
pub fn save_password_for(bank: &str, password: &str) -> Result<(), String> {
    entry_for(bank)?.set_password(password).map_err(|e| e.to_string())
}

/// Return a bank's saved password, or `None` if nothing is stored / keychain unreachable.
pub fn get_password_for(bank: &str) -> Option<String> {
    entry_for(bank).ok()?.get_password().ok()
}

/// Remove a bank's saved password. Succeeds even when nothing was stored.
pub fn clear_password_for(bank: &str) -> Result<(), String> {
    match entry_for(bank)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Whether a bank has a password saved.
pub fn has_password_for(bank: &str) -> bool {
    get_password_for(bank).is_some()
}

// ── BTG shortcuts (pre-015 API, kept so existing call sites read naturally) ──

/// Store (or replace) the saved BTG invoice password.
pub fn save_password(password: &str) -> Result<(), String> {
    save_password_for("BTG", password)
}

/// Return the saved BTG password, or `None`.
pub fn get_password() -> Option<String> {
    get_password_for("BTG")
}

/// Remove the saved BTG password.
pub fn clear_password() -> Result<(), String> {
    clear_password_for("BTG")
}

/// Whether a BTG password is currently saved.
pub fn has_password() -> bool {
    has_password_for("BTG")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    // The mock credential store must be installed exactly once per process.
    static INIT: Once = Once::new();
    fn use_mock() {
        INIT.call_once(|| {
            keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        });
    }

    #[test]
    fn save_get_clear_roundtrip() {
        use_mock();
        assert!(!has_password());

        save_password("11144477735").unwrap();
        assert!(has_password());
        assert_eq!(get_password().as_deref(), Some("11144477735"));

        // Overwrite works.
        save_password("newpass").unwrap();
        assert_eq!(get_password().as_deref(), Some("newpass"));

        clear_password().unwrap();
        assert!(!has_password());
        // Clearing again is a no-op, not an error.
        clear_password().unwrap();
    }

    // T004 — one credential per bank, no collisions, BTG on the legacy key.
    #[test]
    fn per_bank_credentials_do_not_collide() {
        use_mock();

        save_password_for("Santander", "senha-santander").unwrap();
        assert!(has_password_for("Santander"));
        assert_eq!(get_password_for("Santander").as_deref(), Some("senha-santander"));

        // Saving Santander must not touch BTG (and vice versa).
        save_password_for("BTG", "senha-btg").unwrap();
        assert_eq!(get_password_for("Santander").as_deref(), Some("senha-santander"));
        assert_eq!(get_password_for("BTG").as_deref(), Some("senha-btg"));

        // The BTG bank key IS the legacy credential: the pre-015 shortcut reads it.
        assert_eq!(get_password().as_deref(), Some("senha-btg"));

        clear_password_for("Santander").unwrap();
        assert!(!has_password_for("Santander"));
        assert!(has_password_for("BTG"), "limpar Santander não pode limpar BTG");
        clear_password_for("BTG").unwrap();
        // Clearing an absent credential is a no-op.
        clear_password_for("Santander").unwrap();
    }

    #[test]
    fn bank_name_maps_to_stable_keychain_user() {
        assert_eq!(user_for("BTG"), "invoice-password");
        assert_eq!(user_for("btg"), "invoice-password");
        assert_eq!(user_for("Santander"), "invoice-password-santander");
    }
}
