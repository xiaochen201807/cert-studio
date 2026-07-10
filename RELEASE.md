# Cert Studio 发布与签名

## 版本与制品

发布标签必须使用 `vMAJOR.MINOR.PATCH` 格式，并与以下三个文件保持一致：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

发布工作流使用 npm 和 Cargo 锁文件构建 Windows x64/ARM64、macOS Universal 与 Linux x64 制品，并生成 `SHA256SUMS`。

## macOS 签名与公证

在 GitHub Actions Secrets 中配置：

- `APPLE_CERTIFICATE`：Developer ID Application 证书的 Base64 内容。
- `APPLE_CERTIFICATE_PASSWORD`：证书导出密码。
- `APPLE_SIGNING_IDENTITY`：Developer ID Application 身份名称。
- `APPLE_ID`：用于公证的 Apple ID。
- `APPLE_PASSWORD`：Apple ID 的 app-specific password。
- `APPLE_TEAM_ID`：Apple Developer Team ID。

工作流已经把这些 Secrets 传给 Tauri bundler。缺少任一必要值时，macOS 产物应按未签名内部测试包处理。

发布后在 macOS 上验证：

```bash
codesign --verify --deep --strict --verbose=2 /Applications/cert-studio.app
spctl --assess --type execute --verbose=2 /Applications/cert-studio.app
```

## Windows Authenticode

Windows 签名需要组织购买的代码签名证书或 Azure Trusted Signing。将证书安装到 Actions runner 后，在 Tauri `bundle.windows` 中配置 `certificateThumbprint`、`digestAlgorithm` 和可信时间戳 URL；若使用 Azure Trusted Signing，则配置 Tauri 的自定义 `signCommand`。

签名证书、PFX 密码和云签名凭据只能放在 GitHub Environments/Secrets 中，不得写入仓库或工作流明文。配置完成后用 PowerShell 验证：

```powershell
Get-AuthenticodeSignature .\cert-studio.exe | Format-List Status,StatusMessage,SignerCertificate
```

只有 `Status` 为 `Valid` 的制品才能对外标记为已签名版本。
