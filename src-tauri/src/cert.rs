use crate::error::{AppError, AppResult};
use crate::storage;
use base64::{engine::general_purpose, Engine as _};
use openssl::{pkcs12::Pkcs12, pkey::PKey, stack::Stack, x509::X509};
use rcgen::Ia5String;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, SanType,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use time::{Duration, OffsetDateTime};

#[derive(serde::Deserialize, Clone, Debug)]
pub struct IssueCertRequest {
    pub common_name: String,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub days: u32,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub country: Option<String>,
    pub state: Option<String>,
    pub locality: Option<String>,
    pub pfx_password: String,
}

#[derive(Clone, Debug)]
pub struct CertBundle {
    pub cert_pem: String,
    pub key_pem: String,
    pub fullchain_pem: String,
    pub pfx_base64: Option<String>,
    pub nginx_config: String,
    pub electron_readme: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct CertBundlePreview {
    pub bundle_id: String,
    pub cert_pem: String,
    pub nginx_config: String,
    pub electron_readme: String,
}

#[derive(Default)]
pub struct CertBundleStore {
    bundles: Mutex<HashMap<String, CertBundle>>,
}

impl CertBundleStore {
    fn insert(&self, bundle: CertBundle) -> AppResult<CertBundlePreview> {
        use rand::RngCore;

        let mut id_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut id_bytes);
        let bundle_id = id_bytes
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        let preview = CertBundlePreview {
            bundle_id: bundle_id.clone(),
            cert_pem: bundle.cert_pem.clone(),
            nginx_config: bundle.nginx_config.clone(),
            electron_readme: bundle.electron_readme.clone(),
        };

        let mut bundles = self
            .bundles
            .lock()
            .map_err(|_| AppError::Custom("证书导出缓存不可用。".to_string()))?;
        bundles.clear();
        bundles.insert(bundle_id, bundle);
        Ok(preview)
    }

