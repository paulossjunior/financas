use crate::infrastructure::secrets;

/// Whether an invoice password is saved in the OS keychain.
#[tauri::command]
pub fn has_saved_password() -> bool {
    secrets::has_password()
}

/// Forget the saved invoice password.
#[tauri::command]
pub fn clear_saved_password() -> Result<(), String> {
    secrets::clear_password()
}
