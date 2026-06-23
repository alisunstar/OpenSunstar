<div align="center">

# OpenSunstar

### The All-in-One Desktop Manager for AI Coding CLI Tools

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**Website:** [opensunstar.github.io](https://opensunstar.github.io/) · [OpenSunstar.io](https://OpenSunstar.io)

English | [中文](README_ZH.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

</div>

---

## About

Modern AI-assisted development spans **Claude Code**, **Codex**, **Gemini CLI**, and more — yet each tool speaks its own config dialect. Switching API providers means hand-editing JSON, TOML, and `.env` files, and MCP / Skills drift apart across apps.

**OpenSunstar** is a native desktop control plane that unifies all of this:

- Visual provider switching with **50+ presets**
- One panel for **MCP, Skills, Prompts**, and Agent tooling
- A **portfolio dashboard** for multi-repo Git health and AI insights
- **SQLite-backed** storage with atomic writes — your configs stay safe

> **v0.1.0** is the first public release. The product is feature-complete and ready for production use.

## Highlights

- **One app, seven CLI tools** — Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw, Hermes
- **Simple Connect** — Supplier → Key → Apply in three steps
- **Tray quick switch** — Change providers without opening the full window
- **Local proxy & failover** — Format conversion, circuit breaker, health monitoring
- **Portfolio insights** — 7-day commit metrics, code stats, AI weekly reports
- **Cloud sync** — WebDAV, S3-compatible storage, custom config directories
- **Cross-platform** — Windows, macOS, Linux · built with Tauri 2

## Screenshots

| Main Interface | Add Provider |
| :------------: | :----------: |
| ![Main Interface](assets/screenshots/main-en.png) | ![Add Provider](assets/screenshots/add-en.png) |

## Features

### Connect & Providers

- 50+ built-in presets (relays, cloud APIs, coding plans)
- Universal providers shared across Claude Code, Codex, Gemini CLI
- Drag-and-drop ordering, import/export, shared config snippets
- Deep Link (`OpenSunstar://`) one-click imports

### Agent Configuration

- Unified **MCP** panel with discovery registry and per-app sync toggles
- **Skills** from GitHub, ZIP, skills.sh, ClawHub, ModelScope
- **Prompts**, Commands, Hooks, Ignore rules, Permissions, Subagents
- **Session manager** and OpenClaw workspace editor

### Proxy & Reliability

- Local routing proxy with request rectifier
- Auto-failover queue and circuit breaker
- Per-app proxy takeover down to individual providers

### Portfolio & Usage

- **Portfolio dashboard** — multi-repo Git cockpit with unified 7-day metrics
- AI portfolio summary, health scoring, and weekly reports
- Usage dashboard, budget alerts, custom model pricing

### Platform

- Dark / Light / System themes
- i18n: 简体中文 · 繁體中文 · English · 日本語
- Auto-backup, auto-updater, minimal-intrusion design

[Release notes v0.1.0](docs/release-notes/v0.1.0-en.md) · [User manual](docs/user-manual/en/README.md)

## Supported Tools

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

## Quick Start

1. **Download** the latest release for your platform ([Releases](https://github.com/alisunstar/OpenSunstar/releases/latest))
2. **Simple Connect** → pick a supplier → save your API key → apply to a CLI tool
3. **Switch providers** from the main UI or system tray
4. **Restart the terminal** for most CLIs (Claude Code supports hot-switch)

Explore **MCP**, **Skills**, and **Prompts** from the sidebar. Add local Git repos under **Portfolio** for team-wide insights.

> On first launch, existing CLI configs can be imported automatically as the default provider.

## Download

| Platform | Package |
| -------- | ------- |
| **Windows** | `.msi` installer or Portable `.zip` |
| **macOS** | `.dmg` (signed & notarized) · `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR `OpenSunstar-bin` |

**Requirements:** Windows 10+ · macOS 12+ · Ubuntu 22.04+ / Debian 11+ / Fedora 34+

## FAQ

<details>
<summary><strong>Which tools are supported?</strong></summary>

Seven tools: Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw, and Hermes — each with dedicated presets.
</details>

<details>
<summary><strong>Do I need to restart the terminal after switching?</strong></summary>

Usually yes. Claude Code is the exception and supports hot-switching without a restart.
</details>

<details>
<summary><strong>Where is my data stored?</strong></summary>

- Database: `~/.OpenSunstar/OpenSunstar.db`
- Settings: `~/.OpenSunstar/settings.json`
- Backups: `~/.OpenSunstar/backups/` (last 10)
- Skills: `~/.OpenSunstar/skills/`
</details>

<details>
<summary><strong>How do I switch back to official login?</strong></summary>

Add an "Official" preset provider, switch to it, then run the CLI's Log out / Log in flow.
</details>

## Documentation

Full guides: **[User Manual](docs/user-manual/en/README.md)** · **[繁體中文手冊](docs/user-manual/zh-TW/README.md)** · **[Portfolio module](docs/kanban.md)** · **[Release notes v0.1.0](docs/release-notes/v0.1.0-en.md)**

## Development

```bash
pnpm install
pnpm tauri dev        # dev mode
pnpm typecheck        # TypeScript
pnpm test:unit        # unit tests
pnpm tauri build      # production build
```

Stack: React 18 · TypeScript · Tauri 2 · Rust · SQLite · TanStack Query

## Contributing

Issues and PRs are welcome. Before submitting:

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Sponsors

Partner and sponsor information: **[SUPPORT.md](SUPPORT.md)**

## License

[MIT](LICENSE) © Jason Young
