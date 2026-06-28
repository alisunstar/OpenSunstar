<div align="center">

# OpenSunstar

### Claude Code / Codex / Gemini CLI 等 AI 编程工具的一站式桌面管理器

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**代码仓库：** [github.com/alisunstar/OpenSunstar](https://github.com/alisunstar/OpenSunstar)

[English](README.md) | 中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [更新日志](CHANGELOG.md)

</div>

---

## 目录

- [一. 什么是 OpenSunstar](#一-什么是-opensunstar)
- [二. 安装指南](#二-安装指南)
- [三. 快速开始](#三-快速开始)
- [四. 常见问题 FAQ](#四-常见问题-faq)
- [附录](#附录)
  - [文档](#文档)
  - [开发](#开发)
  - [参与贡献](#参与贡献)
  - [许可证](#许可证)

---

## 一. 什么是 OpenSunstar

AI 编程时代，开发者往往同时使用 **Claude Code**、**Codex**、**Gemini CLI** 等多款 CLI——但各家配置格式不同。切换 API 供应商要手动改 JSON / TOML / `.env`；MCP 与 Skills 难以跨应用统一；多仓库团队也缺少 AI 就绪度与资产的一览视图。

**OpenSunstar** 是基于 Tauri 2 + React 的原生桌面控制台，把**接入、配置、项目治理**收敛到一处。

### 支持的 CLI 工具

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

### 核心能力

**接入**

- **快速接入** — 面向 Claude Code、Claude Desktop、Codex、Gemini 的精选向导（官方 / 国产 / 聚合 / 自定义）
- **50+ 供应商预设**，可视化切换 + 系统托盘快捷切换
- **本地路由代理** — 格式转换、健康检查、故障转移与熔断
- **Deep Link**（`OpenSunstar://`）一键导入

**配置**

- 统一管理 **MCP**、**Skills**、**Prompts**、**Commands**、**Hooks**、**Ignore**、**Permissions**、**Subagents**
- **MCP 发现** — 浏览注册表模板，并从 **Smithery** 安装
- **Skills 发现** — GitHub 仓库、ZIP、skills.sh 搜索、**skills.sh 官方排行榜**（全站总榜 / 24h 趋势 TOP 50）、ClawHub、ModelScope
- **会话管理**、配置 **Convert**、同步与备份（WebDAV / S3 兼容）

**治理（工作区）**

- **今日工作台** — 组合级快照，突出需关注项
- **项目看板** — 多仓库驾驶舱：Git 指标、阶段（MVP / 迭代 / 稳定）、AI 组合周报
- **AI 资产总览** — 跨项目的 MCP / Skills / Prompts 数量矩阵
- **Agent 就绪度** — 按项目评分，支持「已配置 vs 磁盘生效态」扫描
- **项目 AI 配置** — 按 Git 仓库绑定与管理 Agent 资产

**系统**

- Windows / macOS / Linux · SQLite 原子写入 · 密钥优先走系统 Keychain
- 深色 / 浅色 / 跟随系统 · 多语言：简体中文 · 繁體中文 · English · 日本語 · Deutsch
- 用量仪表盘、预算告警、自定义模型定价、应用内更新

### 界面预览

| 主界面 | 添加供应商 |
| :----: | :--------: |
| ![主界面](website/assets/screenshots/main-zh.png) | ![添加供应商](website/assets/screenshots/add-zh.png) |

> **v0.1.0** 为首次公开发布版本，可日常使用；工作区与 AI 资产闭环能力仍在持续迭代。

---

## 二. 安装指南

### 下载安装（推荐）

从 [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) 获取最新构建。

| 平台 | 安装包 |
| ---- | ------ |
| **Windows** | `.msi` 安装包或 Portable `.zip` 绿色版 |
| **macOS** | `.dmg`（已签名公证）· `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR `OpenSunstar-bin` |

**系统要求：** Windows 10+ · macOS 12+ · Ubuntu 22.04+ / Debian 11+ / Fedora 34+

### 源码构建

见附录 [开发](#开发)。

---

## 三. 快速开始

### 首次启动

1. 首次运行可**自动导入**现有 CLI 配置为 default 供应商。
2. 若弹出引导向导，按提示完成即可。

### 三步接入 CLI

1. 侧边栏 → **快速接入**
2. 选择目标应用：**Claude Code**、**Claude Desktop**、**Codex** 或 **Gemini**
3. 选择精选供应商 → 填写 API Key（或按官方 OAuth 指引）→ **验证并应用**

官方供应商（Anthropic / OpenAI / Google）需在 **设置 → 供应商管理** 中完成浏览器登录。

> **代理提示：** Claude Code、Codex、Gemini、Claude Desktop 使用时请**保持 OpenSunstar 运行**，CLI 请求经本地代理转发。

### 切换供应商

- 在主界面或**系统托盘**切换当前供应商
- 大多数 CLI 切换后需**重启终端**（Claude Code 支持**热切换**）

### 配置工作区

1. 侧边栏 → **工作区** → **添加项目**，绑定本地 Git 仓库
2. 打开 **今日工作台** 查看待办与就绪度缺口
3. 在 **项目看板** 查看提交活跃度与 AI 组合报告
4. 进入项目的 **AI 配置** 管理仓库级 MCP / Skills / Prompts

### 探索 Agent 工具

| 目标 | 入口 |
| ---- | ---- |
| 安装 MCP | Agent 配置 → **MCP** → 发现（Smithery / 注册表） |
| 浏览热门 Skills | Agent 配置 → **Skills** → skills.sh 排行榜 |
| 管理 Prompts / Hooks | Agent 配置 → **Prompts** / **Commands** / **Hooks** |
| 查看 Token 用量 | 侧边栏 → **AI Tokens** |

---

## 四. 常见问题 FAQ

<details>
<summary><strong>支持哪些 AI 工具？</strong></summary>

七个工具：Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes。快速接入向导覆盖前四个；全部七个可在供应商与 Agent 面板中管理。
</details>

<details>
<summary><strong>切换供应商后要重启终端吗？</strong></summary>

大多数 CLI 需要重启终端。Claude Code 例外，支持热切换。
</details>

<details>
<summary><strong>为什么需要保持 OpenSunstar 运行？</strong></summary>

部分 CLI 的配置会指向 OpenSunstar 本地代理。关闭应用后代理停止，CLI 可能出现连接失败，需重新启动 OpenSunstar。
</details>

<details>
<summary><strong>数据存储在哪里？</strong></summary>

| 路径 | 用途 |
| ---- | ---- |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite 数据库（供应商、MCP、项目、资产） |
| `~/.OpenSunstar/settings.json` | 应用设置 |
| `~/.OpenSunstar/backups/` | 自动备份（保留最近 10 份） |
| `~/.OpenSunstar/skills/` | 已安装 Skills 缓存 |
| `~/.OpenSunstar/cache/` | 远程数据缓存（如 skills.sh 排行榜，约 6 小时 TTL） |
</details>

<details>
<summary><strong>如何切回官方登录？</strong></summary>

添加或选择 **Official（官方）** 预设供应商并切换，然后在终端执行对应 CLI 的 Log out / Log in 流程。
</details>

<details>
<summary><strong>「工作区」是任务看板吗？</strong></summary>

不是。工作区是**多仓库 AI 治理仪表盘**——Git 健康度、Agent 就绪度、项目级资产与 AI 洞察——而非拖拽式 Issue 看板。
</details>

<details>
<summary><strong>skills.sh 排行榜多久更新一次？</strong></summary>

从 skills.sh 拉取后本地缓存约 6 小时。界面显示上次同步时间；可手动刷新强制更新。
</details>

---

## 附录

### 文档

| 资源 | 链接 |
| ---- | ---- |
| 用户手册（中文） | [docs/user-manual/zh/README.md](docs/user-manual/zh/README.md) |
| 用户手册（English） | [docs/user-manual/en/README.md](docs/user-manual/en/README.md) |
| 用户手册（繁體） | [docs/user-manual/zh-TW/README.md](docs/user-manual/zh-TW/README.md) |
| 用户手册（日本語） | [docs/user-manual/ja/README.md](docs/user-manual/ja/README.md) |
| 用户手册（Deutsch） | [docs/user-manual/de/README.md](docs/user-manual/de/README.md) |
| 工作区模块说明 | [docs/kanban.md](docs/kanban.md) |
| v0.1.0 发布说明 | [docs/release-notes/v0.1.0-zh.md](docs/release-notes/v0.1.0-zh.md) |

### 开发

**技术栈：** React 18 · TypeScript · Vite · Tauri 2 · Rust · SQLite · TanStack Query

**环境要求：** Node.js 20+ · pnpm · Rust 1.85+ · 各平台 Tauri 构建依赖

```bash
pnpm install
pnpm tauri dev        # 桌面开发模式
pnpm dev:renderer     # 仅前端
pnpm typecheck        # 类型检查
pnpm test:unit        # 单元测试
pnpm tauri build      # 生产构建
```

### 参与贡献

欢迎提交 Issue 与 PR。提交前请确保：

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit
```

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。合作伙伴与赞助信息见 [SUPPORT.md](SUPPORT.md)。

### 许可证

[MIT](LICENSE) © Jason Young
