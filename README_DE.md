<div align="center">

# OpenSunstar

### Der All-in-One-Desktop-Manager für KI-Coding-CLI-Tools

[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/alisunstar/OpenSunstar/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

**Website:** [opensunstar.github.io](https://opensunstar.github.io/) · [OpenSunstar.io](https://OpenSunstar.io)

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch | [Changelog](CHANGELOG.md)

</div>

---

## Über OpenSunstar

Die KI-gestützte Entwicklung nutzt **Claude Code**, **Codex**, **Gemini CLI** und mehr — doch jedes Tool hat sein eigenes Konfigurationsformat. Anbieterwechsel bedeutet manuelles Bearbeiten von JSON, TOML und `.env`, und MCP / Skills laufen auseinander.

**OpenSunstar** ist eine native Desktop-Steuerzentrale:

- Visueller Anbieterwechsel mit **50+ Presets**
- Ein Panel für **MCP, Skills, Prompts** und Agent-Tools
- **Portfolio-Dashboard** für Multi-Repo-Git-Gesundheit und KI-Einblicke
- **SQLite-Speicher** mit atomaren Schreibvorgängen

> **v0.1.0** ist die erste öffentliche Version — produktionsreif und funktionsvollständig.

## Highlights

- **Eine App, sieben CLI-Tools**
- **Simple Connect** — Anbieter → Key → Anwenden in drei Schritten
- **Tray-Schnellwechsel**
- **Lokaler Proxy & Failover**
- **Portfolio-Einblicke** — 7-Tage-Commit-Metriken, KI-Wochenberichte
- **Cloud-Sync** — WebDAV, S3-kompatibel
- **Plattformübergreifend** — Windows, macOS, Linux · Tauri 2

## Screenshots

| Hauptoberfläche | Anbieter hinzufügen |
| :-------------: | :-----------------: |
| ![Main Interface](assets/screenshots/main-en.png) | ![Add Provider](assets/screenshots/add-en.png) |

## Funktionen

### Anbindung & Anbieter

- 50+ integrierte Presets
- Universelle Anbieter für Claude Code, Codex, Gemini CLI
- Import/Export, gemeinsame Konfigurationsfragmente
- Deep Link (`OpenSunstar://`)

### Agent-Konfiguration

- Einheitliches **MCP**-Panel mit Discovery-Registry
- **Skills** aus GitHub, ZIP, skills.sh, ClawHub, ModelScope
- **Prompts**, Befehle, Hooks, Ignore-Regeln, Berechtigungen, Subagents
- Sitzungsmanager und OpenClaw-Workspace-Editor

### Proxy & Zuverlässigkeit

- Lokaler Routing-Proxy mit Request-Rectifier
- Auto-Failover und Circuit Breaker
- App-spezifische Proxy-Übernahme

### Portfolio & Nutzung

- **Portfolio-Dashboard** — Multi-Repo-Cockpit mit einheitlichen 7-Tage-Metriken
- KI-Zusammenfassung, Gesundheitsbewertung, Wochenberichte
- Nutzungs-Dashboard, Budgetwarnungen

### Plattform

- Dark / Light / System
- i18n: 简体中文 · 繁體中文 · English · 日本語
- Auto-Backup, Auto-Updater

[Release Notes v0.1.0 (DE)](docs/release-notes/v0.1.0-de.md) · [Benutzerhandbuch (DE)](docs/user-manual/de/README.md)

## Unterstützte Tools

| Claude Code | Claude Desktop | Codex | Gemini CLI | OpenCode | OpenClaw | Hermes |
| :---------: | :------------: | :---: | :--------: | :------: | :------: | :----: |

## Schnellstart

1. **Release** für Ihre Plattform herunterladen ([Releases](https://github.com/alisunstar/OpenSunstar/releases/latest))
2. **Simple Connect** → Anbieter → API-Key → auf CLI anwenden
3. Anbieter in der UI oder im **System-Tray wechseln**
4. **Terminal neu starten** (Claude Code unterstützt Hot-Switch)

## Download

| Plattform | Paket |
| --------- | ----- |
| **Windows** | `.msi` oder Portable `.zip` |
| **macOS** | `.dmg` · `brew install --cask OpenSunstar` |
| **Linux** | `.deb` · `.rpm` · `.AppImage` · AUR |

**Anforderungen:** Windows 10+ · macOS 12+ · Ubuntu 22.04+

## FAQ

<details>
<summary><strong>Welche Tools werden unterstützt?</strong></summary>

Sieben Tools: Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw und Hermes.
</details>

<details>
<summary><strong>Muss ich das Terminal nach dem Wechsel neu starten?</strong></summary>

In der Regel ja. Claude Code ist die Ausnahme mit Hot-Switch.
</details>

<details>
<summary><strong>Wo werden Daten gespeichert?</strong></summary>

- Datenbank: `~/.OpenSunstar/OpenSunstar.db`
- Einstellungen: `~/.OpenSunstar/settings.json`
- Backups: `~/.OpenSunstar/backups/`
</details>

## Dokumentation

**[Benutzerhandbuch (DE)](docs/user-manual/de/README.md)** · **[Portfolio-Modul](docs/kanban.md)** · **[Release Notes v0.1.0 (DE)](docs/release-notes/v0.1.0-de.md)**

## Entwicklung

```bash
pnpm install
pnpm tauri dev
pnpm typecheck
pnpm test:unit
pnpm tauri build
```

## Mitwirken

```bash
pnpm typecheck && pnpm format:check && pnpm test:unit
```

Siehe [CONTRIBUTING.md](CONTRIBUTING.md).

## Sponsoren

**[SUPPORT.md](SUPPORT.md)**

## Lizenz

[MIT](LICENSE) © Jason Young
