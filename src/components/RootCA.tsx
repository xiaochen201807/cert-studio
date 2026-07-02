import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ShieldCheck, Key, Calendar, Fingerprint, Upload, Plus, Download, RefreshCw, HelpCircle } from "lucide-react";

interface RootCAProps {
  hasRootCa: boolean;
  onCaChange: () => void;
}

interface RootCaInfo {
  subject: String;
  issuer: String;
  not_before: String;
  not_after: String;
  sha256_fingerprint: String;
}

const RootCA: React.FC<RootCAProps> = ({ hasRootCa, onCaChange }) => {
  const [caInfo, setCaInfo] = useState<RootCaInfo | null>(null);
  const [activeSubTab, setActiveSubTab] = useState<"create" | "import">("create");
  
  // 创建 CA 的表单状态
  const [cn, setCn] = useState("Company Root CA");
  const [org, setOrg] = useState("Company");
  const [days, setDays] = useState(3650);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // 导入 CA 的表单状态
  const [importCertPem, setImportCertPem] = useState("");
  const [importKeyPem, setImportKeyPem] = useState("");

  // 获取 CA 详细信息
  const fetchCaInfo = async () => {
    if (hasRootCa) {
      try {
        const info = await invoke<RootCaInfo>("get_root_ca_info");
        setCaInfo(info);
      } catch (e) {
        console.error("加载 CA 详细信息失败:", e);
      }
    } else {
      setCaInfo(null);
    }
  };

  useEffect(() => {
    fetchCaInfo();
  }, [hasRootCa]);

  // 创建全新 Root CA
  const handleCreateCa = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const info = await invoke<RootCaInfo>("create_root_ca", {
        commonName: cn,
        organization: org || null,
        days: days,
      });
      setCaInfo(info);
      onCaChange();
      alert("🎉 Root CA 根证书及私钥已成功创建！私钥已加密保护。");
    } catch (err) {
      alert("创建失败: " + err);
    } finally {
      setIsSubmitting(false);
    }
  };

  // 导入已有 Root CA
  const handleImportCa = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!importCertPem.trim() || !importKeyPem.trim()) {
      alert("请提供证书 PEM 文本与私钥 PEM 文本！");
      return;
    }
    setIsSubmitting(true);
    try {
      const info = await invoke<RootCaInfo>("import_root_ca", {
        certPem: importCertPem,
        keyPem: importKeyPem,
      });
      setCaInfo(info);
      onCaChange();
      alert("🎉 根证书与私钥已验证匹配并成功导入！");
    } catch (err) {
      alert("导入失败: " + err);
    } finally {
      setIsSubmitting(false);
    }
  };

  // 导入对话框 - 选择证书
  const handleSelectCertFile = async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{ name: "PEM 证书 (*.crt, *.pem)", extensions: ["crt", "pem", "cer"] }]
      });
      if (file && typeof file === "string") {
        const content = await invoke<string>("read_text_file", { path: file });
        setImportCertPem(content);
      }
    } catch (err) {
      alert("打开文件失败: " + err);
    }
  };

  // 导入对话框 - 选择私钥
  const handleSelectKeyFile = async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{ name: "PEM 私钥 (*.key, *.pem)", extensions: ["key", "pem"] }]
      });
      if (file && typeof file === "string") {
        const content = await invoke<string>("read_text_file", { path: file });
        setImportKeyPem(content);
      }
    } catch (err) {
      alert("打开文件失败: " + err);
    }
  };

  // 一键选择路径导出根证书
  const handleExportRootCert = async () => {
    try {
      const dir = await open({
        directory: true,
        multiple: false,
        title: "选择导出根证书的目录"
      });
      if (dir && typeof dir === "string") {
        await invoke("export_root_ca_cert", { outputDir: dir });
        alert(`根证书 (company-root-ca.crt) 已成功导出至：\n${dir}`);
      }
    } catch (err) {
      alert("导出失败: " + err);
    }
  };

  return (
    <div className="page-fade-in" style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      {/* 头部标题 */}
      <div>
        <h2 style={{ fontSize: "24px", fontWeight: 700, marginBottom: "6px" }}>Root CA 根证书管理</h2>
        <p style={{ color: "var(--text-secondary)", fontSize: "14px" }}>
          创建、导入、查看和导出您的 Root CA。私钥存储在硬件密钥环或本地专有加密文件中，杜绝明文落地。
        </p>
      </div>

      {hasRootCa && caInfo ? (
        /* ==================== 1. 展示 CA 证书信息 ==================== */
        <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
          <div className="glass-panel" style={{ padding: "30px", borderLeft: "4px solid var(--accent-neon)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "20px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <ShieldCheck size={24} style={{ color: "var(--accent-neon)" }} />
                <h3 style={{ fontSize: "18px", fontWeight: 600 }}>Root CA 已就绪</h3>
              </div>
              <span className="badge badge-active">受信任</span>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: "16px" }}>
              <div style={{ display: "flex", gap: "12px", borderBottom: "1px solid var(--border-glass)", paddingBottom: "12px" }}>
                <Key size={16} style={{ color: "var(--primary-neon)", marginTop: "2px", flexShrink: 0 }} />
                <div>
                  <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>颁发主体 (Subject)</div>
                  <div style={{ fontSize: "14px", fontWeight: 600, marginTop: "2px", wordBreak: "break-all" }}>{caInfo.subject}</div>
                </div>
              </div>

              <div style={{ display: "flex", gap: "12px", borderBottom: "1px solid var(--border-glass)", paddingBottom: "12px" }}>
                <RefreshCw size={16} style={{ color: "var(--secondary-neon)", marginTop: "2px", flexShrink: 0 }} />
                <div>
                  <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>颁发机构 (Issuer)</div>
                  <div style={{ fontSize: "14px", fontWeight: 600, marginTop: "2px", wordBreak: "break-all" }}>{caInfo.issuer}</div>
                </div>
              </div>

              <div style={{ display: "flex", gap: "12px", borderBottom: "1px solid var(--border-glass)", paddingBottom: "12px" }}>
                <Calendar size={16} style={{ color: "var(--accent-neon)", marginTop: "2px", flexShrink: 0 }} />
                <div>
                  <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>有效期范围 (Validity)</div>
                  <div style={{ fontSize: "13px", fontWeight: 500, marginTop: "2px" }}>
                    <span style={{ color: "var(--text-muted)" }}>From</span> {caInfo.not_before} 
                    <br />
                    <span style={{ color: "var(--text-muted)" }}>To</span> {caInfo.not_after}
                  </div>
                </div>
              </div>

              <div style={{ display: "flex", gap: "12px" }}>
                <Fingerprint size={16} style={{ color: "var(--primary-neon)", marginTop: "2px", flexShrink: 0 }} />
                <div>
                  <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>SHA256 指纹 (Fingerprint)</div>
                  <div style={{ fontSize: "12px", fontFamily: "monospace", color: "var(--text-secondary)", marginTop: "2px", wordBreak: "break-all" }}>
                    {caInfo.sha256_fingerprint}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div style={{ display: "flex", gap: "16px" }}>
            <button
              onClick={handleExportRootCert}
              style={{
                background: "var(--primary-theme)",
                color: "#fff",
                padding: "10px 20px",
                borderRadius: "6px",
                display: "flex",
                alignItems: "center",
                gap: "8px",
                fontSize: "13px",
                fontWeight: 500,
                boxShadow: "none"
              }}
            >
              <Download size={16} />
              <span>导出 Root CA 根证书</span>
            </button>

            <button
              onClick={() => {
                if (confirm("⚠️ 确定要重新生成或替换当前的 Root CA 吗？警告：重新生成会导致之前已发行的子证书全部失效，您必须重新下载和安装根证书。")) {
                  onCaChange(); // 强制刷新状态
                  setCaInfo(null);
                }
              }}
              style={{
                background: "#18181b",
                color: "var(--text-secondary)",
                padding: "10px 20px",
                borderRadius: "6px",
                border: "1px solid var(--border-subtle)",
                fontSize: "13px",
                fontWeight: 500,
              }}
            >
              重新初始化根 CA
            </button>
          </div>

          {/* 本地安装说明 */}
          <div className="glass-panel" style={{ padding: "24px" }}>
            <h4 style={{ fontSize: "15px", fontWeight: 600, marginBottom: "12px", display: "flex", alignItems: "center", gap: "6px" }}>
              <HelpCircle size={16} style={{ color: "#818cf8" }} />
              如何在本地安装受信任的根证书
            </h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", marginBottom: "14px", lineHeight: 1.5 }}>
              为了让您本地的 Chrome/Edge 等浏览器在访问自签 HTTPS 网站时不报错，您必须先通过上面的按钮导出根证书（`company-root-ca.crt`），并将其安装进系统受信任根证书存储区：
            </p>
            <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <div style={{ fontSize: "11px", background: "#09090b", padding: "10px", borderRadius: "6px", border: "1px solid var(--border-subtle)" }}>
                <span style={{ color: "#818cf8", fontWeight: 600 }}>Windows (管理员权限 PowerShell)</span>
                <code style={{ display: "block", marginTop: "4px", fontFamily: "monospace" }}>
                  certutil -addstore -f "Root" company-root-ca.crt
                </code>
              </div>
              <div style={{ fontSize: "11px", background: "#09090b", padding: "10px", borderRadius: "6px", border: "1px solid var(--border-subtle)" }}>
                <span style={{ color: "var(--accent-success)", fontWeight: 600 }}>macOS (管理员权限 Terminal)</span>
                <code style={{ display: "block", marginTop: "4px", fontFamily: "monospace" }}>
                  sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain company-root-ca.crt
                </code>
              </div>
            </div>
          </div>
        </div>
      ) : (
        /* ==================== 2. 未设置根证书 - 引导创建与导入 ==================== */
        <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
          {/* Tab 选项卡 */}
          <div style={{ display: "flex", gap: "10px", borderBottom: "1px solid var(--border-glass)", paddingBottom: "10px" }}>
            <button
              onClick={() => setActiveSubTab("create")}
              className={`menu-item-btn ${activeSubTab === "create" ? "active" : ""}`}
              style={{ width: "auto", padding: "10px 20px" }}
            >
              <Plus size={16} />
              <span>新建 Root CA</span>
            </button>
            <button
              onClick={() => setActiveSubTab("import")}
              className={`menu-item-btn ${activeSubTab === "import" ? "active" : ""}`}
              style={{ width: "auto", padding: "10px 20px" }}
            >
              <Upload size={16} />
              <span>导入已有 Root CA</span>
            </button>
          </div>

          {activeSubTab === "create" ? (
            /* 创建表单 */
            <form onSubmit={handleCreateCa} className="glass-panel" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "20px" }}>
              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                <label style={{ fontSize: "14px", color: "var(--text-secondary)", fontWeight: 500 }}>
                  常用名称 (Common Name) *
                </label>
                <input
                  type="text"
                  required
                  value={cn}
                  onChange={(e) => setCn(e.target.value)}
                  placeholder="例如: Company Internal Root CA"
                />
                <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>根证书的标识名称，建议使用代表您团队或公司的名字。</span>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                <label style={{ fontSize: "14px", color: "var(--text-secondary)", fontWeight: 500 }}>
                  组织名称 (Organization)
                </label>
                <input
                  type="text"
                  value={org}
                  onChange={(e) => setOrg(e.target.value)}
                  placeholder="例如: My Company Ltd."
                />
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                <label style={{ fontSize: "14px", color: "var(--text-secondary)", fontWeight: 500 }}>
                  有效期 (天数) *
                </label>
                <input
                  type="number"
                  required
                  value={days}
                  onChange={(e) => setDays(Number(e.target.value))}
                  placeholder="默认 3650 天 (10年)"
                />
              </div>

              <div style={{ paddingTop: "10px" }}>
                <button
                  type="submit"
                  disabled={isSubmitting}
                  style={{
                    background: "var(--primary-theme)",
                    color: "#fff",
                    padding: "10px 18px",
                    borderRadius: "6px",
                    fontSize: "13px",
                    fontWeight: 500,
                    boxShadow: "none",
                    width: "100%",
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center"
                  }}
                >
                  {isSubmitting ? "生成根证书中..." : "一键创建受信任 Root CA"}
                </button>
              </div>
            </form>
          ) : (
            /* 导入表单 */
            <form onSubmit={handleImportCa} className="glass-panel" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "20px" }}>
              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <label style={{ fontSize: "14px", color: "var(--text-secondary)", fontWeight: 500 }}>
                    根证书 PEM 文本 (cert.pem / root.crt) *
                  </label>
                  <button
                    type="button"
                    onClick={handleSelectCertFile}
                    style={{ background: "transparent", color: "#818cf8", fontSize: "12px", display: "flex", alignItems: "center", gap: "4px" }}
                  >
                    <Upload size={14} />
                    选择证书文件
                  </button>
                </div>
                <textarea
                  required
                  rows={6}
                  value={importCertPem}
                  onChange={(e) => setImportCertPem(e.target.value)}
                  placeholder="-----BEGIN CERTIFICATE-----&#10;...&#10;-----END CERTIFICATE-----"
                  style={{ background: "#09090b", color: "var(--text-primary)", border: "1px solid var(--border-subtle)", borderRadius: "8px", padding: "10px 14px", fontFamily: "monospace", resize: "vertical", outline: "none" }}
                />
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <label style={{ fontSize: "14px", color: "var(--text-secondary)", fontWeight: 500 }}>
                    CA 私钥 PEM 文本 (key.pem / private.key) *
                  </label>
                  <button
                    type="button"
                    onClick={handleSelectKeyFile}
                    style={{ background: "transparent", color: "#818cf8", fontSize: "12px", display: "flex", alignItems: "center", gap: "4px" }}
                  >
                    <Upload size={14} />
                    选择私钥文件
                  </button>
                </div>
                <textarea
                  required
                  rows={6}
                  value={importKeyPem}
                  onChange={(e) => setImportKeyPem(e.target.value)}
                  placeholder="-----BEGIN PRIVATE KEY-----&#10;...&#10;-----END PRIVATE KEY-----"
                  style={{ background: "#09090b", color: "var(--text-primary)", border: "1px solid var(--border-subtle)", borderRadius: "8px", padding: "10px 14px", fontFamily: "monospace", resize: "vertical", outline: "none" }}
                />
              </div>

              <div style={{ paddingTop: "10px" }}>
                <button
                  type="submit"
                  disabled={isSubmitting}
                  style={{
                    background: "var(--primary-theme)",
                    color: "#fff",
                    padding: "10px 18px",
                    borderRadius: "6px",
                    fontSize: "13px",
                    fontWeight: 500,
                    boxShadow: "none",
                    width: "100%",
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center"
                  }}
                >
                  {isSubmitting ? "校验并保存中..." : "校验并导入 Root CA"}
                </button>
              </div>
            </form>
          )}
        </div>
      )}
    </div>
  );
};

export default RootCA;
