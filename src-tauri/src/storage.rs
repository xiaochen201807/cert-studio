use crate::error::{AppError, AppResult};
use crate::fs_utils::{atomic_write, PRIVATE_FILE_MODE, PUBLIC_FILE_MODE};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "cert-studio";
const LEGACY_KEYRING_USER: &str = "root-ca-key";
const ACTIVE_MATERIAL_FILE: &str = "active-root-ca";
const LOCAL_KEY_LEN: usize = 32;
const LOCAL_NONCE_LEN: usize = 12;
const LOCAL_TAG_LEN: usize = 16;
const LOCAL_KEY_FILE_MAGIC: &[u8] = b"CSKEY1";

fn get_app_dir(app_handle: &AppHandle) -> AppResult<PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Custom(format!("无法获取 AppData 目录: {}", e)))
}

fn material_id(cert_pem: &str) -> String {
    let digest = Sha256::digest(cert_pem.as_bytes());
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn validate_material_id(id: &str) -> AppResult<()> {
    if id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(());
    }

    Err(AppError::Custom("活动 Root CA 材料标识无效。".to_string()))
}

fn active_material_id(app_dir: &Path) -> AppResult<Option<String>> {
    let path = app_dir.join(ACTIVE_MATERIAL_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let id = fs::read_to_string(path)?.trim().to_ascii_lowercase();
    validate_material_id(&id)?;
    Ok(Some(id))
}

fn cert_path(app_dir: &Path, id: &str) -> PathBuf {
    app_dir.join(format!("root-ca-{}.crt", id))
}

fn encrypted_key_path(app_dir: &Path, id: &str) -> PathBuf {
    app_dir.join(format!("root-ca-{}.key.enc", id))
}

fn keyring_user(id: &str) -> String {
    format!("root-ca-key-{}", id)
}

fn xor_encrypt_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key[index % key.len()])
        .collect()
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
    atomic_write(storage_key_path, &key, PRIVATE_FILE_MODE)?;
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
    if storage_key.len() != LOCAL_KEY_LEN {
        return Err(AppError::Custom(
            "本地 Root CA 私钥加密密钥长度不正确。".to_string(),
        ));
    }

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
        decrypt_aead(
            Cipher::aes_256_gcm(),
            storage_key,
            Some(&encrypted_data[nonce_start..tag_start]),
            LOCAL_KEY_FILE_MAGIC,
            &encrypted_data[ciphertext_start..],
            &encrypted_data[tag_start..ciphertext_start],
        )?
    } else {
        xor_encrypt_decrypt(encrypted_data, storage_key)
    };

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Custom(format!("私钥解密后非有效的 UTF-8 文本: {}", e)))
}

fn save_material_key(app_dir: &Path, id: &str, key_pem: &str) -> AppResult<()> {
    let user = keyring_user(id);
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &user) {
        if entry.set_password(key_pem).is_ok() {
            let _ = fs::remove_file(encrypted_key_path(app_dir, id));
            return Ok(());
        }
    }

    let storage_key_path = app_dir.join("storage.key");
    let storage_key = get_or_create_local_storage_key(&storage_key_path)?;
    let encrypted_data = encrypt_root_ca_key_for_local_storage(key_pem, &storage_key)?;
    atomic_write(
        &encrypted_key_path(app_dir, id),
        &encrypted_data,
        PRIVATE_FILE_MODE,
    )
}

fn get_material_key(app_dir: &Path, id: &str) -> AppResult<String> {
    let user = keyring_user(id);
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &user) {
        if let Ok(password) = entry.get_password() {
            return Ok(password);
        }
    }

    let key_file = encrypted_key_path(app_dir, id);
    let storage_key_path = app_dir.join("storage.key");
    if !key_file.exists() || !storage_key_path.exists() {
        return Err(AppError::Custom(
            "未找到当前 Root CA 对应的私钥。".to_string(),
        ));
    }

    let storage_key = fs::read(storage_key_path)?;
    let encrypted_data = fs::read(key_file)?;
    decrypt_root_ca_key_from_local_storage(&encrypted_data, &storage_key)
}

