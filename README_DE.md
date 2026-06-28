<div align="center">

# OpenSunstar

### Der All-in-One-Desktop-Manager für KI-Coding-CLI-Tools

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**Repository:** [github.com/alisunstar/OpenSunstar](https://github.com/alisunstar/OpenSunstar)

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch | [Changelog](CHANGELOG.md)

</div>

---

## Inhaltsverzeichnis

- [1. Was ist OpenSunstar](#1-was-ist-opensunstar)
- [2. Installation](#2-installation)
- [3. Schnellstart](#3-schnellstart)
- [4. FAQ](#4-faq)
- [Anhang](#anhang)
  - [Dokumentation](#dokumentation)
  - [Entwicklung](#entwicklung)
  - [Mitwirken](#mitwirken)
  - [Lizenz](#lizenz)

---

## 1. Was ist OpenSunstar

KI-gestützte Entwicklung nutzt oft **Claude Code**, **Codex**, **Gemini CLI** und mehr — doch jedes Tool hat sein eigenes Konfigurationsformat. Anbieterwechsel bedeutet manuelles Bearbeiten von JSON, TOML und `.env`; MCP und Skills laufen auseinander; Multi-Repo-Teams fehlt eine Gesamtansicht der AI-Readiness.

**OpenSunstar** ist eine native Desktop-Steuerzentrale (Tauri 2 + React), die **Anbindung, Konfiguration und Projekt-Governance** vereint.

### Unterstützte CLI-Tools

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

### Kernfunktionen

**Anbindung**

- **Schnellstart** — kuratierter Assistent für Claude Code, Claude Desktop, Codex und Gemini (offiziell, China-freundlich, Aggregator, benutzerdefiniert)
- **50+ Anbieter-Presets**, visuelles Umschalten + Tray-Schnellwechsel
- **Lokaler Routing-Proxy** — Formatkonvertierung, Health-Checks, Failover, Circuit Breaker
- **Deep Link** (`OpenSunstar://`) für Ein-Klick-Importe

**Konfiguration**

- Einheitliche Panels für **MCP**, **Skills**, **Prompts**, **Commands**, **Hooks**, **Ignore**, **Permissions**, **Subagents**
- **MCP Discovery** — Registry-Vorlagen durchsuchen, Installation über **Smithery**
- **Skills Discovery** — GitHub, ZIP, skills.sh-Suche, **skills.sh offizielles Ranking** (All-time / 24h Trend TOP 50), ClawHub, ModelScope
- **Sitzungsmanager**, Config-**Convert**, Sync & Backup (WebDAV / S3-kompatibel)

**Governance (Workspace)**

- **Heutiger Arbeitsbereich** — Portfolio-Snapshot mit Aufmerksamkeitspunkten
- **Projekt-Board** — Multi-Repo-Git-Cockpit, Stufen (MVP / Iteration / Stabil), KI-Wochenberichte
- **AI-Asset-Übersicht** — Matrix der MCP-/Skills-/Prompts-Anzahlen pro Projekt
- **Agent Readiness** — Projekt-Score mit konfiguriert vs. effektiv (auf der Festplatte)
- **Projekt-AI-Konfiguration** — Agent-Assets pro Git-Repository

**Plattform**

- Windows, macOS, Linux · SQLite mit atomaren Schreibvorgängen · OS-Keychain für Secrets (wo unterstützt)
- Dark / Light / System · i18n: 简体中文 · 繁體中文 · English · 日本語 · Deutsch
- Nutzungs-Dashboard, Budgetwarnungen, benutzerdefinierte Modellpreise, In-App-Updater

### Screenshots

| Hauptoberfläche | Anbieter hinzufügen |
| :-------------: | :-----------------: |
| ![Main Interface](website/assets/screenshots/main-zh.png) | ![Add Provider](website/assets/screenshots/add-zh.png) |

> **v0.1.0** — erste öffentliche Version, produktionsreif für den Alltag; Workspace- und Asset-Lifecycle-Features werden aktiv weiterentwickelt.

---

## 2. Installation

### Download (empfohlen)

Neuesten Build von [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) laden.

| Plattform | Paket |
| --------- | ----- |
| **Windows** | `.msi` oder Portable `.zip` |
| **macOS** | `.dmg` · `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR |

**Anforderungen:** Windows 10+ · macOS 12+ · Ubuntu 22.04+ / Debian 11+ / Fedora 34+

### Aus Quellcode bauen

Siehe [Entwicklung](#entwicklung) im Anhang.

---

## 3. Schnellstart

### Erster Start

1. Beim ersten Start können bestehende CLI-Konfigurationen als **default**-Anbieter importiert werden.
2. Onboarding-Assistenten bei Bedarf durchlaufen.

### CLI in drei Schritten anbinden

1. Seitenleiste → **Schnellstart** (快速接入)
2. Ziel-App wählen: **Claude Code**, **Claude Desktop**, **Codex** oder **Gemini**
3. Kuratierten Anbieter wählen → API-Key eingeben (oder offizielle OAuth-Anleitung) → **Prüfen & Anwenden**

Offizielle Anbieter (Anthropic / OpenAI / Google) erfordern Browser-Login unter **Einstellungen → Anbieterverwaltung**.

> **Proxy-Hinweis:** Für Claude Code, Codex, Gemini und Claude Desktop **OpenSunstar laufen lassen** — CLI-Anfragen laufen über den lokalen Proxy.

### Anbieter wechseln

- In der UI oder über die **Systemleiste** umschalten
- Für die meisten CLIs **Terminal neu starten** (Claude Code unterstützt **Hot-Switch**)

### Workspace einrichten

1. Seitenleiste → **Workspace** → **Projekt hinzufügen** (lokale Git-Repos)
2. **Heutiger Arbeitsbereich** für Aufmerksamkeitspunkte und Readiness-Lücken
3. **Projekt-Board** für Commit-Aktivität und KI-Portfolio-Berichte
4. **AI-Konfiguration** pro Projekt für MCP / Skills / Prompts auf Repo-Ebene

### Agent-Tools erkunden

| Ziel | Ort |
| ---- | --- |
| MCP installieren | Agent-Konfig → **MCP** → Discovery (Smithery / Registry) |
| Trending Skills | Agent-Konfig → **Skills** → skills.sh-Ranking |
| Prompts / Hooks | Agent-Konfig → **Prompts** / **Commands** / **Hooks** |
| Token-Nutzung | Seitenleiste → **AI Tokens** |

---

## 4. FAQ

<details>
<summary><strong>Welche CLI-Tools werden unterstützt?</strong></summary>

Sieben Tools: Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw und Hermes. Schnellstart führt die ersten vier; alle sieben sind über Anbieter- und Agent-Panels verwaltbar.
</details>

<details>
<summary><strong>Muss ich das Terminal nach dem Wechsel neu starten?</strong></summary>

In der Regel ja. Claude Code ist die Ausnahme mit Hot-Switch.
</details>

<details>
<summary><strong>Warum muss OpenSunstar laufen bleiben?</strong></summary>

Einige CLIs zeigen auf den lokalen OpenSunstar-Proxy. Beim Beenden stoppt der Proxy — Verbindungsfehler bis zum erneuten Start möglich.
</details>

<details>
<summary><strong>Wo werden Daten gespeichert?</strong></summary>

| Pfad | Zweck |
| ---- | ----- |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite-Datenbank |
| `~/.OpenSunstar/settings.json` | App-Einstellungen |
| `~/.OpenSunstar/backups/` | Auto-Backups (letzte 10) |
| `~/.OpenSunstar/skills/` | Installierte Skills |
| `~/.OpenSunstar/cache/` | Remote-Cache (z. B. skills.sh-Ranking, ~6 h TTL) |
</details>

<details>
<summary><strong>Wie kehre ich zur offiziellen Anmeldung zurück?</strong></summary>

**Official**-Preset wählen und umschalten, dann Log out / Log in im Terminal ausführen.
</details>

<details>
<summary><strong>Ist der Workspace ein Task-Kanban?</strong></summary>

Nein. Es ist ein **Multi-Repo-AI-Governance-Dashboard** — Git-Gesundheit, Agent Readiness, Projekt-Assets, KI-Einblicke — kein Issue-Drag-and-Drop-Board.
</details>

<details>
<summary><strong>Wie aktuell ist das skills.sh-Ranking?</strong></summary>

Daten von skills.sh, lokal ~6 Stunden gecacht. Letzte Sync-Zeit in der UI; manuelles Aktualisieren möglich.
</details>

---

## Anhang

### Dokumentation

| Ressource | Link |
| --------- | ---- |
| Benutzerhandbuch (Deutsch) | [docs/user-manual/de/README.md](docs/user-manual/de/README.md) |
| Benutzerhandbuch (English) | [docs/user-manual/en/README.md](docs/user-manual/en/README.md) |
| Benutzerhandbuch (中文) | [docs/user-manual/zh/README.md](docs/user-manual/zh/README.md) |
| Workspace-Modul | [docs/kanban.md](docs/kanban.md) |
| Release Notes v0.1.0 | [docs/release-notes/v0.1.0-de.md](docs/release-notes/v0.1.0-de.md) |

### Entwicklung

**Stack:** React 18 · TypeScript · Vite · Tauri 2 · Rust · SQLite · TanStack Query

**Voraussetzungen:** Node.js 20+ · pnpm · Rust 1.85+ · plattformspezifische Tauri-Abhängigkeiten

```bash
pnpm install
pnpm tauri dev        # Desktop-Entwicklung
pnpm dev:renderer     # nur Frontend
pnpm typecheck
pnpm test:unit
pnpm tauri build
```

### Mitwirken

Issues und PRs willkommen. Vor dem Einreichen:

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit
```

Siehe [CONTRIBUTING.md](CONTRIBUTING.md). Sponsoren: [SUPPORT.md](SUPPORT.md)

### Lizenz

[MIT](LICENSE) © Jason Young
