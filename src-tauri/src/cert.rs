use crate::error::{AppError, AppResult};
use crate::storage;
use rcgen::{
    CertificateParams, KeyPair, DistinguishedName, DnType, SanType, IsCa,
    KeyUsagePurpose, ExtendedKeyUsagePurpose,
};
use rcgen::string::Ia5String;
use time::{OffsetDateTime, Duration};
use std::net::IpAddr;

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
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CertBundle {
    pub cert_pem: String,
    pub key_pem: String,
    pub fullchain_pem: String,
    pub pfx_base64: Option<String>,
    pub nginx_config: String,
    pub electron_readme: String,
}

pub fn issue_server_cert_internal(
    app_handle: &tauri::AppHandle,
    request: IssueCertRequest,
) -> AppResult<CertBundle> {
    // 1. 读取 CA 证书和私钥
    let ca_cert_pem = storage::get_root_ca_cert(app_handle)?;
    let ca_key_pem = storage::get_root_ca_key(app_handle)?;

    let ca_params = CertificateParams::from_ca_cert_pem(&ca_cert_pem)?;
    let ca_key_pair = KeyPair::from_pem(&ca_key_pem)?;
    let ca_cert = ca_params.self_signed(&ca_key_pair)?;

    // 2. 构造子证书参数
    // 在 rcgen 中使用首个 dns name 作为基础构建 params，如果没有则使用 CN 构建
    let primary_dns = request.dns_names.first().cloned().unwrap_or_else(|| request.common_name.clone());
    let mut params = CertificateParams::new(vec![primary_dns])?;

    // 组装 Subject Alternative Names
    let mut subject_alt_names = Vec::new();
    for dns in &request.dns_names {
        let ia5 = Ia5String::try_from(dns.as_str())
            .map_err(|e| AppError::Custom(format!("非法的 DNS 域名 '{}': {}", dns, e)))?;
        subject_alt_names.push(SanType::DnsName(ia5));
    }
    for ip in &request.ip_addresses {
        let ip_addr: IpAddr = ip.parse()
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
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(request.days as i64);
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::ClientAuth,
    ];

    // 生成子证书密钥对并用 CA 签名
    let server_key_pair = KeyPair::generate()?;
    let signed_cert = params.signed_by(&server_key_pair, &ca_cert, &ca_key_pair)?;

    let cert_pem = signed_cert.pem();
    let key_pem = server_key_pair.serialize_pem();

    // 拼接 fullchain
    let fullchain_pem = format!("{}\n{}", cert_pem, ca_cert_pem);

    // nginx 配置文件生成
    let nginx_config = generate_nginx_config_internal(&request.common_name);

    // Electron README
    let electron_readme = generate_electron_readme_internal();

    Ok(CertBundle {
        cert_pem,
        key_pem,
        fullchain_pem,
        pfx_base64: None,
        nginx_config,
        electron_readme,
    })
}

#[tauri::command]
pub fn issue_server_cert(
    app_handle: tauri::AppHandle,
    request: IssueCertRequest,
) -> AppResult<CertBundle> {
    issue_server_cert_internal(&app_handle, request)
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
