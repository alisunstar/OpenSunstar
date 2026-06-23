# Changelog

All notable changes to OpenSunstar are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
