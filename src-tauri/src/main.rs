// Prevents additional console window on Windows in release, do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod error;
mod storage;
mod ca;
mod cert;
mod export;

use ca::{create_root_ca, import_root_ca, get_root_ca_info, has_valid_root_ca, read_text_file};
use cert::issue_server_cert;
use export::{export_cert_bundle, export_root_ca_cert};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Cert Studio!", name)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            create_root_ca,
            import_root_ca,
            get_root_ca_info,
            has_valid_root_ca,
            read_text_file,
            issue_server_cert,
            export_cert_bundle,
            export_root_ca_cert
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