fn get_legacy_key(app_dir: &Path) -> AppResult<String> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, LEGACY_KEYRING_USER) {
        if let Ok(password) = entry.get_password() {
            return Ok(password);
        }
    }

    let key_file = app_dir.join("root-ca.key.enc");
    let storage_key_path = app_dir.join("storage.key");
    if !key_file.exists() || !storage_key_path.exists() {
        return Err(AppError::Custom("未找到 Root CA 私钥。".to_string()));
    }

    decrypt_root_ca_key_from_local_storage(&fs::read(key_file)?, &fs::read(storage_key_path)?)
}

fn cleanup_material(app_dir: &Path, id: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &keyring_user(id)) {
        let _ = entry.delete_credential();
    }
    let _ = fs::remove_file(cert_path(app_dir, id));
    let _ = fs::remove_file(encrypted_key_path(app_dir, id));
}

fn cleanup_legacy_material(app_dir: &Path) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, LEGACY_KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    let _ = fs::remove_file(app_dir.join("root-ca.crt"));
    let _ = fs::remove_file(app_dir.join("root-ca.key.enc"));
}

pub fn save_root_ca_material(
    app_handle: &AppHandle,
    cert_pem: &str,
    key_pem: &str,
) -> AppResult<()> {
    let app_dir = get_app_dir(app_handle)?;
    fs::create_dir_all(&app_dir)?;

    let new_id = material_id(cert_pem);
    let previous_id = active_material_id(&app_dir)?;

    save_material_key(&app_dir, &new_id, key_pem)?;
    if let Err(error) = atomic_write(
        &cert_path(&app_dir, &new_id),
        cert_pem.as_bytes(),
        PUBLIC_FILE_MODE,
    ) {
        cleanup_material(&app_dir, &new_id);
        return Err(error);
    }

    let saved_cert = fs::read_to_string(cert_path(&app_dir, &new_id))?;
    let saved_key = get_material_key(&app_dir, &new_id)?;
    if saved_cert != cert_pem || saved_key != key_pem {
        cleanup_material(&app_dir, &new_id);
        return Err(AppError::Custom(
            "Root CA 材料写入校验失败，原有 Root CA 未被替换。".to_string(),
        ));
    }

    atomic_write(
        &app_dir.join(ACTIVE_MATERIAL_FILE),
        new_id.as_bytes(),
        PRIVATE_FILE_MODE,
    )?;

    if let Some(previous_id) = previous_id {
        if previous_id != new_id {
            cleanup_material(&app_dir, &previous_id);
        }
    }
    cleanup_legacy_material(&app_dir);
    Ok(())
}

pub fn get_root_ca_key(app_handle: &AppHandle) -> AppResult<String> {
    let app_dir = get_app_dir(app_handle)?;
    match active_material_id(&app_dir)? {
        Some(id) => get_material_key(&app_dir, &id),
        None => get_legacy_key(&app_dir),
    }
}

pub fn get_root_ca_cert_path(app_handle: &AppHandle) -> AppResult<PathBuf> {
    let app_dir = get_app_dir(app_handle)?;
    match active_material_id(&app_dir)? {
        Some(id) => Ok(cert_path(&app_dir, &id)),
        None => Ok(app_dir.join("root-ca.crt")),
    }
}

pub fn get_root_ca_cert(app_handle: &AppHandle) -> AppResult<String> {
    let cert_file = get_root_ca_cert_path(app_handle)?;
    if !cert_file.exists() {
        return Err(AppError::Custom("未找到 Root CA 证书。".to_string()));
    }
    Ok(fs::read_to_string(cert_file)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_id_is_stable_and_filename_safe() {
        let first = material_id("certificate");
        let second = material_id("certificate");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

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
        let decrypted =
            decrypt_root_ca_key_from_local_storage(&legacy_encrypted, &storage_key).unwrap();
        assert_eq!(decrypted, key_pem);
    }

    #[test]
    fn local_storage_decryption_rejects_truncated_aes_payload() {
        let storage_key = [13u8; LOCAL_KEY_LEN];
        let err = decrypt_root_ca_key_from_local_storage(LOCAL_KEY_FILE_MAGIC, &storage_key)
            .expect_err("truncated AES payload should fail");
        assert!(err.to_string().contains("密文格式不完整"));
    }

    #[test]
    fn local_storage_decryption_rejects_invalid_key_length() {
        let err = decrypt_root_ca_key_from_local_storage(b"ciphertext", &[1, 2, 3])
            .expect_err("invalid key length should fail");
        assert!(err.to_string().contains("加密密钥长度不正确"));
    }
}
