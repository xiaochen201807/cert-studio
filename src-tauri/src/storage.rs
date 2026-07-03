use crate::error::{AppError, AppResult};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "cert-studio";
const KEYRING_USER: &str = "root-ca-key";
const LOCAL_KEY_LEN: usize = 32;
const LOCAL_NONCE_LEN: usize = 12;
const LOCAL_TAG_LEN: usize = 16;
const LOCAL_KEY_FILE_MAGIC: &[u8] = b"CSKEY1";

// 获取应用的 AppData 目录
fn get_app_dir(app_handle: &AppHandle) -> AppResult<PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Custom(format!("无法获取 AppData 目录: {}", e)))
}

// 旧版本本地回退使用 XOR 混淆。保留读取兼容，新的保存逻辑不再使用。
fn xor_encrypt_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

fn set_owner_only_permissions(path: &PathBuf) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = fs::set_permissions(path, perms);
        }
    }
}

fn get_or_create_local_storage_key(storage_key_path: &PathBuf) -> AppResult<Vec<u8>> {
    if storage_key_path.exists() {
        let key = fs::read(storage_key_path)?;
        if key.len() == LOCAL_KEY_LEN {
            return Ok(key);
        }

        return Err(AppError::Custom(
            "本地 Root CA 私钥加密密钥长度不正确，请检查 storage.key 文件。".to_string(),
        ));
    }

    use rand::RngCore;
    let mut key = vec![0u8; LOCAL_KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    fs::write(storage_key_path, &key)?;
    set_owner_only_permissions(storage_key_path);
    Ok(key)
}

fn encrypt_root_ca_key_for_local_storage(key_pem: &str, storage_key: &[u8]) -> AppResult<Vec<u8>> {
    use rand::RngCore;

    let mut nonce = [0u8; LOCAL_NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce);

    let mut tag = [0u8; LOCAL_TAG_LEN];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        storage_key,
        Some(&nonce),
        LOCAL_KEY_FILE_MAGIC,
        key_pem.as_bytes(),
        &mut tag,
    )?;

    let mut output = Vec::with_capacity(
        LOCAL_KEY_FILE_MAGIC.len() + LOCAL_NONCE_LEN + LOCAL_TAG_LEN + ciphertext.len(),
    );
    output.extend_from_slice(LOCAL_KEY_FILE_MAGIC);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&tag);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

fn decrypt_root_ca_key_from_local_storage(
    encrypted_data: &[u8],
    storage_key: &[u8],
) -> AppResult<String> {
    let plaintext = if encrypted_data.starts_with(LOCAL_KEY_FILE_MAGIC) {
        let header_len = LOCAL_KEY_FILE_MAGIC.len();
        let min_len = header_len + LOCAL_NONCE_LEN + LOCAL_TAG_LEN;
        if encrypted_data.len() <= min_len {
            return Err(AppError::Custom(
                "本地 Root CA 私钥密文格式不完整。".to_string(),
            ));
        }

        let nonce_start = header_len;
        let tag_start = nonce_start + LOCAL_NONCE_LEN;
        let ciphertext_start = tag_start + LOCAL_TAG_LEN;
        let nonce = &encrypted_data[nonce_start..tag_start];
        let tag = &encrypted_data[tag_start..ciphertext_start];
        let ciphertext = &encrypted_data[ciphertext_start..];

        decrypt_aead(
            Cipher::aes_256_gcm(),
            storage_key,
            Some(nonce),
            LOCAL_KEY_FILE_MAGIC,
            ciphertext,
            tag,
        )?
    } else {
        xor_encrypt_decrypt(encrypted_data, storage_key)
    };

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Custom(format!("私钥解密后非有效的 UTF-8 文本: {}", e)))
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

    // 若系统密钥环失败，回退到本地 AES-GCM 加密文件
    if !keyring_success {
        let app_dir = get_app_dir(app_handle)?;
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir)?;
        }

        let key_file = app_dir.join("root-ca.key.enc");
        let storage_key_path = app_dir.join("storage.key");

        let key = get_or_create_local_storage_key(&storage_key_path)?;
        let encrypted_data = encrypt_root_ca_key_for_local_storage(key_pem, &key)?;
        fs::write(&key_file, encrypted_data)?;
        set_owner_only_permissions(&key_file);
    }

    Ok(())
}

// 获取私钥 (优先从系统密钥环获取，回退本地加密文件)
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
    decrypt_root_ca_key_from_local_storage(&encrypted_data, &key)
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

// 获取根证书文件物理路径
pub fn get_root_ca_cert_path(app_handle: &AppHandle) -> AppResult<PathBuf> {
    let app_dir = get_app_dir(app_handle)?;
    Ok(app_dir.join("root-ca.crt"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_storage_encryption_roundtrips_private_key_text() {
        let storage_key = [7u8; LOCAL_KEY_LEN];
        let key_pem = "-----BEGIN PRIVATE KEY-----\nlocal-key\n-----END PRIVATE KEY-----";

        let encrypted = encrypt_root_ca_key_for_local_storage(key_pem, &storage_key).unwrap();
        let decrypted = decrypt_root_ca_key_from_local_storage(&encrypted, &storage_key).unwrap();

        assert_eq!(decrypted, key_pem);
        assert!(encrypted.starts_with(LOCAL_KEY_FILE_MAGIC));
        assert_ne!(encrypted, key_pem.as_bytes());
    }

    #[test]
    fn local_storage_decryption_keeps_legacy_xor_compatibility() {
        let storage_key = [11u8; LOCAL_KEY_LEN];
        let key_pem = "legacy-private-key";
        let legacy_encrypted = xor_encrypt_decrypt(key_pem.as_bytes(), &storage_key);

        let decrypted = decrypt_root_ca_key_from_local_storage(&legacy_encrypted, &storage_key).unwrap();

        assert_eq!(decrypted, key_pem);
    }

    #[test]
    fn local_storage_decryption_rejects_truncated_aes_payload() {
        let storage_key = [13u8; LOCAL_KEY_LEN];
        let truncated = LOCAL_KEY_FILE_MAGIC.to_vec();

        let err = decrypt_root_ca_key_from_local_storage(&truncated, &storage_key)
            .expect_err("truncated AES payload should fail");

        assert!(err.to_string().contains("密文格式不完整"));
    }
}
