import React, { useState } from "react";
import { ShieldAlert, ShieldCheck, ArrowRight, Shield, Key, FileText, Lock } from "lucide-react";

interface DashboardProps {
  hasRootCa: boolean;
  onNavigate: (tab: string) => void;
}

const Dashboard: React.FC<DashboardProps> = ({ hasRootCa, onNavigate }) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div className="page-fade-in" style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      {/* 欢迎模块 */}
      <div className="glass-panel" style={{ padding: "40px", position: "relative", overflow: "hidden" }}>
        <div style={{ position: "relative", zIndex: 2 }}>
          <h1 style={{ fontSize: "30px", fontWeight: 600, letterSpacing: "-0.02em", marginBottom: "12px", color: "var(--text-primary)" }}>
            欢迎使用 Cert Studio
          </h1>
          <p style={{ color: "var(--text-secondary)", fontSize: "15px", maxWidth: "600px", lineHeight: 1.6 }}>
            企业级自签本地 Root CA 管理与 HTTPS 服务端证书一键签发工具。符合安全规范，零依赖一键本地部署。
          </p>
        </div>
        <div 
          style={{ 
            position: "absolute", 
            right: "-20px", 
            bottom: "-40px", 
            opacity: 0.04, 
            color: "var(--text-secondary)",
            transform: "rotate(-15deg)"
          }}
        >
          <Shield size={240} />
        </div>
      </div>

      {/* 状态快捷操作卡 */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "24px" }}>
        {/* CA 引导卡 */}
        <div className="glass-panel" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "16px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            {hasRootCa ? (
              <div style={{ padding: "8px", borderRadius: "8px", background: "rgba(16, 185, 129, 0.08)", color: "var(--accent-success)" }}>
                <ShieldCheck size={20} />
              </div>
            ) : (
              <div style={{ padding: "8px", borderRadius: "8px", background: "rgba(239, 68, 68, 0.08)", color: "var(--accent-danger)" }}>
                <ShieldAlert size={20} />
              </div>
            )}
            <h3 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)" }}>自签 Root CA 状态</h3>
          </div>
          
          <p style={{ color: "var(--text-secondary)", fontSize: "13px", lineHeight: 1.5 }}>
            {hasRootCa 
              ? "本地根证书已初始化完毕，系统已获得自签名信任锚点，可立即为您签发安全的 HTTPS 服务端证书。" 
              : "检测到您当前尚未配置 Root CA 证书。请先创建全新的根证书或导入公司已有的 CA 证书和私钥。"}
          </p>

          <div style={{ marginTop: "auto", paddingTop: "16px" }}>
            {hasRootCa ? (
              <button 
                onClick={() => onNavigate("issue")}
                onMouseEnter={() => setIsHovered(true)}
                onMouseLeave={() => setIsHovered(false)}
                style={{ 
                  background: isHovered ? "var(--primary-theme-hover)" : "var(--primary-theme)", 
                  color: "#fff", 
                  padding: "10px 18px", 
                  borderRadius: "6px", 
                  display: "flex", 
                  alignItems: "center", 
                  gap: "8px",
                  fontSize: "13px",
                  fontWeight: 500,
                  boxShadow: "none"
                }}
              >
                <span>立即签发 HTTPS 证书</span>
                <ArrowRight size={14} />
              </button>
            ) : (
              <button 
                onClick={() => onNavigate("rootca")}
                onMouseEnter={() => setIsHovered(true)}
                onMouseLeave={() => setIsHovered(false)}
                style={{ 
                  background: isHovered ? "var(--primary-theme-hover)" : "var(--primary-theme)", 
                  color: "#fff", 
                  padding: "10px 18px", 
                  borderRadius: "6px", 
                  display: "flex", 
                  alignItems: "center", 
                  gap: "8px",
                  fontSize: "13px",
                  fontWeight: 500,
                  boxShadow: "none"
                }}
              >
                <span>初始化 Root CA</span>
                <ArrowRight size={14} />
              </button>
            )}
          </div>
        </div>

        {/* 核心特性展示 */}
        <div className="glass-panel" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "20px" }}>
          <h3 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)" }}>Cert Studio 核心优势</h3>
          
          <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
            <div style={{ display: "flex", gap: "12px" }}>
              <Lock size={15} style={{ color: "#818cf8", marginTop: "3px", flexShrink: 0 }} />
              <div>
                <h4 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "2px", color: "var(--text-primary)" }}>系统级硬件密钥环加密</h4>
                <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.4 }}>私钥优先委托系统 Keyring 加密托管，绝对不向普通磁盘落地明文私钥。</p>
              </div>
            </div>

            <div style={{ display: "flex", gap: "12px" }}>
              <Key size={15} style={{ color: "var(--accent-success)", marginTop: "3px", flexShrink: 0 }} />
              <div>
                <h4 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "2px", color: "var(--text-primary)" }}>完整 SANs 多域名支持</h4>
                <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.4 }}>同时支持多 DNS 域名与 IP 绑定，无缝匹配 Chrome 现代浏览器的校验规范。</p>
              </div>
            </div>

            <div style={{ display: "flex", gap: "12px" }}>
              <FileText size={15} style={{ color: "#f43f5e", marginTop: "3px", flexShrink: 0 }} />
              <div>
                <h4 style={{ fontSize: "13px", fontWeight: 600, marginBottom: "2px", color: "var(--text-primary)" }}>自动化配套导出</h4>
                <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.4 }}>一键导出全链证书、私钥，并自动配套生成 Nginx 服务器和 Electron 开发环境的配置指南。</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 极简使用流程引导 */}
      <div className="glass-panel" style={{ padding: "30px" }}>
        <h3 style={{ fontSize: "16px", fontWeight: 600, marginBottom: "20px", color: "var(--text-primary)" }}>3 步开始开发测试</h3>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "24px" }}>
          <div style={{ background: "#09090b", padding: "20px", borderRadius: "8px", border: "1px solid var(--border-subtle)" }}>
            <div style={{ fontSize: "24px", fontWeight: 700, color: "#818cf8", marginBottom: "8px", letterSpacing: "-0.02em" }}>01</div>
            <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "6px", color: "var(--text-primary)" }}>初始化 Root CA</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.5 }}>点击 Root CA 导航，创建或者导入根证书。导出根证书并安装到您的操作系统“受信任的根证书颁发机构”中。</p>
          </div>
          <div style={{ background: "#09090b", padding: "20px", borderRadius: "8px", border: "1px solid var(--border-subtle)" }}>
            <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--accent-success)", marginBottom: "8px", letterSpacing: "-0.02em" }}>02</div>
            <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "6px", color: "var(--text-primary)" }}>签发服务端证书</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.5 }}>输入您局域网开发使用的域名（如 *.company.com）或 IP（如 127.0.0.1），生成您的服务端 SSL 证书对。</p>
          </div>
          <div style={{ background: "#09090b", padding: "20px", borderRadius: "8px", border: "1px solid var(--border-subtle)" }}>
            <div style={{ fontSize: "24px", fontWeight: 700, color: "#f43f5e", marginBottom: "8px", letterSpacing: "-0.02em" }}>03</div>
            <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "6px", color: "var(--text-primary)" }}>应用到您的服务</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.5 }}>导出证书。参考生成的 nginx.conf 示例配置 Web 服务，或者根据 electron.md 使用自签 HTTPS 连接。</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
