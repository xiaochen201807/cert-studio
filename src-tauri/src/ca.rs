use crate::error::{AppError, AppResult};
use crate::storage;
use base64::{engine::general_purpose, Engine as _};
use openssl::{
    hash::MessageDigest,
    pkcs5::pbkdf2_hmac,
    pkey::PKey,
    symm::{decrypt_aead, encrypt_aead, Cipher},
    x509::X509,
};
use rcgen::{CertificateParams, KeyPair, DistinguishedName, DnType, IsCa, KeyUsagePurpose, BasicConstraints};
use ::time::{OffsetDateTime, Duration};
use x509_parser::prelude::*;
use x509_parser::pem::parse_x509_pem;
use sha2::{Sha256, Digest};

const ROOT_CA_BACKUP_VERSION: u32 = 1;
const ROOT_CA_BACKUP_ITERATIONS: usize = 200_000;
const ROOT_CA_BACKUP_SALT_LEN: usize = 16;
const ROOT_CA_BACKUP_NONCE_LEN: usize = 12;
const ROOT_CA_BACKUP_TAG_LEN: usize = 16;
const ROOT_CA_BACKUP_AAD: &[u8] = b"cert-studio-root-ca-backup-v1";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RootCaInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
    pub sha256_fingerprint: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct RootCaBackupPayload {
    cert_pem: String,
    key_pem: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct RootCaBackupFile {
    version: u32,
    kdf: String,
    cipher: String,
    iterations: usize,
    salt_base64: String,
    nonce_base64: String,
    tag_base64: String,
    ciphertext_base64: String,
}

// 解析 PEM 证书信息
pub fn parse_cert_info(cert_pem: &str) -> AppResult<RootCaInfo> {
    let (_, pem) = parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| AppError::X509(format!("PEM 解析失败: {}", e)))?;
    
    let x509 = pem.parse_x509()
        .map_err(|e| AppError::X509(format!("X509 解析失败: {}", e)))?;
    
    let subject = x509.subject().to_string();
    let issuer = x509.issuer().to_string();
    
    // 直接使用 Display 实现获取时间字符串，格式可读且极其稳定
    let not_before = x509.validity().not_before.to_string();
    let not_after = x509.validity().not_after.to_string();
    
    // 计算 SHA256 指纹
    let mut hasher = Sha256::new();
    hasher.update(&pem.contents);
    let hash = hasher.finalize();
    let sha256_fingerprint = hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<String>>()
        .join(":");
        
    Ok(RootCaInfo {
        subject,
        issuer,
        not_before,
        not_after,
        sha256_fingerprint,
    })
}

#[tauri::command]
pub fn create_root_ca(
    app_handle: tauri::AppHandle,
    common_name: String,
    organization: Option<String>,
    days: u32,
) -> AppResult<RootCaInfo> {
    // 1. 生成密钥对
    let key_pair = KeyPair::generate()?;
    
    // 2. 构造 CA 参数
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &common_name);
    if let Some(ref org) = organization {
        dn.push(DnType::OrganizationName, org);
    }
    params.distinguished_name = dn;
    
    // 设定有效期
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(days as i64);
    
    // 3. 生成证书
    let cert = params.self_signed(&key_pair)?;
    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    
    // 4. 保存到本地 (安全存储)
    storage::save_root_ca_cert(&app_handle, &cert_pem)?;
    storage::save_root_ca_key(&app_handle, &key_pem)?;
    
    // 5. 解析并返回 CA 信息
    parse_cert_info(&cert_pem)
}

#[tauri::command]
pub fn import_root_ca(
    app_handle: tauri::AppHandle,
    cert_pem: String,
    key_pem: String,
) -> AppResult<RootCaInfo> {
    let info = validate_import_root_ca_pems(&cert_pem, &key_pem)?;

    // 2. 保存
    storage::save_root_ca_cert(&app_handle, &cert_pem)?;
    storage::save_root_ca_key(&app_handle, &key_pem)?;
    
    Ok(info)
}

