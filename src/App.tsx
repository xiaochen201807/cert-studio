import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LayoutDashboard, ShieldCheck, FileSpreadsheet, Lock, Sun, Moon } from "lucide-react";
import logoUrl from "./assets/logo.png";
import "./App.css";

import Dashboard from "./components/Dashboard";
import RootCA from "./components/RootCA";
import IssueCert from "./components/IssueCert";

function App() {
  const [activeTab, setActiveTab] = useState<string>("dashboard");
  const [hasRootCa, setHasRootCa] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [theme, setTheme] = useState<"light" | "dark">("light"); // 默认使用更清爽舒服的浅色主题

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

  const toggleTheme = () => {
    const nextTheme = theme === "light" ? "dark" : "light";
    setTheme(nextTheme);
    localStorage.setItem("theme", nextTheme);
    document.documentElement.setAttribute("data-theme", nextTheme);
  };

  useEffect(() => {
    checkRootCaStatus();
    
    // 初始化主题加载
    const savedTheme = localStorage.getItem("theme") as "light" | "dark" | null;
    const initialTheme = savedTheme || "light";
    setTheme(initialTheme);
    document.documentElement.setAttribute("data-theme", initialTheme);
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

        {/* 主题切换与状态 */}
        <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: "10px" }}>
          <button
            onClick={toggleTheme}
            className="menu-item-btn"
            style={{ 
              justifyContent: "space-between", 
              padding: "10px 14px",
              background: "var(--bg-card)",
              border: "1px solid var(--border-subtle)",
              cursor: "pointer"
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
              {theme === "light" ? <Moon size={15} /> : <Sun size={15} />}
              <span>{theme === "light" ? "深色模式" : "浅色模式"}</span>
            </div>
          </button>

          {/* 根 CA 安全状态显示卡 */}
          <div className="sidebar-status-card">
            <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "6px" }}>
              <Lock size={14} style={{ color: hasRootCa ? "var(--accent-success)" : "var(--accent-danger)" }} />
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
        </div>
      </aside>

      <main className="main-content" style={{ position: "relative" }}>
        <div style={{ display: activeTab === "dashboard" ? "block" : "none" }}>
          <Dashboard onNavigate={setActiveTab} hasRootCa={hasRootCa} />
        </div>
        <div style={{ display: activeTab === "rootca" ? "block" : "none" }}>
          <RootCA onCaChange={checkRootCaStatus} hasRootCa={hasRootCa} />
        </div>
        <div style={{ display: activeTab === "issue" ? "block" : "none" }}>
          <IssueCert hasRootCa={hasRootCa} onNavigate={setActiveTab} />
        </div>
      </main>
    </div>
  );
}

export default App;