    pub fn get(&self, bundle_id: &str) -> AppResult<CertBundle> {
        let bundles = self
            .bundles
            .lock()
            .map_err(|_| AppError::Custom("证书导出缓存不可用。".to_string()))?;
        bundles
            .get(bundle_id)
            .cloned()
            .ok_or_else(|| AppError::Custom("证书导出会话已失效，请重新签发。".to_string()))
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_valid_dns_name(value: &str, allow_wildcard: bool) -> bool {
    let value = if allow_wildcard {
        value.strip_prefix("*.").unwrap_or(value)
    } else {
        value
    };

    if value.is_empty() || value.len() > 253 || !value.is_ascii() || value.ends_with('.') {
        return false;
    }

    value.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn validate_and_normalize_request(mut request: IssueCertRequest) -> AppResult<IssueCertRequest> {
    request.common_name = request.common_name.trim().to_ascii_lowercase();
    request.pfx_password = request.pfx_password.trim().to_string();
    request.dns_names = request
        .dns_names
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();
    request.ip_addresses = request
        .ip_addresses
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    request.organization = normalize_optional(request.organization);
    request.organizational_unit = normalize_optional(request.organizational_unit);
    request.country = normalize_optional(request.country).map(|value| value.to_ascii_uppercase());
    request.state = normalize_optional(request.state);
    request.locality = normalize_optional(request.locality);

    if !is_valid_dns_name(&request.common_name, false) {
        return Err(AppError::Custom(
            "Common Name 必须是有效且不含通配符的 DNS 名称。".to_string(),
        ));
    }
    if request.dns_names.is_empty()
        || request
            .dns_names
            .iter()
            .any(|name| !is_valid_dns_name(name, true))
    {
        return Err(AppError::Custom("请提供有效的 DNS SAN 名称。".to_string()));
    }
    if !request
        .dns_names
        .iter()
        .any(|name| name == &request.common_name)
    {
        return Err(AppError::Custom(
            "Common Name 必须同时包含在 DNS SAN 中。".to_string(),
        ));
    }
    for ip in &request.ip_addresses {
        ip.parse::<IpAddr>()
            .map_err(|_| AppError::Custom(format!("非法的 IP 地址 '{}'", ip)))?;
    }
    if !(1..=825).contains(&request.days) {
        return Err(AppError::Custom(
            "服务端证书有效期必须在 1 到 825 天之间。".to_string(),
        ));
    }
    if request.pfx_password.chars().count() < 8 {
        return Err(AppError::Custom(
            "PFX/PKCS#12 密码至少需要 8 个字符。".to_string(),
        ));
    }
    if let Some(country) = &request.country {
        if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err(AppError::Custom("国家代码必须是两个英文字母。".to_string()));
        }
    }

    Ok(request)
}

pub fn issue_server_cert_internal(
    app_handle: &tauri::AppHandle,
    request: IssueCertRequest,
) -> AppResult<CertBundle> {
    let ca_cert_pem = storage::get_root_ca_cert(app_handle)?;
    let ca_key_pem = storage::get_root_ca_key(app_handle)?;
    issue_server_cert_with_material(&ca_cert_pem, &ca_key_pem, request)
}

fn issue_server_cert_with_material(
    ca_cert_pem: &str,
    ca_key_pem: &str,
    request: IssueCertRequest,
) -> AppResult<CertBundle> {
    let request = validate_and_normalize_request(request)?;
    crate::ca::validate_import_root_ca_pems(ca_cert_pem, ca_key_pem)?;

    let ca_params = CertificateParams::from_ca_cert_pem(ca_cert_pem)?;
    let now = OffsetDateTime::now_utc();
    let requested_not_after = now + Duration::days(request.days as i64);
    if requested_not_after > ca_params.not_after {
        return Err(AppError::Custom(
            "服务端证书有效期不能超过 Root CA 的到期时间。".to_string(),
        ));
    }
    let ca_key_pair = KeyPair::from_pem(ca_key_pem)?;
    let ca_cert = ca_params.self_signed(&ca_key_pair)?;

    // 2. 构造子证书参数
    // 在 rcgen 中使用首个 dns name 作为基础构建 params，如果没有则使用 CN 构建
    let primary_dns = request
        .dns_names
        .first()
        .cloned()
        .unwrap_or_else(|| request.common_name.clone());
    let mut params = CertificateParams::new(vec![primary_dns])?;

    // 组装 Subject Alternative Names
    let mut subject_alt_names = Vec::new();
    for dns in &request.dns_names {
        let ia5 = Ia5String::try_from(dns.as_str())
            .map_err(|e| AppError::Custom(format!("非法的 DNS 域名 '{}': {}", dns, e)))?;
        subject_alt_names.push(SanType::DnsName(ia5));
    }
    for ip in &request.ip_addresses {
        let ip_addr: IpAddr = ip
            .parse()
            .map_err(|_| AppError::Custom(format!("非法的 IP 地址 '{}'", ip)))?;
        subject_alt_names.push(SanType::IpAddress(ip_addr));
    }
    params.subject_alt_names = subject_alt_names;

    // 组装 DN (Distinguished Name)
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &request.common_name);
    if let Some(ref org) = request.organization {
        dn.push(DnType::OrganizationName, org);
    }
    if let Some(ref ou) = request.organizational_unit {
        dn.push(DnType::OrganizationalUnitName, ou);
    }
    if let Some(ref c) = request.country {
        dn.push(DnType::CountryName, c);
    }
    if let Some(ref st) = request.state {
        dn.push(DnType::StateOrProvinceName, st);
    }
    if let Some(ref l) = request.locality {
        dn.push(DnType::LocalityName, l);
    }
    params.distinguished_name = dn;

    // 设定有效期与密钥用途
    params.not_before = now;
    params.not_after = requested_not_after;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    // 生成子证书密钥对并用 CA 签名
    let server_key_pair = KeyPair::generate()?;
    let signed_cert = params.signed_by(&server_key_pair, &ca_cert, &ca_key_pair)?;

    let cert_pem = signed_cert.pem();
    let key_pem = server_key_pair.serialize_pem();

    // 拼接 fullchain
    let fullchain_pem = format!("{}\n{}", cert_pem, ca_cert_pem);

    // 生成受密码保护的 PFX/PKCS#12 证书包，便于 Windows/IIS/.NET 等工具链导入。
    let pfx_base64 = generate_pfx_base64_internal(
        &cert_pem,
        &key_pem,
        ca_cert_pem,
        &request.common_name,
        &request.pfx_password,
    )?;

    // nginx 配置文件生成
    let nginx_config = generate_nginx_config_internal(&request.common_name);

    // Electron README
    let electron_readme = generate_electron_readme_internal();

    Ok(CertBundle {
        cert_pem,
        key_pem,
        fullchain_pem,
        pfx_base64: Some(pfx_base64),
        nginx_config,
        electron_readme,
    })
}

#[tauri::command]
pub fn issue_server_cert(
    app_handle: tauri::AppHandle,
    store: tauri::State<'_, CertBundleStore>,
    request: IssueCertRequest,
) -> AppResult<CertBundlePreview> {
    let bundle = issue_server_cert_internal(&app_handle, request)?;
    store.insert(bundle)
}

fn generate_pfx_base64_internal(
    cert_pem: &str,
    key_pem: &str,
    ca_cert_pem: &str,
    common_name: &str,
    pfx_password: &str,
) -> AppResult<String> {
    let password = pfx_password.trim();
    if password.is_empty() {
        return Err(AppError::Custom(
            "请设置 PFX/PKCS#12 导出密码。".to_string(),
        ));
    }

    let cert = X509::from_pem(cert_pem.as_bytes())?;
    let key = PKey::private_key_from_pem(key_pem.as_bytes())?;
    let ca_cert = X509::from_pem(ca_cert_pem.as_bytes())?;

    let mut ca_stack = Stack::new()?;
    ca_stack.push(ca_cert)?;

    let mut builder = Pkcs12::builder();
    builder
        .name(common_name)
        .pkey(&key)
        .cert(&cert)
        .ca(ca_stack);

    let pkcs12 = builder.build2(password)?;
    let der = pkcs12.to_der()?;

    Ok(general_purpose::STANDARD.encode(der))
}

fn generate_nginx_config_internal(domain: &str) -> String {
    format!(
        r#"server {{
    listen 443 ssl http2;
    server_name {};

    # 请将证书及密钥文件放置在合适的系统路径，并在此配置
    ssl_certificate /etc/nginx/ssl/fullchain.pem;
    ssl_certificate_key /etc/nginx/ssl/server.key;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    location / {{
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}
}}"#,
        domain
    )
}

