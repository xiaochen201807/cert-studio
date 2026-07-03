import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FileSpreadsheet, ShieldAlert, ChevronDown, ChevronUp, Download, Eye, Terminal, Code } from "lucide-react";

interface IssueCertProps {
  hasRootCa: boolean;
  onNavigate: (tab: string) => void;
}

interface CertBundle {
  cert_pem: string;
  key_pem: string;
  fullchain_pem: string;
  pfx_base64: string | null;
  nginx_config: string;
  electron_readme: string;
}

const IssueCert: React.FC<IssueCertProps> = ({ hasRootCa, onNavigate }) => {
  // 表单状态
  const [cn, setCn] = useState("pdf.internal.company.com");
  const [dnsInput, setDnsInput] = useState("pdf.internal.company.com, localhost");
  const [ipInput, setIpInput] = useState("127.0.0.1");
  const [days, setDays] = useState(365);
  const [pfxPassword, setPfxPassword] = useState("");
  
  // 额外的高级 DN 属性
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [org, setOrg] = useState("Company");
  const [ou, setOu] = useState("DevOps");
  const [country, setCountry] = useState("CN");
  const [state, setState] = useState("Beijing");
  const [locality, setLocality] = useState("Beijing");

  const [isIssuing, setIsIssuing] = useState(false);
  const [bundle, setBundle] = useState<CertBundle | null>(null);

  // 签发成功后的视图 Tab
  const [activeViewTab, setActiveViewTab] = useState<"nginx" | "electron" | "cert">("nginx");

  // 执行签发
  const handleIssue = async (e: React.FormEvent) => {
    e.preventDefault();

    // 校验常用名称
    const trimmedCn = cn.trim();
    if (trimmedCn.includes(",") || trimmedCn.includes("，")) {
      alert("常用名称 (Common Name) 只能填写一个，不能包含逗号！");
      return;
    }
    if (trimmedCn.includes("*")) {
      alert("常用名称 (Common Name) 不支持通配符！");
      return;
    }

    // 解析 DNS Names 与 IP 地址
    const dnsNames = dnsInput
      .split(",")
      .map((d) => d.trim())
      .filter((d) => d.length > 0);

    const ipAddresses = ipInput
      .split(",")
      .map((ip: string) => ip.trim())
      .filter((ip) => ip.length > 0);

    // 校验 IP 地址是否包含通配符
    if (ipAddresses.some((ip: string) => ip.includes("*"))) {
      alert("IP 使用者备用名称 (IP SANs) 不支持通配符！");
      return;
    }
    if (!pfxPassword.trim()) {
      alert("请设置 PFX/PKCS#12 导出密码，用于保护 server.pfx 中的私钥。");
      return;
    }

    setIsIssuing(true);
    setBundle(null);

    try {
      const res = await invoke<CertBundle>("issue_server_cert", {
        request: {
          common_name: cn,
          dns_names: dnsNames,
          ip_addresses: ipAddresses,
          days: days,
          organization: org || null,
          organizational_unit: ou || null,
          country: country || null,
          state: state || null,
          locality: locality || null,
          pfx_password: pfxPassword,
        },
      });
      setBundle(res);
      alert("🎉 服务端 HTTPS 证书签发成功！可在下方预览配置并一键导出。");
    } catch (err) {
      alert("签发失败: " + err);
    } finally {
      setIsIssuing(false);
    }
  };

  // 一键选择路径导出 Bundle
  const handleExportBundle = async () => {
    if (!bundle) return;
    try {
      const dir = await open({
        directory: true,
        multiple: false,
        title: "选择导出证书的保存目录"
      });
      if (dir && typeof dir === "string") {
        await invoke("export_cert_bundle", {
          bundle: bundle,
          outputDir: dir
        });
        alert(`🎉 证书束已成功导出！\n\n导出目录包含以下文件：\n- server.crt (证书)\n- server.key (私钥)\n- server.pfx (PFX/PKCS#12 证书包)\n- fullchain.pem (含根的完整链)\n- company-root-ca.crt (根证书)\n- nginx.conf (Nginx 配置示例)\n- electron.md (Electron 接入说明)\n\nserver.pfx 使用签发时设置的 PFX 密码保护。\n\n路径: ${dir}`);
      }
    } catch (err) {
      alert("导出失败: " + err);
    }
  };

  if (!hasRootCa) {
    return (
      /* ==================== 1. 未配置根 CA 的警告界面 ==================== */
      <div className="page-fade-in glass-panel" style={{ padding: "40px", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "20px", textAlign: "center" }}>
        <div style={{ padding: "16px", borderRadius: "50%", background: "hsla(318, 100%, 62%, 0.12)", color: "var(--secondary-neon)" }}>
          <ShieldAlert size={48} />
        </div>
        <h2 style={{ fontSize: "22px", fontWeight: 700 }}>需要先初始化 Root CA</h2>
        <p style={{ color: "var(--text-secondary)", fontSize: "14px", maxWidth: "500px", lineHeight: 1.5 }}>
          签发服务端 SSL 证书前，必须先生成或导入您的 Root CA 根证书。这是由于服务端证书必须由该根证书签名才能生效。
        </p>
        <button
          onClick={() => onNavigate("rootca")}
          style={{
            background: "linear-gradient(135deg, var(--secondary-neon), #db2777)",
            color: "#fff",
            padding: "12px 24px",
            borderRadius: "10px",
            fontSize: "14px",
            fontWeight: 600,
            boxShadow: "var(--glow-shadow-pink)",
            marginTop: "10px"
          }}
        >
          立即前往配置 Root CA
        </button>
      </div>
    );
  }

  return (
    /* ==================== 2. 已有根 CA 的签发界面 ==================== */
    <div className="page-fade-in" style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      {/* 头部标题 */}
      <div>
        <h2 style={{ fontSize: "24px", fontWeight: 700, marginBottom: "6px" }}>签发 HTTPS 服务端证书</h2>
        <p style={{ color: "var(--text-secondary)", fontSize: "14px" }}>
          填写服务端域名或局域网 IP，利用您的本地 Root CA 快速签署高安全强度的证书。
        </p>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: bundle ? "1fr 1.2fr" : "1fr", gap: "28px", alignItems: "start" }}>
        {/* 左侧：输入表单 */}
        <form onSubmit={handleIssue} className="glass-panel" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "20px" }}>
          <h3 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid var(--border-glass)", paddingBottom: "12px", display: "flex", alignItems: "center", gap: "8px" }}>
            <FileSpreadsheet size={20} style={{ color: "var(--primary-neon)" }} />
            <span>证书申请参数</span>
          </h3>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>
              DNS 使用者备用名称 (DNS SANs) *
            </label>
            <input
              type="text"
              required
              value={dnsInput}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDnsInput(e.target.value)}
              placeholder="多个域名用英文逗号分隔"
            />
            <span style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: "1.4" }}>
              允许通过域名访问此服务的列表。支持填写多个（用英文逗号分隔），且支持通配符域名（如 *.internal.company.com）。
            </span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>
              常用名称 (Common Name) *
            </label>
            <input
              type="text"
              required
              value={cn}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCn(e.target.value)}
              placeholder="例如: pdf.internal.company.com"
            />
            <span style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: "1.4" }}>
              证书关联的单个主域名（如 pdf.internal.company.com）。只能填写一个，且不支持通配符。为兼容老旧系统，一般填写您在“DNS 使用者备用名称”中设置的主域名。
            </span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>
              IP 使用者备用名称 (IP SANs)
            </label>
            <input
              type="text"
              value={ipInput}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setIpInput(e.target.value)}
              placeholder="多个 IP 用英文逗号分隔"
            />
            <span style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: "1.4" }}>
              允许通过 IP 直接访问此服务的列表（如 127.0.0.1, 10.0.0.5）。支持填写多个（用英文逗号分隔），但不支持通配符。若只用域名访问可留空。
            </span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>
              有效期 (天数) *
            </label>
            <input
              type="number"
              required
              value={days}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDays(Number(e.target.value))}
              placeholder="默认 365 天"
            />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ fontSize: "13px", color: "var(--text-secondary)", fontWeight: 500 }}>
              PFX/PKCS#12 导出密码 *
            </label>
            <input
              type="password"
              required
              value={pfxPassword}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPfxPassword(e.target.value)}
              placeholder="用于保护 server.pfx 中的私钥"
            />
            <span style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: "1.4" }}>
              导出的 server.pfx 会使用该密码保护，适合导入 Windows、IIS、.NET 或其他需要 PFX/PKCS#12 的工具链。请自行妥善保存该密码。
            </span>
          </div>

          {/* 高级属性折叠 */}
          <div>
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              style={{ background: "transparent", color: "var(--text-secondary)", fontSize: "13px", display: "flex", alignItems: "center", gap: "6px" }}
            >
              {showAdvanced ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              <span>{showAdvanced ? "折叠高级证书主体信息" : "展开高级证书主体信息"}</span>
            </button>

            {showAdvanced && (
              <div style={{ display: "flex", flexDirection: "column", gap: "14px", marginTop: "14px", padding: "16px", background: "#09090b", borderRadius: "6px", border: "1px solid var(--border-subtle)" }}>
                <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                  <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>组织 (Organization)</label>
                  <input type="text" value={org} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setOrg(e.target.value)} style={{ padding: "8px 12px" }} />
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                  <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>部门 (Organizational Unit)</label>
                  <input type="text" value={ou} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setOu(e.target.value)} style={{ padding: "8px 12px" }} />
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                  <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>国家代码 (Country)</label>
                  <input type="text" value={country} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCountry(e.target.value)} maxLength={2} placeholder="CN" style={{ padding: "8px 12px" }} />
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                  <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>省/直辖市 (State)</label>
                  <input type="text" value={state} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setState(e.target.value)} style={{ padding: "8px 12px" }} />
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                  <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>城市 (Locality)</label>
                  <input type="text" value={locality} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setLocality(e.target.value)} style={{ padding: "8px 12px" }} />
                </div>
              </div>
            )}
          </div>

          <div style={{ paddingTop: "10px" }}>
            <button
              type="submit"
              disabled={isIssuing}
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
              {isIssuing ? "正在用本地根证书签名..." : "一键签署服务端 SSL 证书"}
            </button>
          </div>
        </form>

        {/* 右侧：签发成功展示 */}
        {bundle && (
          <div className="glass-panel page-fade-in" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "20px" }}>
            <h3 style={{ fontSize: "18px", fontWeight: 600, display: "flex", alignItems: "center", gap: "8px" }}>
              <Eye size={20} style={{ color: "var(--accent-success)" }} />
              <span>证书签发成果</span>
            </h3>

            {/* 顶栏 Tab 选择 */}
            <div style={{ display: "flex", gap: "8px", borderBottom: "1px solid var(--border-glass)", paddingBottom: "8px" }}>
              <button
                onClick={() => setActiveViewTab("nginx")}
                className={`menu-item-btn ${activeViewTab === "nginx" ? "active" : ""}`}
                style={{ width: "auto", padding: "8px 14px", fontSize: "12px" }}
              >
                <Code size={14} />
                <span>Nginx 配置</span>
              </button>
              <button
                onClick={() => setActiveViewTab("electron")}
                className={`menu-item-btn ${activeViewTab === "electron" ? "active" : ""}`}
                style={{ width: "auto", padding: "8px 14px", fontSize: "12px" }}
              >
                <Terminal size={14} />
                <span>Electron 接入</span>
              </button>
              <button
                onClick={() => setActiveViewTab("cert")}
                className={`menu-item-btn ${activeViewTab === "cert" ? "active" : ""}`}
                style={{ width: "auto", padding: "8px 14px", fontSize: "12px" }}
              >
                <FileSpreadsheet size={14} />
                <span>证书 PEM</span>
              </button>
            </div>

            {/* 内容预览框 */}
            <div style={{ background: "#09090b", border: "1px solid var(--border-subtle)", borderRadius: "6px", padding: "16px", flex: 1, minHeight: "260px", maxHeight: "360px", overflowY: "auto" }}>
              {activeViewTab === "nginx" && (
                <pre style={{ margin: 0, fontSize: "11px", fontFamily: "monospace", color: "var(--text-secondary)", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
                  {bundle.nginx_config}
                </pre>
              )}
              {activeViewTab === "electron" && (
                <pre style={{ margin: 0, fontSize: "11px", fontFamily: "monospace", color: "var(--text-secondary)", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
                  {bundle.electron_readme}
                </pre>
              )}
              {activeViewTab === "cert" && (
                <pre style={{ margin: 0, fontSize: "10px", fontFamily: "monospace", color: "var(--text-secondary)", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
                  {bundle.cert_pem}
                </pre>
              )}
            </div>

            {/* 导出按钮 */}
            <div>
              <button
                onClick={handleExportBundle}
                style={{
                  background: "#10b981",
                  color: "#fff",
                  padding: "10px 18px",
                  borderRadius: "6px",
                  fontSize: "13px",
                  fontWeight: 500,
                  boxShadow: "none",
                  width: "100%",
                  display: "flex",
                  justifyContent: "center",
                  alignItems: "center",
                  gap: "8px"
                }}
              >
                <Download size={16} />
                <span>一键导出证书及配套配置</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default IssueCert;
