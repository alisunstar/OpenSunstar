<div align="center">

# OpenSunstar

### Der All-in-One-Desktop-Manager für KI-Coding-CLI-Tools

[![Version](https://img.shields.io/badge/version-v1.1.6-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**Repository:** [github.com/alisunstar/OpenSunstar](https://github.com/alisunstar/OpenSunstar)

[English](README.md) | [中文](README_ZH.md) | [繁體中文](docs/user-manual/zh-TW/README.md) | [日本語](README_JA.md) | Deutsch | [Changelog](CHANGELOG.md)

</div>

---

## Inhaltsverzeichnis

- [1. Was ist OpenSunstar](#1-was-ist-opensunstar)
  - [Zielgruppe](#zielgruppe)
  - [Kern-Anwendungsfälle (8 Szenarien)](#kern-anwendungsfälle-8-szenarien)
  - [Sechs gelöste Schmerzpunkte](#sechs-gelöste-schmerzpunkte)
  - [Funktionsübersicht](#funktionsübersicht)
- [2. Installation](#2-installation)
- [3. Schnellstart](#3-schnellstart)
- [4. FAQ](#4-faq)
- [Anhang](#anhang)
  - [Dokumentation](#dokumentation)
  - [Entwicklung](#entwicklung)
  - [Mitwirken](#mitwirken)
  - [Danksagung](#danksagung)
  - [Lizenz](#lizenz)

---

## 1. Was ist OpenSunstar

**OpenSunstar** ist eine plattformübergreifende native Desktop-App (Tauri 2 + React) für die AI-Coding-CLI-Ära. Sie vereint **Anbieter-Onboarding, Agent-Konfiguration und Multi-Projekt-Governance**.

### Zielgruppe

#### Kern-Personas (5 Typen)

| Persona | Profil | Bedarf |
| ------- | ------ | ------ |
| **Multi-CLI-Entwickler** | 2–3 Tools parallel | Anbieterwechsel an einem Ort |
| **AI-Coding-Einsteiger** | Neu bei CLI Agents | **Schnellstart** in 3 Schritten |
| **Multi-Projekt-Indie-Dev** | Mehrere Repos | Stagnation & fehlende AI-Assets sehen |
| **Tech Lead** | Parallele Git-Repos | Board, Readiness, KI-Wochenberichte |
| **Agent-Power-User** | MCP / Skills intensiv | Unified Sync, skills.sh, Smithery |

#### Nicht die Zielgruppe

Teams ohne AI-CLI, Nutzer nur mit offiziellem Abo, PMs die Jira/Linear-Kanban brauchen, reine SaaS-Konfigurationszentren.

### Kern-Anwendungsfälle (8 Szenarien)

1. Schnellstart für Claude Code / Desktop / Codex / Gemini  
2. Multi-Tool-Anbieterwechsel (Tray, Hot-Switch bei Claude Code)  
3. Unified Agent Assets (MCP, Skills, Prompts, …)  
4. MCP & Skills Discovery (Smithery, skills.sh-Ranking)  
5. Multi-Repo-Governance (Today Workspace, Board, Asset-Matrix)  
6. Readiness pro Projekt + direkte Asset-Nachverfolgung  
7. Token-Statistik, Budget, KI-Investitionsberichte  
8. Backup & Sync (WebDAV / S3 / Gist, Deep Link)

### Sechs gelöste Schmerzpunkte

| # | Schmerzpunkt | Lösung |
| - | ------------ | ------ |
| 1 | Unterschiedliche Config-Formate | Visuelles Management + Schnellstart |
| 2 | Anbieterwechsel pro Tool einzeln | Ein-Klick + lokaler Proxy |
| 3 | Single Point of Failure | Failover, Circuit Breaker |
| 4 | Verstreute MCP/Skills/Prompts | Unified Agent Panels |
| 5 | Keine Usage-Sicht | AI Tokens, Budget-Alerts |
| 6 | Keine Multi-Projekt-Readiness | Workspace-Scores, Asset-Matrix |

### Funktionsübersicht

| Feature | Beschreibung |
| ------- | ------------ |
| **7 CLI-Tools** | Claude Code · Desktop · Codex · Gemini CLI · OpenCode · OpenClaw · Hermes |
| **Schnellstart** | Kuratiert für 4 Apps |
| **7+ Presets** | Kuratiert für Schnellzugriff; weitere in Einstellungen (inkl. Relays) |
| **Agent-Konfig** | MCP · Skills · Prompts · Commands · Hooks · Ignore · Permissions · Subagents |
| **Workspace** | Today WS · Board · Asset-Übersicht · Readiness |
| **Discovery** | skills.sh · Smithery · ClawHub · ModelScope |
| **Plattform** | Windows · macOS · Linux · i18n |

### Unterstützte CLI-Tools

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

### Screenshots

| Schnellstart | Heutiger Arbeitsbereich |
| :----------: | :---------------------: |
| ![Quick Start](website/assets/screenshots/quickstart-zh.png) | ![Workspace](website/assets/screenshots/workspace-zh.png) |

> **v0.1.0** — erste öffentliche Version; Workspace-Features werden aktiv weiterentwickelt.

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

### Danksagung

OpenSunstar baut auf dem Open-Source-Projekt [cc-switch](https://github.com/farion1231/cc-switch) auf. OpenSunstar wird unabhängig weiterentwickelt — verankert in strategischer Positionierung, Wertversprechen und Produktnarrative.

### Lizenz

[MIT](LICENSE)

Kern ist Open Source; Team- und Enterprise-Funktionen sind für separate kommerzielle Vereinbarungen geplant.