fn generate_electron_readme_internal() -> String {
    r#"# Electron HTTPS 开发证书使用说明

当你在 Electron 中开发需要访问自签 HTTPS 服务时，由于证书是自签的，Electron 会拒绝连接并报错 `NET::ERR_CERT_AUTHORITY_INVALID`。

### 解决方法 1：环境变量 (推荐)

在启动 Electron 应用前，设置 `NODE_EXTRA_CA_CERTS` 环境变量，指定为你的自签根证书 `company-root-ca.crt` 的绝对路径。

#### macOS / Linux
```bash
export NODE_EXTRA_CA_CERTS="/path/to/company-root-ca.crt"
npm run dev
```

#### Windows (PowerShell)
```powershell
$env:NODE_EXTRA_CA_CERTS="C:\path\to\company-root-ca.crt"
npm run dev
```

#### Windows (CMD)
```cmd
set NODE_EXTRA_CA_CERTS=C:\path\to\company-root-ca.crt
npm run dev
```

---

### 解决方法 2：在 Electron 代码中忽略证书校验 (仅限开发环境)

在 Electron 的 `main.js` 或主进程初始化代码中，加入以下监听器：

```javascript
const { app } = require('electron');

app.on('certificate-error', (event, webContents, url, error, certificate, callback) => {
  // 仅在开发环境下允许特定的自签域名
  if (url.includes('company.com') || url.includes('internal')) {
    event.preventDefault();
    callback(true); // 接受证书
  } else {
    callback(false); // 拒绝
  }
});
```

> **安全提示**：请勿在生产环境使用 `callback(true)` 忽略所有证书错误，这会面临中间人攻击的风险。
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::stack::Stack as OpenSslStack;
    use openssl::x509::store::X509StoreBuilder;
    use openssl::x509::X509StoreContext;
    use rcgen::{BasicConstraints, CertificateParams, IsCa};

    #[test]
    fn nginx_config_uses_requested_domain_and_expected_files() {
        let config = generate_nginx_config_internal("api.internal.example.com");

        assert!(config.contains("server_name api.internal.example.com;"));
        assert!(config.contains("ssl_certificate /etc/nginx/ssl/fullchain.pem;"));
        assert!(config.contains("ssl_certificate_key /etc/nginx/ssl/server.key;"));
        assert!(config.contains("proxy_pass http://localhost:8080;"));
    }

    #[test]
    fn electron_readme_mentions_extra_ca_and_production_risk() {
        let readme = generate_electron_readme_internal();

        assert!(readme.contains("NODE_EXTRA_CA_CERTS"));
        assert!(readme.contains("请勿在生产环境使用"));
    }

    #[test]
    fn pfx_generation_requires_non_empty_password_before_parsing_inputs() {
        let err = generate_pfx_base64_internal("", "", "", "example.test", "   ")
            .expect_err("empty PFX password should fail");

        assert!(err.to_string().contains("请设置 PFX/PKCS#12 导出密码"));
    }

    fn valid_request() -> IssueCertRequest {
        IssueCertRequest {
            common_name: "api.internal.example.com".to_string(),
            dns_names: vec!["api.internal.example.com".to_string()],
            ip_addresses: vec!["127.0.0.1".to_string()],
            days: 365,
            organization: None,
            organizational_unit: None,
            country: Some("CN".to_string()),
            state: None,
            locality: None,
            pfx_password: "strong-password".to_string(),
        }
    }

    #[test]
    fn request_validation_rejects_nginx_directive_in_common_name() {
        let mut request = valid_request();
        request.common_name = "example.com; include /tmp/file".to_string();
        let err = validate_and_normalize_request(request).expect_err("injection should fail");
        assert!(err.to_string().contains("Common Name"));
    }

    #[test]
    fn request_validation_requires_common_name_in_sans() {
        let mut request = valid_request();
        request.dns_names = vec!["other.example.com".to_string()];
        let err = validate_and_normalize_request(request).expect_err("missing SAN should fail");
        assert!(err.to_string().contains("DNS SAN"));
    }

    #[test]
    fn request_validation_rejects_short_pfx_password() {
        let mut request = valid_request();
        request.pfx_password = "short".to_string();
        let err = validate_and_normalize_request(request).expect_err("short password should fail");
        assert!(err.to_string().contains("至少需要 8 个字符"));
    }

    #[test]
    fn issued_certificate_verifies_against_original_root_ca() {
        let ca_key = KeyPair::generate().unwrap();
        let mut ca_params = CertificateParams::default();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.not_before = OffsetDateTime::now_utc() - Duration::days(1);
        ca_params.not_after = OffsetDateTime::now_utc() + Duration::days(3650);
        let ca_cert = ca_params.self_signed(&ca_key).unwrap();
        let ca_cert_pem = ca_cert.pem();
        let ca_key_pem = ca_key.serialize_pem();

        let bundle = issue_server_cert_with_material(&ca_cert_pem, &ca_key_pem, valid_request())
            .expect("certificate issuance should succeed");

        let leaf = X509::from_pem(bundle.cert_pem.as_bytes()).unwrap();
        let root = X509::from_pem(ca_cert_pem.as_bytes()).unwrap();
        let mut store_builder = X509StoreBuilder::new().unwrap();
        store_builder.add_cert(root).unwrap();
        let store = store_builder.build();
        let chain = OpenSslStack::new().unwrap();
        let mut context = X509StoreContext::new().unwrap();

        let verified = context
            .init(&store, &leaf, &chain, |context| context.verify_cert())
            .unwrap();
        assert!(verified);

        let sans = leaf.subject_alt_names().unwrap();
        assert!(sans
            .iter()
            .any(|name| name.dnsname() == Some("api.internal.example.com")));
        assert!(sans
            .iter()
            .any(|name| name.ipaddress() == Some(&[127, 0, 0, 1])));
    }
}
