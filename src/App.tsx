import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LayoutDashboard, ShieldCheck, FileSpreadsheet, Lock } from "lucide-react";
import logoUrl from "./assets/logo.png";
import "./App.css";

import Dashboard from "./components/Dashboard";
import RootCA from "./components/RootCA";
import IssueCert from "./components/IssueCert";

function App() {
  const [activeTab, setActiveTab] = useState<string>("dashboard");
  const [hasRootCa, setHasRootCa] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  // 检查本地是否存在有效的根证书
  const checkRootCaStatus = async () => {
    try {
      const status = await invoke<boolean>("has_valid_root_ca");
      setHasRootCa(status);
    } catch (e) {
      console.error("检查根证书状态失败:", e);
      setHasRootCa(false);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    checkRootCaStatus();
  }, []);

  return (
    <div className="app-container">
      {/* 侧栏导航 */}
      <aside className="sidebar">
        <div className="brand-section">
          <img src={logoUrl} alt="Logo" className="brand-logo" />
          <span className="brand-name gradient-text-blue">Cert Studio</span>
        </div>

        <nav>
          <ul className="menu-list">
            <li>
              <button
                className={`menu-item-btn ${activeTab === "dashboard" ? "active" : ""}`}
                onClick={() => setActiveTab("dashboard")}
              >
                <LayoutDashboard size={18} />
                <span>仪表盘概览</span>
              </button>
            </li>
            <li>
              <button
                className={`menu-item-btn ${activeTab === "rootca" ? "active" : ""}`}
                onClick={() => setActiveTab("rootca")}
              >
                <ShieldCheck size={18} />
                <span>Root CA 管理</span>
              </button>
            </li>
            <li>
              <button
                className={`menu-item-btn ${activeTab === "issue" ? "active" : ""}`}
                onClick={() => setActiveTab("issue")}
              >
                <FileSpreadsheet size={18} />
                <span>签发 HTTPS 证书</span>
              </button>
            </li>
          </ul>
        </nav>

        {/* 根 CA 安全状态显示卡 */}
        <div className="sidebar-status-card">
          <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "6px" }}>
            <Lock size={14} className={hasRootCa ? "gradient-text-green" : "gradient-text-pink"} />
            <span style={{ fontSize: "12px", color: "var(--text-secondary)", fontWeight: 600 }}>安全守护状态</span>
          </div>
          {isLoading ? (
            <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>检测中...</span>
          ) : (
            <div style={{ display: "flex", alignItems: "center", marginTop: "4px" }}>
              <span className={`status-indicator ${hasRootCa ? "active" : "inactive"}`}></span>
              <span style={{ fontSize: "11px", color: "var(--text-primary)" }}>
                {hasRootCa ? "Root CA 已就绪" : "未检测到 Root CA"}
              </span>
            </div>
          )}
        </div>
      </aside>

      {/* 主界面切换 */}
      <main className="main-content">
        {activeTab === "dashboard" && (
          <Dashboard onNavigate={setActiveTab} hasRootCa={hasRootCa} />
        )}
        {activeTab === "rootca" && (
          <RootCA onCaChange={checkRootCaStatus} hasRootCa={hasRootCa} />
        )}
        {activeTab === "issue" && (
          <IssueCert hasRootCa={hasRootCa} onNavigate={setActiveTab} />
        )}
      </main>
    </div>
  );
}

export default App;
