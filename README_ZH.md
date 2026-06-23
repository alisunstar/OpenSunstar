<div align="center">

# OpenSunstar

### Claude Code / Codex / Gemini CLI 等 AI 编程工具的一站式桌面管理器

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**官网：** [opensunstar.github.io](https://opensunstar.github.io/) · [OpenSunstar.io](https://OpenSunstar.io)

[English](README.md) | 中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [更新日志](CHANGELOG.md)

</div>

---

## 产品简介

AI 编程时代，开发者往往同时使用 **Claude Code**、**Codex**、**Gemini CLI** 等多款 CLI——但每家工具的配置格式各不相同。切换 API 供应商意味着手动改 JSON / TOML / `.env`，MCP 与 Skills 也难以跨应用统一管理。

**OpenSunstar** 是一款原生桌面控制台，将这些能力收敛到一处：

- **50+ 供应商预设**，可视化一键切换
- **MCP / Skills / Prompts** 统一面板，双向同步
- **项目组合** 多仓库研发仪表盘与 AI 洞察
- **SQLite 原子写入**，配置安全不损坏

> **v0.1.0** 为首次公开发布版本，功能已成型，具备正式上线使用条件。

## 核心亮点

- **一个应用，七个 CLI** — Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes
- **快速接入** — 选供应商 → 保存 Key → 应用到 CLI，三步完成
- **系统托盘切换** — 无需打开主窗口即可换供应商
- **本地代理与故障转移** — 格式转换、熔断器、健康监控
- **项目组合洞察** — 7 天统一提交指标、代码统计、AI 周报
- **云同步** — WebDAV、S3 兼容存储、自定义配置目录
- **跨平台** — Windows / macOS / Linux · 基于 Tauri 2

## 界面预览

| 主界面 | 添加供应商 |
| :----: | :--------: |
| ![主界面](assets/screenshots/main-zh.png) | ![添加供应商](assets/screenshots/add-zh.png) |

## 功能特性

### 接入与供应商

- 50+ 内置预设（中转站、云 API、Coding Plan）
- 通用供应商跨 Claude Code / Codex / Gemini CLI 同步
- 拖拽排序、导入导出、通用配置片段（保留插件数据）
- Deep Link（`OpenSunstar://`）一键导入

### Agent 配置

- 统一 **MCP** 面板：发现注册表、按应用同步开关
- **Skills**：GitHub / ZIP / skills.sh / ClawHub / ModelScope 一键安装
- **Prompts**、命令、钩子、忽略规则、权限、Subagent 全套工具
- **会话管理**与 OpenClaw 工作区编辑器

### 代理与可靠性

- 本地路由代理与请求整流器
- 自动故障转移队列与熔断器
- 按应用 / 按供应商的代理接管

### 项目组合与用量

- **项目组合仪表盘** — 多 Git 仓库驾驶舱，7 天指标统一
- AI 组合摘要、健康评分与周报生成
- 用量仪表盘、预算告警、自定义模型定价

### 系统能力

- 深色 / 浅色 / 跟随系统主题
- 国际化：简体中文 · 繁體中文 · English · 日本語
- 自动备份、应用内更新、最小侵入式设计

[v0.1.0 发布说明](docs/release-notes/v0.1.0-zh.md) · [用户手册](docs/user-manual/zh/README.md) · [繁體中文手冊](docs/user-manual/zh-TW/README.md)

## 支持的工具

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

## 快速开始

1. **下载** 对应平台的最新安装包（[Releases](https://github.com/alisunstar/OpenSunstar/releases/latest)）
2. 打开 **快速接入** → 选供应商 → 保存 Key → 应用到 CLI
3. 在主界面或系统托盘 **切换供应商**
4. **重启终端**使大多数 CLI 生效（Claude Code 支持热切换）

侧边栏可进入 **MCP**、**Skills**、**Prompts**；**项目组合** 中添加本地 Git 仓库即可查看研发洞察。

> 首次启动可自动导入现有 CLI 配置为 default 供应商。

## 下载安装

| 平台 | 安装包 |
| ---- | ------ |
| **Windows** | `.msi` 安装包或 Portable `.zip` 绿色版 |
| **macOS** | `.dmg`（已签名公证）· `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR `OpenSunstar-bin` |

**系统要求：** Windows 10+ · macOS 12+ · Ubuntu 22.04+ / Debian 11+ / Fedora 34+

## 常见问题

<details>
<summary><strong>支持哪些 AI 工具？</strong></summary>

七个工具：Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes，各有专属预设。
</details>

<details>
<summary><strong>切换供应商后要重启终端吗？</strong></summary>

大多数 CLI 需要重启终端。Claude Code 例外，支持热切换。
</details>

<details>
<summary><strong>数据存储在哪里？</strong></summary>

- 数据库：`~/.OpenSunstar/OpenSunstar.db`
- 本地设置：`~/.OpenSunstar/settings.json`
- 自动备份：`~/.OpenSunstar/backups/`（保留最近 10 份）
- Skills：`~/.OpenSunstar/skills/`
</details>

<details>
<summary><strong>如何切回官方登录？</strong></summary>

添加「官方（Official）」预设供应商并切换，然后在对应 CLI 中执行 Log out / Log in 流程。
</details>

<details>
<summary><strong>「项目组合」是看板吗？</strong></summary>

不是拖拽式任务看板，而是多 Git 仓库的研发健康度仪表盘，含代码量、提交活跃度与 AI 周报。
</details>

## 文档

完整指南：**[用户手册](docs/user-manual/zh/README.md)** · **[繁體中文手冊](docs/user-manual/zh-TW/README.md)** · **[项目组合说明](docs/kanban.md)** · **[v0.1.0 发布说明](docs/release-notes/v0.1.0-zh.md)**

## 开发

```bash
pnpm install
pnpm tauri dev        # 开发模式
pnpm typecheck        # 类型检查
pnpm test:unit        # 单元测试
pnpm tauri build      # 生产构建
```

技术栈：React 18 · TypeScript · Tauri 2 · Rust · SQLite · TanStack Query

## 参与贡献

欢迎提交 Issue 与 PR。提交前请确保：

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit
```

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 赞助商

合作伙伴与赞助信息见 **[SUPPORT.md](SUPPORT.md)**。

## 许可证

[MIT](LICENSE) © Jason Young
