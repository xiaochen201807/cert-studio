# Cert Studio

Cert Studio 是一个基于 Tauri 2、React 和 Rust 的本地证书工具，用于创建本地 Root CA、签发内部 HTTPS 服务端证书，并导出 Nginx、Electron、PFX/PKCS#12 等常见使用场景需要的配套文件。

> 适用场景：本地开发、内网服务、企业内部测试环境。不要把 Cert Studio 生成的 Root CA 当作公网可信 CA 使用。

## 功能概览

- 创建或导入本地 Root CA。
- 查看 Root CA 主题、签发者、有效期和 SHA-256 指纹。
- 将 Root CA 导入当前系统信任区。
- 导出 Root CA 公开证书。
- 使用密码加密导出 Root CA 备份包，支持迁移到另一台机器。
- 签发 HTTPS 服务端证书，支持 DNS SAN、IP SAN 和证书主体信息。
- 导出 `server.crt`、`server.key`、`server.pfx`、`fullchain.pem`、`nginx.conf`、`electron.md` 等文件。
- 对 Root CA 重新初始化、Root CA 导入、PFX 密码、服务端私钥导出提供明确的风险提示和校验。

## 快速开始

### 环境要求

- Node.js 18 或更高版本。
- Rust 工具链。
- 系统可用的 OpenSSL 构建依赖。项目使用 `openssl` 的 vendored 构建特性，通常不需要额外指定本机 OpenSSL 路径。

### 安装依赖

```bash
npm install
```

### 启动前端开发服务

```bash
npm run dev
```

### 启动 Tauri 桌面应用

```bash
npm run tauri dev
```

### 构建前端

```bash
npm run build
```

### 构建桌面安装包

```bash
npm run tauri build
```

## 基本使用流程

1. 打开 Root CA 页面。
2. 创建新的 Root CA，或导入已有 Root CA 的证书 PEM 和私钥 PEM。
3. 按需将 Root CA 导入系统信任区。
4. 切换到证书签发页面，填写 DNS SAN、Common Name、IP SAN、有效期和 PFX 导出密码。
5. 签发证书后预览 Nginx、Electron 或 PEM 内容。
6. 选择受控目录导出证书束。

## Root CA 管理

Root CA 私钥是整套信任链的核心。应用会优先把 Root CA 私钥写入系统密钥环；如果系统密钥环不可用，则回退到本地 AES-256-GCM 加密文件，并单独保存本地加密密钥。

重新初始化 Root CA 会替换当前 Root CA。旧 Root CA 签发过的服务端证书不会被新 Root CA 信任，客户端也需要重新安装新的根证书。应用会要求二次确认，并输入固定确认文本后才允许进入重新初始化流程。

### 备份与迁移

Root CA 备份包包含 Root CA 证书和私钥，使用 PBKDF2-HMAC-SHA256 派生密钥并通过 AES-256-GCM 加密。导出备份包时请使用强密码，并把备份包和密码分别保存在可信位置。

迁移到新机器时，在 Root CA 页面选择恢复备份包，输入备份密码即可恢复同一套 Root CA。

## 系统信任导入

Cert Studio 支持通过系统命令将 Root CA 导入当前系统信任区：

- macOS：调用 `security add-trusted-cert`。
- Windows：调用 `certutil -user -addstore Root`。
- Linux：按系统能力依次尝试 `update-ca-certificates`、`update-ca-trust extract`、`trust anchor --store`，并优先通过 `pkexec` 授权，缺失时回退到 `sudo`。

Linux 不同发行版的信任链工具差异较大。如果自动导入失败，请根据发行版文档手动安装导出的 `company-root-ca.crt`。

## 证书签发与导出

签发服务端证书时：

- DNS SAN 至少填写一个域名。
- Common Name 只能填写一个主域名，不支持逗号和通配符。
- IP SAN 可留空，但不能包含通配符。
- PFX/PKCS#12 导出密码不能为空。

导出的证书束包含：

- `server.crt`：服务端证书。
- `server.key`：服务端私钥。
- `server.pfx`：带密码保护的 PFX/PKCS#12 证书包。
- `fullchain.pem`：服务端证书和 Root CA 证书链。
- `company-root-ca.crt`：Root CA 公开证书。
- `nginx.conf`：Nginx 配置示例。
- `electron.md`：Electron 开发接入说明。

## 安全说明

- 不要把 Root CA 私钥、Root CA 备份包、`server.key` 或 PFX 密码提交到 Git。
- 不要上传私钥到公共制品库、日志系统、聊天工具或工单附件。
- `company-root-ca.crt` 是公开证书，可以分发给需要信任该 Root CA 的客户端。
- `server.pfx` 虽然有密码保护，仍应按敏感文件处理。
- Electron 中忽略证书错误的方案仅限开发环境，不要在生产环境使用全局跳过证书校验。

## 测试与校验

前端构建：

```bash
npm run build
```

Rust 单元测试：

```bash
cd src-tauri
cargo test
```

提交前建议额外执行：

```bash
git diff --check
```

## 项目文档

- 路线图：[RLOADMAP.md](./RLOADMAP.md)
- GitHub Wiki：用于维护更完整的安装、发布、运维和任务拆分说明。
- GitHub Pages：https://xiaochen201807.github.io/cert-studio/
