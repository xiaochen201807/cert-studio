use crate::error::{AppError, AppResult};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "cert-studio";
const KEYRING_USER: &str = "root-ca-key";

// 获取应用的 AppData 目录
fn get_app_dir(app_handle: &AppHandle) -> AppResult<PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Custom(format!("无法获取 AppData 目录: {}", e)))
}

// 混淆加密/解密辅助函数
fn xor_encrypt_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

// 保存私钥 (优先系统密钥环，若失败则本地加密存储)
pub fn save_root_ca_key(app_handle: &AppHandle, key_pem: &str) -> AppResult<()> {
    let mut keyring_success = false;

    // 尝试写入系统密钥环
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if entry.set_password(key_pem).is_ok() {
            keyring_success = true;
        }
    }

    // 若系统密钥环失败，回退到本地混淆文件
    if !keyring_success {
        let app_dir = get_app_dir(app_handle)?;
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir)?;
        }

        let key_file = app_dir.join("root-ca.key.enc");
        let storage_key_path = app_dir.join("storage.key");

        // 如果本地混淆密钥不存在，则生成一个
        let key = if storage_key_path.exists() {
            fs::read(&storage_key_path)?
        } else {
            use rand::RngCore;
            let mut k = vec![0u8; 32];
            rand::thread_rng().fill_bytes(&mut k);
            fs::write(&storage_key_path, &k)?;
            
            // 限制文件权限 (Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&storage_key_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o600);
                    let _ = fs::set_permissions(&storage_key_path, perms);
                }
            }
            k
        };

        let encrypted_data = xor_encrypt_decrypt(key_pem.as_bytes(), &key);
        fs::write(&key_file, encrypted_data)?;

        // 限制文件权限 (Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&key_file) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = fs::set_permissions(&key_file, perms);
            }
        }
    }

    Ok(())
}

// 获取私钥 (优先从系统密钥环获取，回退本地混淆文件)
pub fn get_root_ca_key(app_handle: &AppHandle) -> AppResult<String> {
    // 优先读取密钥环
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(password) = entry.get_password() {
            return Ok(password);
        }
    }

    // 从本地混淆文件读取
    let app_dir = get_app_dir(app_handle)?;
    let key_file = app_dir.join("root-ca.key.enc");
    let storage_key_path = app_dir.join("storage.key");

    if !key_file.exists() || !storage_key_path.exists() {
        return Err(AppError::Custom("未找到 Root CA 私钥密文或混淆密钥".to_string()));
    }

    let key = fs::read(&storage_key_path)?;
    let encrypted_data = fs::read(&key_file)?;
    let decrypted_data = xor_encrypt_decrypt(&encrypted_data, &key);

    String::from_utf8(decrypted_data)
        .map_err(|e| AppError::Custom(format!("私钥解密后非有效的 UTF-8 文本: {}", e)))
}

// 保存根证书 PEM (证书为公开信息，明文存储于 AppData 下)
pub fn save_root_ca_cert(app_handle: &AppHandle, cert_pem: &str) -> AppResult<()> {
    let app_dir = get_app_dir(app_handle)?;
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }
    let cert_file = app_dir.join("root-ca.crt");
    fs::write(cert_file, cert_pem)?;
    Ok(())
}

// 获取根证书 PEM
pub fn get_root_ca_cert(app_handle: &AppHandle) -> AppResult<String> {
    let app_dir = get_app_dir(app_handle)?;
    let cert_file = app_dir.join("root-ca.crt");
    if !cert_file.exists() {
        return Err(AppError::Custom("未找到 Root CA 证书".to_string()));
    }
    let content = fs::read_to_string(cert_file)?;
    Ok(content)
}

// 判断是否存在 Root CA
pub fn has_root_ca(app_handle: &AppHandle) -> bool {
    let app_dir = match get_app_dir(app_handle) {
        Ok(dir) => dir,
        Err(_) => return false,
    };
    
    let has_cert = app_dir.join("root-ca.crt").exists();
    if !has_cert {
        return false;
    }

    // 检查私钥是否在 Keyring 中或存在于本地加密文件
    let has_keyring = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map(|entry| entry.get_password().is_ok())
        .unwrap_or(false);

    let has_local_key = app_dir.join("root-ca.key.enc").exists() && app_dir.join("storage.key").exists();

    has_keyring || has_local_key
}
