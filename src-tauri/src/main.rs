// Prevents additional console window on Windows in release, do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ca;
mod cert;
mod error;
mod export;
mod fs_utils;
mod storage;

use ca::{
    create_root_ca, export_root_ca_backup, get_root_ca_info, has_valid_root_ca, import_root_ca,
    import_root_ca_backup, import_system_trust,
};
use cert::{issue_server_cert, CertBundleStore};
use export::{export_cert_bundle, export_root_ca_cert};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Cert Studio!", name)
}

fn main() {
    tauri::Builder::default()
        .manage(CertBundleStore::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            create_root_ca,
            import_root_ca,
            get_root_ca_info,
            has_valid_root_ca,
            export_root_ca_backup,
            import_root_ca_backup,
            issue_server_cert,
            export_cert_bundle,
            export_root_ca_cert,
            import_system_trust
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
