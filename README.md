<div align="center">

# OpenSunstar

### The All-in-One Desktop Manager for AI Coding CLI Tools

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**Repository:** [github.com/alisunstar/OpenSunstar](https://github.com/alisunstar/OpenSunstar)

English | [中文](README_ZH.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

</div>

---

## Table of Contents

- [1. What is OpenSunstar](#1-what-is-opensunstar)
- [2. Installation](#2-installation)
- [3. Quick Start](#3-quick-start)
- [4. FAQ](#4-faq)
- [Appendix](#appendix)
  - [Documentation](#documentation)
  - [Development](#development)
  - [Contributing](#contributing)
  - [License](#license)

---

## 1. What is OpenSunstar

Modern AI-assisted development often spans **Claude Code**, **Codex**, **Gemini CLI**, and more — yet each tool uses its own config format. Switching API providers means hand-editing JSON, TOML, and `.env` files; MCP servers and Skills drift apart across apps; multi-repo teams lack a single view of AI readiness.

**OpenSunstar** is a native desktop control plane (Tauri 2 + React) that brings connection, configuration, and project governance into one place.

### Supported CLI tools

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

### What you get

**Connect**

- **Quick Start** — curated onboarding wizard for Claude Code, Claude Desktop, Codex, and Gemini (official, China-friendly, aggregator, and custom endpoints)
- **50+ provider presets** with visual switching and system-tray quick switch
- **Local routing proxy** — format conversion, health checks, failover, and circuit breaker
- **Deep Link** (`OpenSunstar://`) for one-click imports

**Configure**

- Unified panels for **MCP**, **Skills**, **Prompts**, **Commands**, **Hooks**, **Ignore**, **Permissions**, and **Subagents**
- **MCP Discovery** — browse registry templates and install from **Smithery**
- **Skills Discovery** — GitHub repos, ZIP, skills.sh search, **skills.sh official leaderboard** (All-time / 24h Trending TOP 50), ClawHub, ModelScope
- **Session manager**, config **Convert**, sync & backup (WebDAV / S3-compatible)

**Govern (Workspace)**

- **Today Workspace** — portfolio snapshot with items that need attention
- **Project Board** — multi-repo cockpit with Git metrics, stages (MVP / Iteration / Stable), and AI weekly insights
- **AI Assets Overview** — matrix of MCP / Skills / Prompts counts across projects
- **Agent Readiness** — per-project readiness score with configured vs. effective (on-disk) scan
- **Project AI Assets** — bind and manage agent assets per Git repository

**Platform**

- Windows, macOS, Linux · SQLite storage with atomic writes · OS keychain for secrets where supported
- Dark / Light / System themes · i18n: 简体中文 · 繁體中文 · English · 日本語 · Deutsch
- Usage dashboard, budget alerts, custom model pricing, in-app updater

### Screenshots

| Main interface | Add provider |
| :------------: | :----------: |
| ![Main interface](website/assets/screenshots/main-zh.png) | ![Add provider](website/assets/screenshots/add-zh.png) |

> **v0.1.0** — first public release. Feature-complete for daily use; active development continues on workspace and asset lifecycle features.

---

## 2. Installation

### Download (recommended)

Get the latest build from [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest).

| Platform | Package |
| -------- | ------- |
| **Windows** | `.msi` installer or portable `.zip` |
| **macOS** | `.dmg` (signed & notarized) · `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR `OpenSunstar-bin` |

**Requirements:** Windows 10+ · macOS 12+ · Ubuntu 22.04+ / Debian 11+ / Fedora 34+

### Build from source

See [Development](#development) in the appendix.

---

## 3. Quick Start

### First launch

1. OpenSunstar can **auto-import** existing CLI configs as your default provider on first run.
2. Complete the **onboarding wizard** if prompted.

### Connect a CLI in three steps

1. Open sidebar → **Quick Start** (快速接入)
2. Pick a target app: **Claude Code**, **Claude Desktop**, **Codex**, or **Gemini**
3. Choose a curated supplier → enter API key (or follow official OAuth guidance) → **Verify & Apply**

For official providers (Anthropic / OpenAI / Google), Quick Start links you to **Settings → Provider management** for browser login.

> **Proxy note:** For Claude Code, Codex, Gemini, and Claude Desktop, keep OpenSunstar running while using the CLI — requests route through the local proxy.

### Switch providers

- Use the main UI or **system tray** to switch active provider
- **Restart the terminal** for most CLIs after switching (Claude Code supports **hot-switch**)

### Set up your workspace

1. Sidebar → **Workspace** → **Add project** and point to local Git repos
2. Open **Today Workspace** for attention items and readiness gaps
3. Use **Project Board** for commit activity and AI portfolio reports
4. Open a project's **AI Assets** to manage MCP / Skills / Prompts at repo level

### Explore agent tooling

| Goal | Where to go |
| ---- | ----------- |
| Install MCP servers | Agent config → **MCP** → Discovery (Smithery / registry) |
| Browse trending Skills | Agent config → **Skills** → skills.sh leaderboard |
| Manage prompts & hooks | Agent config → **Prompts** / **Commands** / **Hooks** |
| Track token spend | Sidebar → **AI Tokens** |

---

## 4. FAQ

<details>
<summary><strong>Which CLI tools are supported?</strong></summary>

Seven tools: Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw, and Hermes. Quick Start currently guides the first four; all seven are manageable from provider and agent panels.
</details>

<details>
<summary><strong>Do I need to restart the terminal after switching providers?</strong></summary>

Usually yes. Claude Code is the exception and supports hot-switching without a restart.
</details>

<details>
<summary><strong>Why must OpenSunstar stay running?</strong></summary>

For several CLIs, OpenSunstar writes a local proxy endpoint into the tool config. Closing the app stops the proxy and the CLI may show connection errors until OpenSunstar is running again.
</details>

<details>
<summary><strong>Where is my data stored?</strong></summary>

| Path | Purpose |
| ---- | ------- |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite database (providers, MCP, projects, assets) |
| `~/.OpenSunstar/settings.json` | App settings |
| `~/.OpenSunstar/backups/` | Auto-backups (last 10) |
| `~/.OpenSunstar/skills/` | Installed Skills cache |
| `~/.OpenSunstar/cache/` | Remote data cache (e.g. skills.sh leaderboard, 6h TTL) |
</details>

<details>
<summary><strong>How do I switch back to official login?</strong></summary>

Add or select an **Official** preset provider, switch to it, then run the CLI's Log out / Log in flow in your terminal.
</details>

<details>
<summary><strong>Is Workspace a task kanban?</strong></summary>

No. Workspace is a **multi-repo AI governance dashboard** — Git health, agent readiness, project-scoped assets, and AI insights — not a drag-and-drop issue board.
</details>

<details>
<summary><strong>How fresh is the skills.sh leaderboard?</strong></summary>

Data is fetched from skills.sh and cached locally (~6 hours). The UI shows last sync time; use refresh to force an update.
</details>

---

## Appendix

### Documentation

| Resource | Link |
| -------- | ---- |
| User manual (EN) | [docs/user-manual/en/README.md](docs/user-manual/en/README.md) |
| User manual (中文) | [docs/user-manual/zh/README.md](docs/user-manual/zh/README.md) |
| User manual (繁體) | [docs/user-manual/zh-TW/README.md](docs/user-manual/zh-TW/README.md) |
| User manual (日本語) | [docs/user-manual/ja/README.md](docs/user-manual/ja/README.md) |
| User manual (Deutsch) | [docs/user-manual/de/README.md](docs/user-manual/de/README.md) |
| Workspace module | [docs/kanban.md](docs/kanban.md) |
| Release notes v0.1.0 | [docs/release-notes/v0.1.0-en.md](docs/release-notes/v0.1.0-en.md) |

### Development

**Stack:** React 18 · TypeScript · Vite · Tauri 2 · Rust · SQLite · TanStack Query

**Prerequisites:** Node.js 20+ · pnpm · Rust 1.85+ · platform Tauri dependencies

```bash
pnpm install
pnpm tauri dev        # dev mode (desktop)
pnpm dev:renderer     # frontend only
pnpm typecheck        # TypeScript
pnpm test:unit        # unit tests
pnpm tauri build      # production build
```

### Contributing

Issues and pull requests are welcome. Before submitting:

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit
```

See [CONTRIBUTING.md](CONTRIBUTING.md). Partner and sponsor info: [SUPPORT.md](SUPPORT.md).

### License

[MIT](LICENSE) © Jason Young