fn validate_import_root_ca_pems(cert_pem: &str, key_pem: &str) -> AppResult<RootCaInfo> {
    let cert = X509::from_pem(cert_pem.as_bytes())
        .map_err(|e| AppError::Custom(format!("Root CA 证书 PEM 解析失败: {}", e)))?;
    let key = PKey::private_key_from_pem(key_pem.as_bytes())
        .map_err(|e| AppError::Custom(format!("Root CA 私钥 PEM 解析失败: {}", e)))?;
    let cert_public_key = cert
        .public_key()
        .map_err(|e| AppError::Custom(format!("无法读取 Root CA 证书公钥: {}", e)))?;
    if !cert_public_key.public_eq(&key) {
        return Err(AppError::Custom(
            "Root CA 证书与私钥不匹配，请确认选择的是同一套证书和私钥。".to_string(),
        ));
    }

    KeyPair::from_pem(key_pem)
        .map_err(|e| AppError::Custom(format!("Root CA 私钥不是 rcgen 支持的 PEM 格式: {}", e)))?;

    let info = parse_cert_info(cert_pem)?;
    CertificateParams::from_ca_cert_pem(cert_pem)
        .map_err(|e| AppError::Custom(format!("Root CA 证书不是有效的 CA 证书或不受当前解析器支持: {}", e)))?;

    Ok(info)
}

fn derive_backup_key(password: &str, salt: &[u8], iterations: usize) -> AppResult<[u8; 32]> {
    let password = password.trim();
    if password.is_empty() {
        return Err(AppError::Custom("请设置 Root CA 备份包密码。".to_string()));
    }

    let mut key = [0u8; 32];
    pbkdf2_hmac(
        password.as_bytes(),
        salt,
        iterations,
        MessageDigest::sha256(),
        &mut key,
    )?;
    Ok(key)
}

fn encrypt_root_ca_backup(payload: &RootCaBackupPayload, password: &str) -> AppResult<RootCaBackupFile> {
    use rand::RngCore;

    let mut salt = [0u8; ROOT_CA_BACKUP_SALT_LEN];
    let mut nonce = [0u8; ROOT_CA_BACKUP_NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    let key = derive_backup_key(password, &salt, ROOT_CA_BACKUP_ITERATIONS)?;
    let plaintext = serde_json::to_vec(payload)?;
    let mut tag = [0u8; ROOT_CA_BACKUP_TAG_LEN];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(&nonce),
        ROOT_CA_BACKUP_AAD,
        &plaintext,
        &mut tag,
    )?;

    Ok(RootCaBackupFile {
        version: ROOT_CA_BACKUP_VERSION,
        kdf: "PBKDF2-HMAC-SHA256".to_string(),
        cipher: "AES-256-GCM".to_string(),
        iterations: ROOT_CA_BACKUP_ITERATIONS,
        salt_base64: general_purpose::STANDARD.encode(salt),
        nonce_base64: general_purpose::STANDARD.encode(nonce),
        tag_base64: general_purpose::STANDARD.encode(tag),
        ciphertext_base64: general_purpose::STANDARD.encode(ciphertext),
    })
}

fn decrypt_root_ca_backup(backup: &RootCaBackupFile, password: &str) -> AppResult<RootCaBackupPayload> {
    if backup.version != ROOT_CA_BACKUP_VERSION {
        return Err(AppError::Custom(format!(
            "不支持的 Root CA 备份包版本: {}",
            backup.version
        )));
    }
    if backup.kdf != "PBKDF2-HMAC-SHA256" || backup.cipher != "AES-256-GCM" {
        return Err(AppError::Custom("不支持的 Root CA 备份包加密参数。".to_string()));
    }

    let salt = general_purpose::STANDARD.decode(&backup.salt_base64)?;
    let nonce = general_purpose::STANDARD.decode(&backup.nonce_base64)?;
    let tag = general_purpose::STANDARD.decode(&backup.tag_base64)?;
    let ciphertext = general_purpose::STANDARD.decode(&backup.ciphertext_base64)?;

    if nonce.len() != ROOT_CA_BACKUP_NONCE_LEN || tag.len() != ROOT_CA_BACKUP_TAG_LEN {
        return Err(AppError::Custom("Root CA 备份包密文参数长度不正确。".to_string()));
    }

    let key = derive_backup_key(password, &salt, backup.iterations)?;
    let plaintext = decrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(&nonce),
        ROOT_CA_BACKUP_AAD,
        &ciphertext,
        &tag,
    )
    .map_err(|_| AppError::Custom("Root CA 备份包密码错误或文件已损坏。".to_string()))?;

    serde_json::from_slice(&plaintext).map_err(AppError::from)
}

