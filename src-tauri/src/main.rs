//! Binary entry point — delegates to [`financas_lib::run`] to launch the Tauri app.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    financas_lib::run();
}
