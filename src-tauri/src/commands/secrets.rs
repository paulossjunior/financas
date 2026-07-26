//! Commands: OS keychain access for saved invoice passwords (check / clear), one
//! credential per bank. `bank` defaults to "BTG" so the pre-015 frontend keeps
//! working unchanged.

use crate::infrastructure::secrets;

/// Whether an invoice password is saved in the OS keychain for `bank` (default BTG).
#[tauri::command]
pub fn has_saved_password(bank: Option<String>) -> bool {
    secrets::has_password_for(bank.as_deref().unwrap_or("BTG"))
}

/// Forget the saved invoice password of `bank` (default BTG).
#[tauri::command]
pub fn clear_saved_password(bank: Option<String>) -> Result<(), String> {
    secrets::clear_password_for(bank.as_deref().unwrap_or("BTG"))
}
