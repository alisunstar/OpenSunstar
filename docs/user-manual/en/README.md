# OpenSunstar User Manual (English)

**Version:** v1.2.0 · **License:** Apache-2.0

> Local-first, all-in-one AI coding workflow engineering configuration platform for Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw, and Hermes.
>
> Source positioning: **本地优先，一站式统一管理你的 AI 编程工作流工程化配置平台** — 跨多项目组合矩阵以AI驱动的项目驾驶舱，一站式帮你基于项目的AI资产配置&工作流编排和跨工具跨设备Agent扩展配置同步
> Provider copy: **预设22+供应商，支持用户自定义配置更多供应商（含聚合/中转站）**

---

## Table of contents

1. [Getting started](#1-getting-started)
2. [Simple Connect & providers](#2-simple-connect--providers)
3. [Agent configuration](#3-agent-configuration)
4. [Project Cockpit](#4-project-cockpit)
5. [Proxy & failover](#5-proxy--failover)
6. [Usage & budget](#6-usage--budget)
7. [Sync & Collaboration](#7-sync--collaboration)
8. [Settings & data paths](#8-settings--data-paths)
9. [FAQ](#9-faq)

Related: [v0.1.0 Release Notes](../release-notes/v0.1.0-en.md) · [Project Cockpit module detail](../kanban.md)

---

## 1. Getting started

### Install

| Platform | Package |
| -------- | ------- |
| Windows | `.msi` or Portable `.zip` |
| macOS | `.dmg` or `brew install --cask OpenSunstar` |
| Linux | `.deb` / `.rpm` / `.AppImage` |

Download from [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest).

### First launch

1. OpenSunstar detects existing CLI configs and imports them as the **default** provider.
2. Use **Quick Start** (sidebar → AI Models → 快速接入) for a guided setup.
3. Switch providers from the main UI or **system tray**.
4. Restart your terminal for most CLIs (Claude Code supports **hot-switch**).

### Sidebar overview

OpenSunstar is no longer documented as a single provider switcher. Its source-of-truth navigation is the sidebar: a local-first engineering platform where projects, AI assets, workflow orchestration, Agent extensions, model access, sync, and collaboration share one cockpit.

| Sidebar entry | Submenus / entry points | Product narrative |
| --- | --- | --- |
| **Project Cockpit** | Today Alerts / Project Board | AI-driven multi-project portfolio cockpit: see risks, readiness gaps, stalled repos, stages, commit activity, and portfolio health. |
| **My Projects** | Project list / Add project / View project / Remove project | Bring real Git repositories into OpenSunstar and persist AI assets, wiki baselines, environment snapshots, and governance state per project. |
| **Project Config** | AI Asset Config / Workflow Orchestration | Land assets inside a selected repo: asset links, readiness & effectiveness, project environment & Wiki, rules/context, discovery, workflow config, change recipes, and design contracts. |
| **Agent Config (global)** | MCP / Skills / Prompt & Rules / Commands / Hooks / Ignore / Permissions / Subagents / Convert | Global Agent asset library: install, audit, convert, and sync extensions across tools, then decide per project which assets take effect. |
| **AI Models** | Quick Start / Context / AI Tokens | Quick Start ships **22+ preset providers** and lets users define more custom providers, including aggregators/relays; Context manages sessions, and AI Tokens tracks usage, budgets, and model costs. |
| **Sync & Collaboration** | Cross-device Cloud Sync / Team Collaboration (Beta) | WebDAV, S3, and GitHub Gist sync configs across devices; team config packages, members, invites, team keys, and deployments support collaboration. |
| **Bottom & Settings** | Sync status / Settings / Theme / Collapse sidebar | Show sync health and centralize General, Auth, Advanced, and About settings while preserving the local-first desktop workflow. |

---

## 2. Simple Connect & providers

### Simple Connect (3 steps)

1. **Provider** — Pick from 22+ preset providers (official, global AI, China AI, aggregators/relays) or define a custom endpoint
2. **Key** — Save API key (Keychain on macOS where supported)
3. **Apply** — Choose CLI tool and model, then write config

Switch to the **Expert** tab for full provider management, including user-defined providers and relay/aggregator endpoints.

### Provider operations

- **Preset catalog** — 22+ curated providers plus custom providers, including aggregators/relays
- **Enable** — Writes live config for the selected app
- **Add** — Preset or custom endpoint
- **Edit** — Keys, base URL, models, shared config snippet
- **Sort** — Drag to reorder
- **Tray** — Click provider name for instant switch

### Shared config snippets

When switching providers, plugin and extension data can be preserved:

1. Edit provider → **Shared config panel** → **Extract from current provider**
2. When creating a new provider, keep **Write shared config** checked (default)

### Supported apps

Claude Code · Claude Desktop · Codex · Gemini CLI · OpenCode · OpenClaw · Hermes

### Deep Link

Import via URL: `OpenSunstar://import/...` (providers, MCP, prompts, skills).

---

## 3. Agent configuration

### MCP

- **MCP panel** — Add, enable, import servers per app
- **Discovery** — Browse registry and install templates
- **Sync toggles** — Bidirectional sync between OpenSunstar DB and live app configs

### Skills

- **Manage** — Installed skills, per-app enable toggles, batch operations
- **Discover** — skills.sh, ClawHub, ModelScope, custom Git repos
- **Install** — GitHub repo, ZIP upload, one-click from discovery
- Default storage: `~/.OpenSunstar/skills/` (symlink or copy per settings)

### Prompts & rules

- Markdown editor for CLAUDE.md / AGENTS.md / GEMINI.md equivalents
- Activate to sync to live files; backfill protection on read

### Other Agent tools

| Feature | Description |
| ------- | ----------- |
| **Commands** | Custom slash commands |
| **Hooks** | Lifecycle hook scripts |
| **Ignore** | Ignore rules for tools |
| **Permissions** | Tool permission presets |
| **Subagents** | Agent definitions |
| **Sessions** | Browse and restore conversation history |
| **OpenClaw workspace** | Edit AGENTS.md, SOUL.md, etc. |

---

## 4. Project Cockpit

The sidebar entry **Project Cockpit** (项目驾驶舱) is a **multi-repo AI development cockpit**, not a drag-and-drop task board.

### Add projects

1. Sidebar → **My Projects → Add project** or Project Cockpit → Add project
2. Enter name and local Git repository path
3. Click **Refresh metrics** to scan code lines and Git stats

### Metrics (7-day window)

These share the same **7-day commit count**:

- Summary card “commits in last 7 days”
- Project Cockpit matrix X-axis
- AI-generated weekly report

Health scoring still references **30-day** commits for trend rules.

See [kanban.md](../kanban.md) for architecture and persistence (SQLite + localStorage).

### AI insights

- Project Cockpit summary, health breakdown, weekly report
- Requires configured AI provider in Settings → AI provider

---

## 5. Proxy & failover

### Local routing proxy

- Format conversion between API styles (Anthropic ↔ OpenAI, etc.)
- Request rectifier for upstream compatibility
- Enable in Settings → Proxy or provider panel

### Failover

- Queue backup providers with automatic switch on failure
- Circuit breaker thresholds configurable
- Provider health status in UI

### App-level takeover

Proxy can target Claude, Codex, or Gemini independently, down to a single provider.

---

## 6. Usage & budget

### Usage dashboard

- Spending, request count, token usage over time
- Per-model pricing overrides
- Data sources: proxy logs, OpenCode sessions, optional official subscription quota template

### Budget alerts

Set daily / monthly USD limits per provider; alerts via system events.

---

## 7. Sync & Collaboration

### Cloud sync

- **WebDAV** — Manual upload/download + optional auto-sync
- **S3-compatible** — AWS, R2, MinIO, OSS, COS, OBS presets
- Only one active cloud backend at a time

### Config directory

Point `~/.OpenSunstar` to Dropbox, iCloud, OneDrive, or NAS via Settings → Directories.

### Import / export

- Export full SQL backup (providers, MCP, prompts, skills, settings)
- Import restores from backup file with confirmation

---

## 8. Settings & data paths

| Path | Content |
| ---- | ------- |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite — providers, MCP, prompts, skills, projects, AI cache |
| `~/.OpenSunstar/settings.json` | UI preferences |
| `~/.OpenSunstar/backups/` | Auto backups (last 10) |
| `~/.OpenSunstar/skills/` | Skill storage |
| `~/.OpenSunstar/skill-backups/` | Pre-uninstall backups (last 20) |

### Languages

简体中文 · 繁體中文 · English · 日本語

### Themes

Dark · Light · Follow system

---

## 9. FAQ

**Restart terminal after switch?**
Usually yes. Claude Code hot-switch is the exception.

**Delete active provider?**
At least one active config is kept so the CLI remains usable. Hide unused apps in Settings instead.

**Back to official login?**
Add Official preset → switch → run CLI logout/login flow.

**Where is portfolio data?**
Projects in SQLite `projects` table; stage/progress in localStorage (migration planned).

---

[← Manual index](../README.md) · [Release notes v0.1.0](../release-notes/v0.1.0-en.md)
