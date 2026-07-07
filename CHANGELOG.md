# Changelog

All notable changes to OpenSunstar are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]


## [1.1.3] - 2026-07-08

### Changed

- **方法论与编排：** 侧边栏「配置维度」更名为「方法论与编排」
- **方法论框架 Tab：** 进入页面自动恢复上次扫描结果；检测到框架后展示「去预设编排」入口及推荐流程档位
- **方法论框架 Tab：** 未检测框架收进「框架参考目录」折叠区；区分「未扫描」与「已扫描但未检测到」
- **自定义编排（Recipe Composer）：** 阶段图默认适应窗口宽度，支持「实际大小 / 适应宽度」切换
- **自定义编排（Recipe Composer）：** 字段、占位符、图例、按钮及提示文案全面中文化（zh / en / zh-TW / ja）
- **导航：** 移除侧栏重复的「同步备份」入口，统一在「设置 → 高级」管理

### Fixed

- **AI 资产配置：** 修复工作区「去配置」进入项目资产配置时 React #310 崩溃（Hooks 顺序）
- **方法论框架 Tab：** 修复刷新后扫描结果不恢复、未扫描时误显示「N 项目均未检测到」

## [1.1.2] - 2026-07-07

### Fixed

- **Quick Start:** Restored provider edit/delete via「管理供应商」tab (`ExpertProviderPanel`)
- **Quick Start:** Fixed misleading「前往供应商管理」button (now opens manage tab; proxy settings labeled correctly)
- **Portfolio:** Project stage and MVP progress now persist in SQLite (`projects.stage`, `projects.mvp_progress`); migrates from localStorage on first launch
- **Docs:** Restored `docs/kanban.md`; fixed dead links across README and user manual

### Changed

- **AI insight keys:** Migrated from localStorage to OS Keychain (DeepSeek / GLM / Custom)
- **Sidebar:** Added「同步备份」entry for `SyncBackupPage`
- **Versioning:** Aligned app version with 1.1.x release line (`1.1.2`); added `docs/VERSIONING.md` and `get_build_info` (app + schema version)

### Database

- Schema v28: `projects.stage`, `projects.mvp_progress`

## [0.1.0] - 2026-06-19

**Initial public release.** OpenSunstar is a native desktop app for managing AI coding CLI tools — provider switching, Agent configuration, and multi-repo portfolio insights in one place.

### Added

#### API Connect & Providers

- **Simple Connect** — Three-step wizard: pick supplier → save API key → apply to CLI tools
- **Seven supported tools** — Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw, Hermes
- **50+ provider presets** — One-click import for mainstream relays, cloud APIs, and coding plans
- **Universal providers** — Single config synced across Claude Code, Codex, and Gemini CLI
- **One-click switching** — Main UI and system-tray quick switch (Claude Code supports hot-switch)
- **Shared config snippets** — Preserve plugins and extensions when switching providers
- **Import / export** — Backup and restore provider configurations

#### Agent Configuration

- **Unified MCP panel** — Manage MCP servers across five apps with bidirectional sync and discovery registry
- **Skills** — Install from GitHub, ZIP, skills.sh, ClawHub, and ModelScope; custom repo management
- **Prompts & rules** — Markdown editor with cross-app sync (CLAUDE.md / AGENTS.md / GEMINI.md)
- **Commands, Hooks, Ignore rules, Permissions, Subagents** — Full Agent workspace tooling
- **Session manager** — Browse, search, and restore conversation history
- **OpenClaw workspace editor** — Edit AGENTS.md, SOUL.md, and related agent files
- **Deep Link** (`OpenSunstar://`) — Import providers, MCP servers, prompts, and skills via URL

#### Proxy & Reliability

- **Local routing proxy** — Format conversion, request rectifier, and upstream compatibility fixes
- **Auto-failover & circuit breaker** — Provider health monitoring with automatic failover queue
- **App-level proxy takeover** — Per-app, per-provider proxy configuration

#### Portfolio & Insights

- **Portfolio dashboard** — Multi local Git repository cockpit (not a drag-and-drop task board)
- **Unified 7-day metrics** — Commit counts aligned across summary cards, matrix, and AI weekly reports
- **Code metrics** — tokei-based line counts, language breakdown, Git contributors
- **AI insights** — Portfolio summary, health scoring, risk hints, and natural-language queries

#### Usage & Operations

- **Usage dashboard** — Cross-provider spend, token trends, request logs, and custom model pricing
- **Budget alerts** — Daily / monthly budget warnings via system events
- **Cloud sync** — WebDAV and S3-compatible object storage; Gist sync for lightweight backup
- **Custom config directory** — Point data to Dropbox, iCloud, OneDrive, NAS, etc.

#### Platform

- **Cross-platform native app** — Windows, macOS, and Linux (Tauri 2)
- **SQLite storage** — Atomic writes, automatic backups, minimal-intrusion design
- **Themes & i18n** — Dark / Light / System; Simplified Chinese, Traditional Chinese, English, Japanese
- **Auto-updater** — In-app updates on supported platforms

### License

Released under the [MIT License](LICENSE).
