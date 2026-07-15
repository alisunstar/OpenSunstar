# Changelog

All notable changes to OpenSunstar are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]


## [1.1.8] - 2026-07-15

### Added

- **Asset health**: derived healthy/attention/unhealthy status from deployment receipts and runtime evidence (snapshot + repair paths)
- **Project environment snapshots**: capture / restore provider · MCP · Skills · Prompt dimensions per project
- **Quick Start backend service**: idempotent apply pipeline with upstream verification receipts (no raw key in audit)
- **Quick Start usage dialog** and expanded unit/integration coverage for apply/verify and asset health UI
- **Database schema v36**: DAOs for asset_health, project_environment, and quick_start operations

### Changed

- **Simple Connect UI removed**: desk-side SimpleConnect panels deleted; Quick Start is the sole onboarding connect surface
- **Quick Start apply/verify**: stronger pipeline, provider build, and verification flows
- **Project asset panel**: asset capability contracts and health summary wiring
- **Providers / usage**: API and pricing panel updates aligned with the Quick Start path

### Fixed

- **i18n**: new strings for asset health, environment snapshots, and Quick Start flows

## [1.1.7] - 2026-07-14

### Added

- **Orchestration plan / receipt**: plan → snapshot → apply → verify → receipt with rollback from latest receipt
- **Flow orchestrator**: richer apply/export APIs, orchestration log, and restore-from-receipt command
- **Project Flow Orchestrator UI**: plan preview, dry-run, verification receipts, and rollback entry points

### Changed

- **Marker merge & project config sync**: safer merge/conflict handling for orchestration writes
- **Window chrome**: larger default window (1180×720); overlay titlebar safe top for high-DPI Windows/macOS
- **Methodology page**: wiring updates for flow orchestration entry

### Fixed

- **API cleanup**: remove unused vscode config import/export helpers

## [1.1.6] - 2026-07-13

### Added

- **Design system registry**: bundled offline packages (`data-dashboard`, `desktop-workbench`, `developer-console`) with manifest validation and design-contract integration
- **Local CLI auth status**: settings panel to inspect Claude / Gemini credential, route, and SimpleConnect layers
- **Change ID validation**: shared `changeId` rules for flow / recipe / design panels
- **Project asset contract**: machine-readable app-support matrix for asset panel consistency checks
- **Methodology page test**: regression coverage for framework detection UI

### Changed

- **Project orchestration panels**: richer design-contract, flow-orchestrator, and recipe-composer workflows with change-id guardrails
- **Methodology & project assets UI**: improved navigation, asset panel prompts, and install confirmation flows
- **Auth center / settings**: integrates local CLI auth probe alongside existing provider tooling
- **Backend hardening**: broad Rust refactors across audit, proxy, simple-connect, sync, and CLI command surfaces
- **Dev experience**: Tauri dev URL aligned to `127.0.0.1:1420`; bundle ships `resources/design-systems/**`

### Fixed

- **Database migrations**: safer v12 keychain migration and clearer schema upgrade logging
- **i18n**: expanded strings for auth status, methodology, and project orchestration panels

## [1.1.5] - 2026-07-11

### Added

- **Dashboard onboarding**: first-run guided tour for the AI workspace cockpit
- **Portfolio health summary**: classify projects as ok / warn / alert / unscanned with actionable reasons
- **Repair preview dialog**: preview drift repair impact before applying changes
- **Website interactive demos**: dashboard / terminal / sync demos on the marketing site
- **os-cli integrity**: SHA-256 checksums for Release binaries; npm publish verifies checksums and uses `--provenance`
- **CI**: `os` smoke tests, npm package verify, llvm-cov tooling; CI now runs on `master`

### Changed

- **Project assets matrix**: richer readiness / drift presentation and repair entry points
- **Cloud sync**: stronger Gist / WebDAV / S3 sync protocol and marker merge handling
- **Keychain & settings**: more robust credential and settings persistence paths
- **Release CI**: pin Rust 1.95.0 and Node via `.node-version`; generate os-cli checksums before npm publish
- **Docs**: CONTRIBUTING, CLI distribution P1, and dual-mode install guidance updates

### Fixed

- **AI insight / readiness**: safer batch scoring and insight command error handling
- **i18n**: expanded zh / zh-TW / en / ja strings for cockpit and repair flows

## [1.1.4] - 2026-07-09

### Added

- **OpenSunstar CLI (`os`)**: standalone binary with governance commands (`drift`, `readiness`, `flow`, `project`, `provider`, …)
- **Full-screen TUI dashboard**: run `os` in an interactive terminal (build with `--features tui`)
- **CLI bootstrap**: `os config bootstrap` / `os doctor --init` / auto-init on first DB access — no GUI required
- **Release CI**: GitHub Release attachments `OpenSunstar-v*-os-{platform}` for Windows / macOS / Linux
- **Dual-mode docs**: README and website sections for GUI + CLI(TUI) independent startup with shared `~/.OpenSunstar` data

### Changed

- **`os provider switch`**: now calls full `ProviderService::switch` (writes live CLI config, proxy hot-switch when takeover is active)

### Notes

- Advanced local proxy (`:15721`) still requires the desktop app process; governance and provider switch work CLI-only.

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
