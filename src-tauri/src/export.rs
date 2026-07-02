use crate::error::{AppError, AppResult};
use crate::cert::CertBundle;
use crate::storage;
use std::fs;
use std::path::Path;

#[tauri::command]
pub fn export_cert_bundle(
    app_handle: tauri::AppHandle,
    bundle: CertBundle,
    output_dir: String,
) -> AppResult<()> {
    let out_path = Path::new(&output_dir);
    if !out_path.exists() {
        fs::create_dir_all(out_path)?;
    }

    // 写入 server.crt
    fs::write(out_path.join("server.crt"), &bundle.cert_pem)?;

    // 写入 server.key
    fs::write(out_path.join("server.key"), &bundle.key_pem)?;

    // 写入 fullchain.pem
    fs::write(out_path.join("fullchain.pem"), &bundle.fullchain_pem)?;

    // 导出 root-ca 证书 (如果本地已存在)
    if let Ok(ca_cert) = storage::get_root_ca_cert(&app_handle) {
        fs::write(out_path.join("company-root-ca.crt"), ca_cert)?;
    }

    // 写入 nginx.conf
    fs::write(out_path.join("nginx.conf"), &bundle.nginx_config)?;

    // 写入 electron.md
    fs::write(out_path.join("electron.md"), &bundle.electron_readme)?;

    Ok(())
}

#[tauri::command]
pub fn export_root_ca_cert(
    app_handle: tauri::AppHandle,
    output_dir: String,
) -> AppResult<()> {
    let out_path = Path::new(&output_dir);
    if !out_path.exists() {
        fs::create_dir_all(out_path)?;
    }
    
    let ca_cert = storage::get_root_ca_cert(&app_handle)?;
    fs::write(out_path.join("company-root-ca.crt"), ca_cert)?;
    
    Ok(())
}
