import React from "react";
import { ShieldAlert, ShieldCheck, ArrowRight, Shield, Key, FileText } from "lucide-react";

interface DashboardProps {
  hasRootCa: boolean;
  onNavigate: (tab: string) => void;
}

const Dashboard: React.FC<DashboardProps> = ({ hasRootCa, onNavigate }) => {
  return (
    <div className="page-fade-in" style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      {/* 欢迎模块 */}
      <div className="glass-panel" style={{ padding: "40px", position: "relative", overflow: "hidden" }}>
        <div style={{ position: "relative", zIndex: 2 }}>
          <h1 style={{ fontSize: "32px", fontWeight: 700, marginBottom: "12px" }}>
            欢迎使用 <span className="gradient-text-blue">Cert Studio</span>
          </h1>
          <p style={{ color: "var(--text-secondary)", fontSize: "16px", maxWidth: "600px", lineHeight: 1.6 }}>
            企业级自签本地 Root CA 管理与 HTTPS 服务端证书一键签发工具。符合安全规范，零依赖一键本地部署。
          </p>
        </div>
        <div 
          style={{ 
            position: "absolute", 
            right: "-20px", 
            bottom: "-40px", 
            opacity: 0.08, 
            color: "var(--primary-neon)",
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
              <div style={{ padding: "10px", borderRadius: "10px", background: "hsla(152, 90%, 55%, 0.12)", color: "var(--accent-neon)" }}>
                <ShieldCheck size={24} />
              </div>
            ) : (
              <div style={{ padding: "10px", borderRadius: "10px", background: "hsla(318, 100%, 62%, 0.12)", color: "var(--secondary-neon)" }}>
                <ShieldAlert size={24} />
              </div>
            )}
            <h3 style={{ fontSize: "18px", fontWeight: 600 }}>自签 Root CA 状态</h3>
          </div>
          
          <p style={{ color: "var(--text-secondary)", fontSize: "14px", lineHeight: 1.5 }}>
            {hasRootCa 
              ? "本地根证书已初始化完毕，系统已获得自签名信任锚点，可立即为您签发安全的 HTTPS 服务端证书。" 
              : "检测到您当前尚未配置 Root CA 证书。请先创建全新的根证书或导入公司已有的 CA 证书和私钥。"}
          </p>

          <div style={{ marginTop: "auto", paddingTop: "16px" }}>
            {hasRootCa ? (
              <button 
                onClick={() => onNavigate("issue")}
                style={{ 
                  background: "linear-gradient(135deg, var(--primary-neon), #2563eb)", 
                  color: "#fff", 
                  padding: "12px 20px", 
                  borderRadius: "10px", 
                  display: "flex", 
                  alignItems: "center", 
                  gap: "8px",
                  fontSize: "14px",
                  fontWeight: 600,
                  boxShadow: "var(--glow-shadow)"
                }}
              >
                <span>立即签发 HTTPS 证书</span>
                <ArrowRight size={16} />
              </button>
            ) : (
              <button 
                onClick={() => onNavigate("rootca")}
                style={{ 
                  background: "linear-gradient(135deg, var(--secondary-neon), #db2777)", 
                  color: "#fff", 
                  padding: "12px 20px", 
                  borderRadius: "10px", 
                  display: "flex", 
                  alignItems: "center", 
                  gap: "8px",
                  fontSize: "14px",
                  fontWeight: 600,
                  boxShadow: "var(--glow-shadow-pink)"
                }}
              >
                <span>初始化 Root CA</span>
                <ArrowRight size={16} />
              </button>
            )}
          </div>
        </div>

        {/* 核心特性展示 */}
        <div className="glass-panel" style={{ padding: "30px", display: "flex", flexDirection: "column", gap: "20px" }}>
          <h3 style={{ fontSize: "18px", fontWeight: 600, marginBottom: "4px" }}>Cert Studio 核心优势</h3>
          
          <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
            <div style={{ display: "flex", gap: "12px" }}>
              <Lock size={16} style={{ color: "var(--primary-neon)", marginTop: "3px", flexShrink: 0 }} />
              <div>
                <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "2px" }}>系统级硬件密钥环加密</h4>
                <p style={{ color: "var(--text-secondary)", fontSize: "12px" }}>私钥优先委托系统 Keyring 加密托管，绝对不向普通磁盘落地明文私钥。</p>
              </div>
            </div>

            <div style={{ display: "flex", gap: "12px" }}>
              <Key size={16} style={{ color: "var(--accent-neon)", marginTop: "3px", flexShrink: 0 }} />
              <div>
                <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "2px" }}>完整 SANs 多域名支持</h4>
                <p style={{ color: "var(--text-secondary)", fontSize: "12px" }}>同时支持多 DNS 域名与 IP 绑定，无缝匹配 Chrome 现代浏览器的校验规范。</p>
              </div>
            </div>

            <div style={{ display: "flex", gap: "12px" }}>
              <FileText size={16} style={{ color: "var(--secondary-neon)", marginTop: "3px", flexShrink: 0 }} />
              <div>
                <h4 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "2px" }}>自动化配套导出</h4>
                <p style={{ color: "var(--text-secondary)", fontSize: "12px" }}>一键导出全链证书、私钥，并自动配套生成 Nginx 服务器和 Electron 开发环境的配置指南。</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 极简使用流程引导 */}
      <div className="glass-panel" style={{ padding: "30px" }}>
        <h3 style={{ fontSize: "18px", fontWeight: 600, marginBottom: "20px" }}>3 步开始开发测试</h3>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "24px" }}>
          <div style={{ background: "hsla(224, 25%, 10%, 0.4)", padding: "20px", borderRadius: "12px", border: "1px solid var(--border-glass)" }}>
            <div style={{ fontSize: "28px", fontWeight: 800, color: "var(--primary-neon)", marginBottom: "8px" }}>01</div>
            <h4 style={{ fontSize: "15px", fontWeight: 600, marginBottom: "6px" }}>初始化 Root CA</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.4 }}>点击 Root CA 导航，创建或者导入根证书。导出根证书并安装到您的操作系统“受信任的根证书颁发机构”中。</p>
          </div>
          <div style={{ background: "hsla(224, 25%, 10%, 0.4)", padding: "20px", borderRadius: "12px", border: "1px solid var(--border-glass)" }}>
            <div style={{ fontSize: "28px", fontWeight: 800, color: "var(--accent-neon)", marginBottom: "8px" }}>02</div>
            <h4 style={{ fontSize: "15px", fontWeight: 600, marginBottom: "6px" }}>签发服务端证书</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.4 }}>输入您局域网开发使用的域名（如 *.company.com）或 IP（如 127.0.0.1），生成您的服务端 SSL 证书对。</p>
          </div>
          <div style={{ background: "hsla(224, 25%, 10%, 0.4)", padding: "20px", borderRadius: "12px", border: "1px solid var(--border-glass)" }}>
            <div style={{ fontSize: "28px", fontWeight: 800, color: "var(--secondary-neon)", marginBottom: "8px" }}>03</div>
            <h4 style={{ fontSize: "15px", fontWeight: 600, marginBottom: "6px" }}>应用到您的服务</h4>
            <p style={{ color: "var(--text-secondary)", fontSize: "12px", lineHeight: 1.4 }}>导出证书。参考生成的 nginx.conf 示例配置 Web 服务，或者根据 electron.md 使用自签 HTTPS 连接。</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
