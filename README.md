<div align="center">

# OpenSunstar

### The All-in-One Platform for AI Coding Workflow Engineering

**一站式统一管理你的 AI 编程工作流工程化配置平台**

*跨多项目组合矩阵的 AI 就绪度驾驶舱，一站式帮你基于项目的方法论 & 工作流编排和跨工具跨设备 Agent 配置双向同步*

[![Version](https://img.shields.io/badge/version-v1.1.8-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![npm opensunstar-os](https://img.shields.io/npm/v/opensunstar-os.svg)](https://www.npmjs.com/package/opensunstar-os)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**Repository:** [github.com/alisunstar/OpenSunstar](https://github.com/alisunstar/OpenSunstar)

English | [中文](README_ZH.md) | [繁體中文](docs/user-manual/zh-TW/README.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

</div>

---

## Table of Contents

- [1. What is OpenSunstar](#1-what-is-opensunstar)
  - [GUI + CLI(TUI) dual-mode](#gui--clitui-dual-mode)
  - [Target users](#target-users)
  - [Core use cases (8 scenarios)](#core-use-cases-8-scenarios)
  - [Six pain points we solve](#six-pain-points-we-solve)
  - [Feature overview](#feature-overview)
- [2. Installation](#2-installation)
- [3. Quick Start](#3-quick-start)
- [4. FAQ](#4-faq)
- [Appendix](#appendix)
  - [Documentation](#documentation)
  - [Development](#development)
  - [Contributing](#contributing)
  - [Acknowledgements](#acknowledgements)
  - [License](#license)

---

## 1. What is OpenSunstar

**OpenSunstar** is a cross-platform native desktop app (Tauri 2 + React) — **the all-in-one platform to manage your AI coding workflow engineering configuration**.

> **An AI readiness cockpit across a multi-project portfolio matrix**: methodology & workflow orchestration per project, plus bidirectional Agent configuration sync across tools and devices.

Move from “edit config files by hand” to “see project health, orchestrate workflows, fill asset gaps, and keep shipping.”

### GUI + CLI(TUI) dual-mode

OpenSunstar ships **two independent entry points** that share the same data under `~/.OpenSunstar/`:

| Mode | Entry | Best for |
| ---- | ----- | -------- |
| **Desktop GUI** | OpenSunstar app | Visual provider/asset/workspace management, local proxy & HA |
| **CLI `os`** | Terminal commands + **full-screen TUI dashboard** | Agents, CI, SSH/headless, scripting (`--json`) |

**Run CLI without starting the GUI** — `os` bootstraps `OpenSunstar.db` on first use (`os config bootstrap` or automatic). Governance, provider switch, and the TUI work standalone.

**Shared state, not isolated silos** — changes in GUI or CLI both read/write the same SQLite DB and live CLI configs.

| Capability | CLI-only | Notes |
| ---------- | -------- | ----- |
| Drift / readiness / flow / project | ✅ | Agent-native `--json` |
| Provider switch (writes live config) | ✅ | `os provider switch` |
| Full-screen TUI dashboard | ✅ | Run `os` in an interactive terminal |
| Advanced local proxy `:15721` | ⚠️ | Proxy process is started by the **desktop app** today |
| WebDAV/S3 auto-sync pull | ⚠️ | Use GUI or `os sync` export; full pull via GUI |

> **Dual-mode, independently startable; data is unified.** For proxy takeover (Claude Code / Codex / Gemini), keep the desktop app running until headless `os proxy` ships.

### Product map (aligned with the sidebar)

#### Workspace — AI readiness cockpit

| Entry | Capability |
| ----- | ---------- |
| **Today Workspace** | First glance: todos, readiness gaps, portfolio overview |
| **Project Board** | Multi-repo stages, commit activity, AI portfolio reports |
| **AI Assets Overview** | Project × asset matrix (MCP / Skills / Prompts / Cmd / Hooks …) |
| **Project · AI Config** | Enable/link agent assets per repo, jump to fix gaps |

#### Project Config — Methodology & Orchestration

| Tab | Capability |
| --- | ---------- |
| **Methodology Framework** | Read-only SDD framework detection (spec-kit, flow-kit, …) |
| **Preset Orchestration** | Flow tiers, modules & stages → `workflow.profile.json` |
| **Custom Orchestration** | Visual stage graph + Recipe (YAML+Markdown hybrid) |
| **Design Contract** | Brand templates → `DESIGN.md` + DTCG tokens |

#### Agent Config — cross-tool bidirectional sync

MCP · Skills · Prompts · Commands · Hooks · Ignore · Permissions · Subagents · Convert — unified install, audit, and per-app sync to 7 CLIs.

#### AI Models

| Entry | Capability |
| ----- | ---------- |
| **Quick Connect** | 3-step wizard: pick supplier → enter key → apply to CLI |
| **Context** | Browse, search, and restore conversation sessions |
| **AI Tokens** | Usage stats, budget alerts, custom model pricing |

#### Settings

Provider management, cloud sync & backup (WebDAV / S3 / Gist), proxy & HA, theme & language, and more.

### Target users

#### Core personas (5 types)

| Persona | Typical profile | Primary need |
| ------- | --------------- | ------------ |
| **Multi-CLI developer** | Uses 2–3 tools (Claude Code, Codex, Gemini, …) | Switch providers in one place, avoid JSON / TOML / `.env` edits |
| **AI coding newcomer** | New to CLI agents, unfamiliar with vendor configs | **Quick Start**: pick supplier → enter key → apply in 3 steps |
| **Multi-project indie dev** | Several side projects or client repos | See at a glance which projects stall and which lack AI assets |
| **Tech lead / full-stack owner** | Parallel Git repos, needs stage & risk visibility | Project board, readiness score, AI weekly & investment reports |
| **Agent power user** | Heavy MCP / Skills / Prompts / Hooks usage | Unified install & sync, skills.sh leaderboard, Smithery discovery |

#### Who it is not for

- Teams with **no AI CLI workflow** (no Claude Code / Codex / Gemini need)
- Users on **one official subscription only**, never switching providers
- PMs needing **Jira / Linear-style task boards** — Workspace is an **AI governance dashboard**, not issue tracking
- Teams wanting a **hosted SaaS config hub** — OpenSunstar is **local-first** with optional cloud sync

### Core use cases (8 scenarios)

1. **AI workspace governance** — Today Workspace, Project Board, AI Assets Overview, readiness scoring
2. **Methodology & workflow orchestration** — framework detection, preset orchestration, custom Recipe, design contract
3. **One-click API access** — curated Quick Connect for Claude Code / Desktop / Codex / Gemini
4. **Cross-tool Agent sync** — 9 agent modules with bidirectional sync
5. **Context management** — multi-CLI session browse, search, and restore
6. **AI usage & cost** — Token dashboard, budget alerts, investment reports
7. **MCP & Skills discovery** — Smithery, skills.sh leaderboard, ClawHub, ModelScope
8. **Backup & cross-device sync** — WebDAV / S3 / Gist, Deep Link import

### Six pain points we solve

| # | Pain point | How OpenSunstar helps |
| - | ---------- | --------------------- |
| 1 | Different config formats per CLI, error-prone manual edits | Visual provider management + Quick Start writes live configs |
| 2 | Switching API providers means editing each tool separately | One-click switch, local proxy + format conversion, tray shortcut |
| 3 | Single provider failure breaks the workflow | Failover queue, circuit breaker, health monitoring |
| 4 | MCP / Skills / Prompts scattered and hard to unify | Unified agent panels with per-app bidirectional sync |
| 5 | No clear view of API usage and spend | AI Tokens dashboard, budget alerts, custom model pricing |
| 6 | No AI readiness or asset view across projects | Workspace readiness, asset matrix, project-scoped AI config |

### Feature overview

| Feature | Description |
| ------- | ----------- |
| **7 CLI tools** | Claude Code · Claude Desktop · Codex · Gemini CLI · OpenCode · OpenClaw · Hermes |
| **Quick Connect** | 7+ curated presets (Official · CN · Aggregator · Custom); add more in Settings (incl. relays) |
| **Methodology & Orchestration** | Framework detection · preset orchestration · custom Recipe · design contract |
| **Agent configuration** | MCP · Skills · Prompts · Commands · Hooks · Ignore · Permissions · Subagents · Convert |
| **AI workspace** | Today Workspace · Project Board · AI Assets Overview · Agent Readiness |
| **Context & usage** | Context sessions · AI Tokens dashboard · budget alerts |
| **Skills / MCP discovery** | skills.sh leaderboard · Smithery · ClawHub · ModelScope · GitHub |
| **Secret storage** | OS Keychain first, SQLite atomic writes |
| **Sync & backup** | WebDAV / S3 / Gist · auto-backup · Deep Link import |
| **Cross-platform** | Windows · macOS · Linux · dark/light themes · i18n |
| **CLI `os` + TUI** | Standalone binary · governance · provider switch · full-screen dashboard |

### Supported CLI tools

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

### Screenshots

| Quick Start | Today Workspace |
| :---------: | :-------------: |
| ![Quick Start](website/assets/screenshots/quickstart-zh.png) | ![Today Workspace](website/assets/screenshots/workspace-zh.png) |

> **v1.1.8** — Asset health + project environment snapshots, Quick Start as sole connect surface (Simple Connect UI removed), schema v36.

---

## 2. Installation

### Desktop GUI (recommended for visual management)

Get the latest build from [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest).

| Platform | Package |
| -------- | ------- |
| **Windows** | `.msi` installer or portable `.zip` |
| **macOS** | `.dmg` (signed & notarized) · `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR `OpenSunstar-bin` |

**Requirements:** Windows 10+ · macOS 12+ · Ubuntu 22.04+ / Debian 11+ / Fedora 34+

### OpenSunstar CLI (`os`) — standalone binary (no GUI required)

Use **`os`** without the GUI for governance checks, provider switching, and a full-screen TUI dashboard. **No need to launch the desktop app first** — the CLI creates `~/.OpenSunstar/OpenSunstar.db` on first run.

| Platform | Release asset |
| -------- | ------------- |
| **Windows** | `OpenSunstar-v*-os-windows-x86_64.zip` (`os.exe`) |
| **macOS** | `OpenSunstar-v*-os-macos-aarch64.tar.gz` / `os-macos-x86_64` |
| **Linux** | `OpenSunstar-v*-os-linux-x86_64.tar.gz` |

**Install options (pick one):**

| Method | Command |
| ------ | ------- |
| **npm / pnpm** | `npm install -g opensunstar-os` — [npmjs.com/package/opensunstar-os](https://www.npmjs.com/package/opensunstar-os) |
| **GitHub Release** | Download asset above, unzip, add `os` / `os.exe` to PATH |
| **Scoop** (Windows) | `scoop install opensunstar-os` — see [distrib/scoop](distrib/scoop) |
| **Winget** (Windows) | `winget install OpenSunstar.OpenSunstarCLI` — see [distrib/winget](distrib/winget) |

> `opensunstar-os` is a thin Node wrapper that downloads the native binary from GitHub Releases on install — not a JS reimplementation. See [docs/cli-distribution-p1.md](docs/cli-distribution-p1.md).

```bash
# npm global install (good for agents / CI)
npm install -g opensunstar-os

# or manual: unzip Release asset and add to PATH

# First run (creates ~/.OpenSunstar/OpenSunstar.db)
os config bootstrap --yes

# Full-screen TUI dashboard (interactive terminal — run with no subcommand)
os

# Common commands
os doctor --json
os drift check --json
os provider list --app claude
os provider switch --app claude --id <provider-id> --yes
```

See [AGENTS.md](AGENTS.md) for Agent integration.

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

> **Proxy note:** When **proxy takeover** is enabled for Claude Code, Codex, Gemini, or Claude Desktop, keep the **desktop app** running so the local proxy on `:15721` stays up. Pure governance and `os provider switch` do not require the GUI.

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
| Release notes v1.1.3 | [docs/release-notes/v1.1.3-en.md](docs/release-notes/v1.1.3-en.md) |

### Development

**Stack:** React 18 · TypeScript · Vite · Tauri 2 · Rust · SQLite · TanStack Query

**Prerequisites:** Node.js 22.13+ · pnpm 11.5.2 · Rust 1.95.0 · platform Tauri dependencies

```bash
pnpm dev:doctor       # verify local toolchain
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

### Acknowledgements

OpenSunstar builds upon the open-source [cc-switch](https://github.com/farion1231/cc-switch) project. OpenSunstar will continue to evolve independently, anchored to its strategic positioning, value proposition, and product narrative.

### License

[MIT](LICENSE)

Core is open source; team and enterprise capabilities are planned to be offered under separate commercial agreements.