#[tauri::command]
pub fn export_root_ca_backup(
    app_handle: tauri::AppHandle,
    output_dir: String,
    password: String,
) -> AppResult<String> {
    let cert_pem = storage::get_root_ca_cert(&app_handle)?;
    let key_pem = storage::get_root_ca_key(&app_handle)?;
    let payload = RootCaBackupPayload { cert_pem, key_pem };
    let backup = encrypt_root_ca_backup(&payload, &password)?;
    let backup_json = serde_json::to_string_pretty(&backup)?;

    let out_path = std::path::Path::new(&output_dir);
    if !out_path.exists() {
        std::fs::create_dir_all(out_path)?;
    }

    let backup_path = out_path.join("cert-studio-root-ca-backup.json");
    std::fs::write(&backup_path, backup_json)?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_root_ca_backup(
    app_handle: tauri::AppHandle,
    backup_path: String,
    password: String,
) -> AppResult<RootCaInfo> {
    let backup_json = std::fs::read_to_string(backup_path)?;
    let backup: RootCaBackupFile = serde_json::from_str(&backup_json)?;
    let payload = decrypt_root_ca_backup(&backup, &password)?;

    import_root_ca(app_handle, payload.cert_pem, payload.key_pem)
}

#[tauri::command]
pub fn get_root_ca_info(app_handle: tauri::AppHandle) -> AppResult<RootCaInfo> {
    let cert_pem = storage::get_root_ca_cert(&app_handle)?;
    parse_cert_info(&cert_pem)
}

#[tauri::command]
pub fn has_valid_root_ca(app_handle: tauri::AppHandle) -> bool {
    storage::has_root_ca(&app_handle)
}

#[tauri::command]
pub fn read_text_file(path: String) -> AppResult<String> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

#[tauri::command]
pub fn import_system_trust(app_handle: tauri::AppHandle) -> AppResult<()> {
    // 1. 获取根证书的绝对路径
    let cert_path = storage::get_root_ca_cert_path(&app_handle)?;
    if !cert_path.exists() {
        return Err(AppError::Custom("未找到 Root CA 证书，请先生成或导入根证书。".to_string()));
    }
    
    // 2. 根据不同的操作系统执行不同的命令
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        // macOS: 通过 security add-trusted-cert 命令导入
        let status = Command::new("security")
            .arg("add-trusted-cert")
            .arg("-d")
            .arg("-r")
            .arg("trustRoot")
            .arg("-k")
            .arg("/Library/Keychains/System.keychain")
            .arg(cert_path)
            .status()
            .map_err(|e| AppError::Custom(format!("执行 security 命令失败: {}", e)))?;
            
        if !status.success() {
            return Err(AppError::Custom("授权取消或导入信任失败。".to_string()));
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // Windows: 通过 certutil 命令导入当前用户受信任区
        let status = Command::new("certutil")
            .arg("-user")
            .arg("-addstore")
            .arg("-f")
            .arg("Root")
            .arg(cert_path)
            .status()
            .map_err(|e| AppError::Custom(format!("执行 certutil 命令失败: {}", e)))?;
            
        if !status.success() {
            return Err(AppError::Custom("授权取消或导入信任失败。".to_string()));
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let script = r#"
set -e
src="$1"
if command -v update-ca-certificates >/dev/null 2>&1; then
  install -m 0644 "$src" /usr/local/share/ca-certificates/cert-studio-root-ca.crt
  update-ca-certificates
elif command -v update-ca-trust >/dev/null 2>&1; then
  install -D -m 0644 "$src" /etc/pki/ca-trust/source/anchors/cert-studio-root-ca.crt
  update-ca-trust extract
elif command -v trust >/dev/null 2>&1; then
  trust anchor --store "$src"
else
  echo "未找到支持的 Linux 根证书信任更新工具：update-ca-certificates、update-ca-trust 或 trust" >&2
  exit 2
fi
"#;

        let mut command = if Command::new("pkexec").arg("--version").output().is_ok() {
            Command::new("pkexec")
        } else {
            Command::new("sudo")
        };

        let status = command
            .arg("sh")
            .arg("-c")
            .arg(script)
            .arg("cert-studio-trust-import")
            .arg(cert_path)
            .status()
            .map_err(|e| AppError::Custom(format!("执行 Linux 授权导入命令失败: {}", e)))?;
            
        if !status.success() {
            return Err(AppError::Custom("授权取消或导入信任失败。".to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ca_pair(common_name: &str) -> AppResult<(String, String)> {
        let key_pair = KeyPair::generate()?;
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, common_name);
        params.distinguished_name = dn;

        let cert = params.self_signed(&key_pair)?;
        Ok((cert.pem(), key_pair.serialize_pem()))
    }

    #[test]
    fn import_validation_accepts_matching_ca_pair() {
        let (cert_pem, key_pem) = sample_ca_pair("Test Root CA").unwrap();

        let info = validate_import_root_ca_pems(&cert_pem, &key_pem).unwrap();

        assert!(info.subject.contains("Test Root CA"));
        assert_eq!(info.sha256_fingerprint.len(), 95);
    }

    #[test]
    fn import_validation_rejects_mismatched_key() {
        let (cert_pem, _) = sample_ca_pair("Test Root CA").unwrap();
        let (_, other_key_pem) = sample_ca_pair("Other Root CA").unwrap();

        let err = validate_import_root_ca_pems(&cert_pem, &other_key_pem)
            .expect_err("mismatched key should fail");

        assert!(err.to_string().contains("证书与私钥不匹配"));
    }

    #[test]
    fn import_validation_reports_malformed_cert_pem() {
        let (_, key_pem) = sample_ca_pair("Test Root CA").unwrap();

        let err = validate_import_root_ca_pems("not a pem", &key_pem)
            .expect_err("malformed cert should fail");

        assert!(err.to_string().contains("Root CA 证书 PEM 解析失败"));
    }

    #[test]
    fn backup_encrypt_decrypt_roundtrips_payload() {
        let payload = RootCaBackupPayload {
            cert_pem: "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----".to_string(),
            key_pem: "-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----".to_string(),
        };

        let backup = encrypt_root_ca_backup(&payload, "strong-password").unwrap();
        let restored = decrypt_root_ca_backup(&backup, "strong-password").unwrap();

        assert_eq!(restored.cert_pem, payload.cert_pem);
        assert_eq!(restored.key_pem, payload.key_pem);
        assert_eq!(backup.version, ROOT_CA_BACKUP_VERSION);
        assert_eq!(backup.kdf, "PBKDF2-HMAC-SHA256");
        assert_eq!(backup.cipher, "AES-256-GCM");
    }

    #[test]
    fn backup_decrypt_rejects_wrong_password() {
        let payload = RootCaBackupPayload {
            cert_pem: "cert".to_string(),
            key_pem: "key".to_string(),
        };
        let backup = encrypt_root_ca_backup(&payload, "correct-password").unwrap();

        let err = decrypt_root_ca_backup(&backup, "wrong-password")
            .expect_err("wrong password should fail");

        assert!(err.to_string().contains("密码错误或文件已损坏"));
    }

    #[test]
    fn derive_backup_key_rejects_empty_password() {
        let salt = [0u8; ROOT_CA_BACKUP_SALT_LEN];

        let err = derive_backup_key("   ", &salt, ROOT_CA_BACKUP_ITERATIONS)
            .expect_err("empty password should fail");

        assert!(err.to_string().contains("请设置 Root CA 备份包密码"));
    }
}
