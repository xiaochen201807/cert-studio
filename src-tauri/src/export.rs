use crate::cert::CertBundleStore;
use crate::error::{AppError, AppResult};
use crate::fs_utils::{atomic_write, PRIVATE_FILE_MODE, PUBLIC_FILE_MODE};
use crate::storage;
use base64::{engine::general_purpose, Engine as _};
use std::path::{Path, PathBuf};

fn validate_output_directory(output_dir: &str) -> AppResult<PathBuf> {
    let path = Path::new(output_dir);
    if !path.exists() || !path.is_dir() {
        return Err(AppError::Custom(
            "导出目录不存在，请通过目录选择器选择已有目录。".to_string(),
        ));
    }
    Ok(path.canonicalize()?)
}

#[tauri::command]
pub fn export_cert_bundle(
    app_handle: tauri::AppHandle,
    store: tauri::State<'_, CertBundleStore>,
    bundle_id: String,
    output_dir: String,
) -> AppResult<()> {
    let out_path = validate_output_directory(&output_dir)?;
    let bundle = store.get(&bundle_id)?;

    atomic_write(
        &out_path.join("server.crt"),
        bundle.cert_pem.as_bytes(),
        PUBLIC_FILE_MODE,
    )?;
    atomic_write(
        &out_path.join("server.key"),
        bundle.key_pem.as_bytes(),
        PRIVATE_FILE_MODE,
    )?;
    atomic_write(
        &out_path.join("fullchain.pem"),
        bundle.fullchain_pem.as_bytes(),
        PUBLIC_FILE_MODE,
    )?;

    // 写入 server.pfx
    if let Some(pfx_base64) = &bundle.pfx_base64 {
        let pfx_bytes = general_purpose::STANDARD.decode(pfx_base64)?;
        atomic_write(&out_path.join("server.pfx"), &pfx_bytes, PRIVATE_FILE_MODE)?;
    } else {
        return Err(AppError::Custom("未生成 PFX/PKCS#12 证书包".to_string()));
    }

    // 导出 root-ca 证书 (如果本地已存在)
    if let Ok(ca_cert) = storage::get_root_ca_cert(&app_handle) {
        atomic_write(
            &out_path.join("company-root-ca.crt"),
            ca_cert.as_bytes(),
            PUBLIC_FILE_MODE,
        )?;
    }

    // 写入 nginx.conf
    atomic_write(
        &out_path.join("nginx.conf"),
        bundle.nginx_config.as_bytes(),
        PUBLIC_FILE_MODE,
    )?;

    // 写入 electron.md
    atomic_write(
        &out_path.join("electron.md"),
        bundle.electron_readme.as_bytes(),
        PUBLIC_FILE_MODE,
    )?;

    Ok(())
}

#[tauri::command]
pub fn export_root_ca_cert(app_handle: tauri::AppHandle, output_dir: String) -> AppResult<()> {
    let out_path = validate_output_directory(&output_dir)?;

    let ca_cert = storage::get_root_ca_cert(&app_handle)?;
    atomic_write(
        &out_path.join("company-root-ca.crt"),
        ca_cert.as_bytes(),
        PUBLIC_FILE_MODE,
    )?;

    Ok(())
}
