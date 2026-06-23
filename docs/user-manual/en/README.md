# OpenSunstar User Manual (English)

**Version:** v0.1.0 · **License:** MIT

> Native desktop manager for Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw, and Hermes.

---

## Table of contents

1. [Getting started](#1-getting-started)
2. [Simple Connect & providers](#2-simple-connect--providers)
3. [Agent configuration](#3-agent-configuration)
4. [Portfolio dashboard](#4-portfolio-dashboard)
5. [Proxy & failover](#5-proxy--failover)
6. [Usage & budget](#6-usage--budget)
7. [Sync & backup](#7-sync--backup)
8. [Settings & data paths](#8-settings--data-paths)
9. [FAQ](#9-faq)

Related: [v0.1.0 Release Notes](../release-notes/v0.1.0-en.md) · [Portfolio module detail](../kanban.md)

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
2. Use **Simple Connect** (sidebar → 快速接入 / API Connect) for a guided setup.
3. Switch providers from the main UI or **system tray**.
4. Restart your terminal for most CLIs (Claude Code supports **hot-switch**).

### Sidebar overview

| Section | Purpose |
| ------- | ------- |
| **API Connect** | Simple Connect wizard + expert provider panel |
| **Agent config** | MCP, Skills, Prompts, Commands, Hooks, etc. |
| **Portfolio** | Multi-repo Git dashboard |
| **Sync & backup** | WebDAV / S3 / export |
| **Settings** | Language, proxy, directories, about |

---

## 2. Simple Connect & providers

### Simple Connect (3 steps)

1. **Supplier** — Pick a preset (DeepSeek, GLM, custom OpenAI-compatible, etc.)
2. **Key** — Save API key (Keychain on macOS where supported)
3. **Apply** — Choose CLI tool and model, then write config

Switch to the **Expert** tab for full provider list management.

### Provider operations

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

## 4. Portfolio dashboard

The sidebar entry **Portfolio** (项目组合) is a **multi-repo development cockpit**, not a drag-and-drop task board.

### Add projects

1. Sidebar → **+** or Portfolio → Add project
2. Enter name and local Git repository path
3. Click **Refresh metrics** to scan code lines and Git stats

### Metrics (7-day window)

These share the same **7-day commit count**:

- Summary card “commits in last 7 days”
- Portfolio matrix X-axis
- AI-generated weekly report

Health scoring still references **30-day** commits for trend rules.

See [kanban.md](../kanban.md) for architecture and persistence (SQLite + localStorage).

### AI insights

- Portfolio summary, health breakdown, weekly report
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

## 7. Sync & backup

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
