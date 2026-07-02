下面这份 roadmap 可以直接丢给 Codex Code 作为开发任务说明。

# Tauri + rcgen 证书工具开发 Roadmap

## 目标

开发一个跨平台桌面工具，用于公司内部统一管理自签 Root CA，并一键签发服务端 HTTPS 证书。

技术栈：

```txt
Tauri v2
Rust
rcgen
React / Vue / Svelte 任一前端
GitHub Actions
```

## MVP 功能

### 1. Root CA 管理

支持：

```txt
创建新的 Root CA
导入已有 Root CA 证书
导入已有 Root CA 私钥
查看 Root CA 信息
导出 Root CA 证书
```

注意：

```txt
Root CA 私钥不能默认打包进应用
Root CA 私钥默认只保存在用户本机安全目录
需要加密存储
```

Rust 侧建议模块：

```txt
src-tauri/src/ca.rs
```

核心命令：

```rust
create_root_ca()
import_root_ca(cert_pem, key_pem)
get_root_ca_info()
export_root_ca_cert()
```

---

### 2. 服务端证书签发

输入：

```txt
Common Name
DNS Names
IP Addresses
有效期天数
组织名
部门名
国家/省/城市
```

输出：

```txt
server.crt
server.key
fullchain.pem
server.pfx
nginx.conf 示例
```

Rust 侧模块：

```txt
src-tauri/src/cert.rs
```

核心命令：

```rust
issue_server_cert(request)
export_cert_bundle(bundle, output_dir)
generate_nginx_config(domain)
```

---

### 3. 前端页面

建议页面：

```txt
Dashboard
Root CA 管理
签发证书
证书详情
导出配置
设置
```

MVP 流程：

```txt
启动应用
检测是否已有 Root CA
没有则提示创建或导入
进入签发页面
填写域名/IP
点击签发
选择导出目录
生成证书和 nginx 配置
```

---

### 4. 安全设计

必须遵守：

```txt
不要把 Root CA 私钥提交到 git
不要把 Root CA 私钥打包进应用
不要在日志里打印私钥
不要把私钥明文长期保存
```

建议：

```txt
Root CA 私钥用系统 keyring 或本地加密文件保存
导出私钥时需要二次确认
应用启动时校验证书和私钥是否匹配
```

Rust crate 建议：

```toml
rcgen = "0.13"
rustls-pemfile = "2"
x509-parser = "0.16"
time = "0.3"
serde = { version = "1", features = ["derive"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
keyring = "3"
```

---

## 项目结构建议

```txt
cert-studio/
  .github/
    workflows/
      release.yml
  src/
    pages/
      Dashboard.tsx
      RootCA.tsx
      IssueCert.tsx
      Settings.tsx
    components/
      CertInfoCard.tsx
      DomainInput.tsx
      ExportPanel.tsx
  src-tauri/
    src/
      main.rs
      ca.rs
      cert.rs
      storage.rs
      export.rs
      error.rs
    Cargo.toml
    tauri.conf.json
```

---

## Rust 数据结构

```rust
#[derive(serde::Deserialize)]
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

#[derive(serde::Serialize)]
pub struct CertBundle {
    pub cert_pem: String,
    pub key_pem: String,
    pub fullchain_pem: String,
    pub pfx_base64: Option<String>,
    pub nginx_config: String,
}
```

---

## 开发阶段

### Phase 1：项目初始化

任务：

```txt
创建 Tauri v2 项目
配置前端框架
配置 Rust 基础模块
实现 Tauri command 调用
```

验收：

```txt
前端能调用 Rust hello command
应用能在 Windows 本地运行
```

---

### Phase 2：Root CA 创建

任务：

```txt
用 rcgen 创建 Root CA
生成 PEM 格式证书和私钥
保存到本地应用目录
展示证书 Subject、Issuer、有效期、指纹
```

验收：

```txt
点击创建 Root CA 后，可看到 Root CA 信息
可导出 company-root-ca.crt
```

---

### Phase 3：服务证书签发

任务：

```txt
读取 Root CA 证书和私钥
根据用户输入生成服务端证书
支持 DNS SAN
支持 IP SAN
生成 server.crt/server.key/fullchain.pem
```

验收：

```txt
输入 pdf.internal.company.com 和 127.0.0.1
可生成可被 Root CA 验证的服务证书
```

---

### Phase 4：导出功能

任务：

```txt
选择导出目录
导出 crt/key/fullchain
生成 nginx 配置
生成 Electron NODE_EXTRA_CA_CERTS 使用说明
```

验收：

```txt
导出目录包含：
server.crt
server.key
fullchain.pem
company-root-ca.crt
nginx.conf
electron.md
```

---

### Phase 5：本机安装 Root CA

可选但很实用。

Windows：

```powershell
certutil -addstore -f "Root" company-root-ca.crt
```

macOS：

```bash
security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain company-root-ca.crt
```

Linux：

```bash
sudo cp company-root-ca.crt /usr/local/share/ca-certificates/company-root-ca.crt
sudo update-ca-certificates
```

注意：这一步需要管理员权限。

---

### Phase 6：GitHub Actions 构建

目标平台：

```txt
Windows x64
Windows arm64
macOS x64
macOS arm64
Linux x64
Linux arm64
```

先做：

```txt
windows-latest
macos-latest
ubuntu-latest
```

后续再扩展 arm64。

workflow：

```yaml
name: release

on:
  push:
    tags:
      - "v*"

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: windows-latest
          - platform: macos-latest
          - platform: ubuntu-latest

    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - run: npm ci

      - run: npm run tauri build

      - uses: actions/upload-artifact@v4
        with:
          name: cert-studio-${{ matrix.platform }}
          path: |
            src-tauri/target/release/bundle/**
```

---

## Codex Code 初始提示词

可以直接用：

```txt
请基于 Tauri v2 + React + Rust 创建一个桌面应用 cert-studio。

目标是公司内部证书签发工具，底层使用 rcgen。

请实现以下 MVP：

1. Root CA 管理：
- 创建 Root CA
- 导入 Root CA PEM 证书和私钥
- 查看 Root CA subject、issuer、有效期、SHA256 指纹
- 导出 Root CA 证书

2. 服务端证书签发：
- 输入 common_name、dns_names、ip_addresses、days、organization 等
- 使用 Root CA 签发服务端证书
- 输出 server.crt、server.key、fullchain.pem
- 生成 nginx.conf 示例
- 生成 Electron NODE_EXTRA_CA_CERTS 使用说明

3. 数据安全：
- Root CA 私钥不要写入日志
- 默认保存在 Tauri app data 目录
- 预留 keyring 加密存储接口

4. GitHub Actions：
- 添加 release.yml
- 支持 Windows、macOS、Linux 构建

请按模块组织代码：
src-tauri/src/ca.rs
src-tauri/src/cert.rs
src-tauri/src/storage.rs
src-tauri/src/export.rs
src-tauri/src/error.rs

前端提供三个页面：
Dashboard
RootCA
IssueCert
```
