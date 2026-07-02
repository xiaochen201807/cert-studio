use crate::error::{AppError, AppResult};
use crate::storage;
use rcgen::{CertificateParams, KeyPair, DistinguishedName, DnType, IsCa, KeyUsagePurpose, BasicConstraints};
use ::time::{OffsetDateTime, Duration};
use x509_parser::prelude::*;
use x509_parser::pem::parse_x509_pem;
use sha2::{Sha256, Digest};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RootCaInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
    pub sha256_fingerprint: String,
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
    // 1. 验证证书和私钥是否有效
    // 验证私钥
    let key_pair = KeyPair::from_pem(&key_pem)
        .map_err(|e| AppError::Custom(format!("私钥 PEM 解析失败: {}", e)))?;
    
    // 验证证书并解析
    let info = parse_cert_info(&cert_pem)?;
    
    // 用 rcgen 加载 CA 证书参数并自签校验是否与私钥匹配
    let ca_params = CertificateParams::from_ca_cert_pem(&cert_pem)
        .map_err(|e| AppError::Custom(format!("从证书 PEM 提取参数失败: {}", e)))?;
    
    if ca_params.self_signed(&key_pair).is_err() {
        return Err(AppError::Custom("根证书与私钥不匹配，请检查私钥是否正确。".to_string()));
    }
    
    // 2. 保存
    storage::save_root_ca_cert(&app_handle, &cert_pem)?;
    storage::save_root_ca_key(&app_handle, &key_pem)?;
    
    Ok(info)
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
