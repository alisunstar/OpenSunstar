# OpenSunstar 使用者手冊（繁體中文）

**版本：** v0.1.0 · **授權條款：** MIT

> 面向 Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes 的原生桌面管理器。

---

## 目錄

1. [快速上手](#1-快速上手)
2. [快速接入與供應商](#2-快速接入與供應商)
3. [Agent 配置](#3-agent-配置)
4. [專案組合](#4-專案組合)
5. [代理與故障轉移](#5-代理與故障轉移)
6. [用量與預算](#6-用量與預算)
7. [同步與備份](#7-同步與備份)
8. [設定與資料路徑](#8-設定與資料路徑)
9. [常見問題](#9-常見問題)

相關文件：[v0.1.0 發布說明](../release-notes/v0.1.0-zh-TW.md) · [專案組合模組](../kanban.md)

---

## 1. 快速上手

### 安裝

| 平台 | 安裝包 |
| ---- | ------ |
| Windows | `.msi` 或 Portable `.zip` |
| macOS | `.dmg` 或 `brew install --cask OpenSunstar` |
| Linux | `.deb` / `.rpm` / `.AppImage` |

從 [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) 下載。

### 首次啟動

1. 自動偵測並匯入現有 CLI 設定為 **default** 供應商。
2. 側欄 **快速接入** 使用精靈完成 API 配置。
3. 主介面或 **系統匣** 切換供應商。
4. 大多數 CLI 需 **重新啟動終端機**（Claude Code 支援 **熱切換**）。

### 側欄結構

| 區域 | 說明 |
| ---- | ---- |
| **API 接入** | 快速接入精靈 + 進階 Provider 面板 |
| **Agent 配置** | MCP、Skills、Prompts、命令、鉤子等 |
| **專案組合** | 多 Git 儲存庫儀表板 |
| **同步備份** | WebDAV / S3 / 匯入匯出 |
| **設定** | 語言、代理、目錄、關於 |

---

## 2. 快速接入與供應商

### 快速接入（3 步）

1. **供應商** — 選擇預設（DeepSeek、GLM、自訂 OpenAI 相容等）
2. **金鑰** — 儲存 API Key（macOS 可用 Keychain）
3. **套用** — 選擇 CLI 與模型，寫入設定

**進階 Provider** 分頁可管理完整供應商清單。

### 供應商操作

- **啟用** — 寫入對應應用程式的 live 設定
- **新增** — 預設或自訂端點
- **編輯** — Key、Base URL、模型、通用設定片段
- **排序** — 拖曳調整順序
- **匣** — 點擊供應商名稱即時切換

### 通用設定片段

切換供應商時保留外掛等擴充資料：

1. 編輯供應商 → **通用設定面板** → **從目前供應商提取**
2. 新建供應商時保持 **寫入通用設定** 勾選（預設開啟）

### 支援的應用

Claude Code · Claude Desktop · Codex · Gemini CLI · OpenCode · OpenClaw · Hermes

### Deep Link

透過 `OpenSunstar://import/...` URL 一鍵匯入供應商、MCP、Prompts、Skills。

---

## 3. Agent 配置

### MCP

- **MCP 面板** — 按應用新增、啟用、匯入伺服器
- **探索 MCP** — 瀏覽註冊表與範本
- **同步開關** — OpenSunstar 資料庫與 live 設定雙向同步

### Skills

- **管理** — 已安裝清單、按應用開關、批次操作
- **探索** — skills.sh、ClawHub、ModelScope、自訂 Git 儲存庫
- **安裝** — GitHub、ZIP、探索頁一鍵安裝
- 預設目錄：`~/.OpenSunstar/skills/`（可符號連結或複製，見設定）

### Prompts & rules

- Markdown 編輯 CLAUDE.md / AGENTS.md / GEMINI.md 等
- 啟用後同步至 live 檔案；讀取時有回填保護

### 其他 Agent 工具

| 功能 | 說明 |
| ---- | ---- |
| **命令** | 自訂斜線命令 |
| **鉤子** | 生命週期腳本 |
| **忽略規則** | 工具忽略設定 |
| **工具權限** | 權限預設 |
| **Subagent** | Agent 定義管理 |
| **工作階段** | 瀏覽與還原對話歷史 |
| **OpenClaw 工作區** | 編輯 AGENTS.md、SOUL.md 等 |

---

## 4. 專案組合

側欄 **專案組合** 是多 Git 儲存庫的**研發駕駛艙**，不是拖曳式任務看板。

### 新增專案

1. 側欄 **+** 或專案組合頁新增
2. 填寫名稱與本機 Git 儲存庫路徑
3. **重新整理指標** 掃描程式碼行數與 Git 統計

### 指標（7 天視窗）

以下能力共用 **近 7 天 Git 提交數**：

- 總覽卡片「近 7 天提交」
- 專案組合矩陣 X 軸
- AI 產生週報

健康評分仍參考 **30 天**提交，用於更長視窗趨勢判斷。

架構與持久化詳見 [kanban.md](../kanban.md)（SQLite + localStorage）。

### AI 洞察

- 組合摘要、健康評分、週報
- 需在 **設定 → AI 供應商** 中配置可用模型

---

## 5. 代理與故障轉移

### 本機路由代理

- Anthropic ↔ OpenAI 等格式轉換
- 請求整流器，相容各類上游
- 在 **設定 → 代理** 或供應商面板啟用

### 故障轉移

- 備用供應商佇列，失敗自動切換
- 可設定熔斷器閾值
- 介面展示供應商健康狀態

### 應用層級接管

可獨立為 Claude、Codex、Gemini 啟用代理，精確到單一供應商。

---

## 6. 用量與預算

### 用量儀表板

- 跨供應商支出、請求數、Token 趨勢
- 自訂模型定價
- 資料來源：代理日誌、OpenCode 工作階段、可選官方訂閱額度範本

### 預算告警

按供應商設定日 / 月 USD 上限，超限透過系統事件提醒。

---

## 7. 同步與備份

### 雲端同步

- **WebDAV** — 手動上傳/下載 + 可選自動同步
- **S3 相容** — AWS、R2、MinIO、OSS、COS、OBS 等預設
- WebDAV 與 S3 同時只能啟用一種

### 設定目錄

**設定 → 目錄** 可將資料目錄指向 Dropbox、iCloud、OneDrive、NAS 等。

### 匯入 / 匯出

- 匯出完整 SQL 備份（供應商、MCP、Prompts、Skills、設定等）
- 匯入時確認還原

---

## 8. 設定與資料路徑

| 路徑 | 內容 |
| ---- | ---- |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite — 供應商、MCP、Prompts、Skills、專案、AI 快取 |
| `~/.OpenSunstar/settings.json` | 介面與裝置偏好 |
| `~/.OpenSunstar/backups/` | 自動備份（最近 10 份） |
| `~/.OpenSunstar/skills/` | Skills 儲存 |
| `~/.OpenSunstar/skill-backups/` | 解除安裝前備份（最近 20 份） |

### 語言

简体中文 · 繁體中文 · English · 日本語

### 主題

深色 · 淺色 · 跟隨系統

---

## 9. 常見問題

**切換後要重新啟動終端機嗎？**  
多數需要。Claude Code 支援熱切換。

**為什麼不能刪除目前供應商？**  
至少保留一個啟用設定以保證 CLI 可用；不常用的應用可在設定中隱藏。

**如何切回官方登入？**  
新增官方預設 → 切換 → 在 CLI 中執行登出/登入。

**專案組合資料存在哪？**  
專案清單在 SQLite `projects` 表；階段/進度在 localStorage（後續遷入 SQLite）。

---

[← 手冊索引](../README.md) · [v0.1.0 發布說明](../release-notes/v0.1.0-zh-TW.md)
