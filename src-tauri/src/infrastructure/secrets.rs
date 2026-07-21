//! OS-keychain-backed storage for the invoice decryption password.
//!
//! The password (a BTG file password, often the account holder's CPF) is a
//! reusable credential, so it lives in the OS keychain — encrypted at rest and
//! guarded by the OS — never in `financas.db` in plaintext.

use keyring::Entry;
use std::sync::OnceLock;

const SERVICE: &str = "com.financas.app";
const USER: &str = "invoice-password";

/// Single cached handle to the (service, user) credential. Caching it (rather than
/// rebuilding per call) keeps every operation pointed at the same credential — which
/// is what the mock store in tests needs, and is a harmless optimization in prod.
fn entry() -> Result<&'static Entry, String> {
    static ENTRY: OnceLock<Option<Entry>> = OnceLock::new();
    ENTRY
        .get_or_init(|| Entry::new(SERVICE, USER).ok())
        .as_ref()
        .ok_or_else(|| "keychain indisponível".to_string())
}

/// Store (or replace) the saved invoice password.
pub fn save_password(password: &str) -> Result<(), String> {
    entry()?.set_password(password).map_err(|e| e.to_string())
}

/// Return the saved password, or `None` if nothing is stored / the keychain is unreachable.
pub fn get_password() -> Option<String> {
    entry().ok()?.get_password().ok()
}

/// Remove the saved password. Succeeds even when nothing was stored.
pub fn clear_password() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Whether a password is currently saved.
pub fn has_password() -> bool {
    get_password().is_some()
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

        save_password("05512570757").unwrap();
        assert!(has_password());
        assert_eq!(get_password().as_deref(), Some("05512570757"));

        // Overwrite works.
        save_password("newpass").unwrap();
        assert_eq!(get_password().as_deref(), Some("newpass"));

        clear_password().unwrap();
        assert!(!has_password());
        // Clearing again is a no-op, not an error.
        clear_password().unwrap();
    }
}
