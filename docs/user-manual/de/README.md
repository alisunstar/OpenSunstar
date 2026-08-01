# OpenSunstar Benutzerhandbuch (Deutsch)

**Version:** v1.2.0 · **Lizenz:** Apache-2.0

> Local-first Engineering-Plattform für AI-Coding-Workflows mit Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw und Hermes.
>
> Quellpositionierung: **本地优先，一站式统一管理你的 AI 编程工作流工程化配置平台** — 跨多项目组合矩阵以AI驱动的项目驾驶舱，一站式帮你基于项目的AI资产配置&工作流编排和跨工具跨设备Agent扩展配置同步
> Provider copy: **预设22+供应商，支持用户自定义配置更多供应商（含聚合/中转站）**

---

## Inhaltsverzeichnis

1. [Erste Schritte](#1-erste-schritte)
2. [Simple Connect & Anbieter](#2-simple-connect--anbieter)
3. [Agent-Konfiguration](#3-agent-konfiguration)
4. [Project Cockpit](#4-project-cockpit)
5. [Proxy & Failover](#5-proxy--failover)
6. [Nutzung & Budget](#6-nutzung--budget)
7. [Sync & Collaboration](#7-sync--collaboration)
8. [Einstellungen & Datenpfade](#8-einstellungen--datenpfade)
9. [FAQ](#9-faq)

Verwandt: [v0.1.0 Release Notes](../release-notes/v0.1.0-de.md) · [Project-Cockpit-Modul](../kanban.md)

---

## 1. Erste Schritte

### Installation

| Plattform | Paket |
| --------- | ----- |
| Windows | `.msi` oder Portable `.zip` |
| macOS | `.dmg` oder `brew install --cask OpenSunstar` |
| Linux | `.deb` / `.rpm` / `.AppImage` |

Download von [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest).

### Erster Start

1. OpenSunstar erkennt vorhandene CLI-Konfigurationen und importiert sie als **default**-Anbieter.
2. Nutzen Sie **Quick Start** (Seitenleiste → AI Models → 快速接入) für die geführte Einrichtung.
3. Wechseln Sie Anbieter in der Hauptoberfläche oder über die **Systemleiste**.
4. Starten Sie das Terminal für die meisten CLIs neu (Claude Code unterstützt **Hot-Switch**).

### Seitenleiste im Überblick

OpenSunstar wird nicht mehr als einzelner Anbieter-Umschalter beschrieben. Die Seitenleiste aus dem Quellcode ist die Informationsarchitektur: eine local-first Engineering-Plattform für Projekte, AI-Assets, Workflow-Orchestrierung, Agent-Erweiterungen, Modellzugang, Sync und Zusammenarbeit.

| Seitenleisten-Eintrag | Untermenüs / Einstiegspunkte | Produktnarrativ |
| --- | --- | --- |
| **Project Cockpit** | Today Alerts / Project Board | KI-getriebenes Multi-Projekt-Portfolio: Risiken, Readiness-Lücken, stagnierende Repos, Phasen, Commit-Aktivität und Portfolio-Gesundheit. |
| **My Projects** | Projektliste / Projekt hinzufügen / ansehen / entfernen | Reale Git-Repositories aufnehmen und AI-Assets, Wiki-Basis, Umgebungs-Snapshots und Governance-Status pro Projekt speichern. |
| **Project Config** | AI Asset Config / Workflow Orchestration | Assets im ausgewählten Repo verankern: Verknüpfungen, Readiness & Wirksamkeit, Projektumgebung & Wiki, Regeln/Kontext, Discovery, Workflow-Konfiguration, Change Recipes und Design Contracts. |
| **Agent Config (global)** | MCP / Skills / Prompt & Rules / Commands / Hooks / Ignore / Permissions / Subagents / Convert | Globale Agent-Asset-Bibliothek: Erweiterungen installieren, auditieren, konvertieren und je Tool synchronisieren; pro Projekt wird entschieden, was aktiv ist. |
| **AI Models** | Quick Start / Context / AI Tokens | Quick Start enthält **22+ Anbieter-Presets** und erlaubt zusätzliche benutzerdefinierte Anbieter inklusive Aggregatoren/Relays; Context verwaltet Sessions, AI Tokens Nutzung, Budgets und Modellkosten. |
| **Sync & Collaboration** | Cross-device Cloud Sync / Team Collaboration (Beta) | WebDAV, S3 und GitHub Gist synchronisieren Konfigurationen; Team-Pakete, Mitglieder, Einladungen, Team-Keys und Deployments stützen Zusammenarbeit. |
| **Bottom & Settings** | Sync-Status / Settings / Theme / Sidebar einklappen | Sync-Zustand anzeigen und General, Auth, Advanced und About zentral verwalten. |

---

## 2. Simple Connect & Anbieter

### Simple Connect (3 Schritte)

1. **Anbieter** — Aus 22+ Anbieter-Presets wählen (offiziell, globale AI, China-AI, Aggregatoren/Relays) oder eigenen Endpunkt definieren
2. **Schlüssel** — API-Key speichern (Keychain unter macOS, wo unterstützt)
3. **Anwenden** — CLI-Tool und Modell wählen, Konfiguration schreiben

Wechseln Sie zum Tab **Expert** für vollständige Anbieterverwaltung inklusive eigener Anbieter und Relay/Aggregator-Endpunkte.

### Anbieter-Operationen

- **Preset-Katalog** — 22+ kuratierte Anbieter plus eigene Anbieter inklusive Aggregatoren/Relays
- **Enable** — Schreibt live Konfiguration für die gewählte App
- **Add** — Preset oder benutzerdefinierter Endpunkt
- **Edit** — Keys, Base URL, Modelle, gemeinsames Konfigurationsfragment
- **Sort** — Per Drag & Drop sortieren
- **Tray** — Anbietername anklicken für sofortigen Wechsel

### Gemeinsame Konfigurationsfragmente

Beim Anbieterwechsel können Plugin- und Erweiterungsdaten erhalten bleiben:

1. Anbieter bearbeiten → **Shared config panel** → **Extract from current provider**
2. Beim Anlegen eines neuen Anbieters **Write shared config** aktiviert lassen (Standard)

### Unterstützte Apps

Claude Code · Claude Desktop · Codex · Gemini CLI · OpenCode · OpenClaw · Hermes

### Deep Link

Import per URL: `OpenSunstar://import/...` (Anbieter, MCP, Prompts, Skills).

---

## 3. Agent-Konfiguration

### MCP

- **MCP panel** — Server pro App hinzufügen, aktivieren, importieren
- **Discovery** — Registry durchsuchen und Vorlagen installieren
- **Sync toggles** — Bidirektionale Synchronisation zwischen OpenSunstar-DB und live App-Konfigurationen

### Skills

- **Manage** — Installierte Skills, Aktivierung pro App, Batch-Operationen
- **Discover** — skills.sh, ClawHub, ModelScope, benutzerdefinierte Git-Repos
- **Install** — GitHub-Repo, ZIP-Upload, Ein-Klick aus Discovery
- Standard-Speicher: `~/.OpenSunstar/skills/` (Symlink oder Kopie je nach Einstellung)

### Prompts & Regeln

- Markdown-Editor für CLAUDE.md / AGENTS.md / GEMINI.md und Äquivalente
- Aktivieren synchronisiert in live Dateien; Backfill-Schutz beim Lesen

### Weitere Agent-Tools

| Feature | Beschreibung |
| ------- | ------------ |
| **Commands** | Benutzerdefinierte Slash-Befehle |
| **Hooks** | Lifecycle-Hook-Skripte |
| **Ignore** | Ignore-Regeln für Tools |
| **Permissions** | Tool-Berechtigungs-Presets |
| **Subagents** | Agent-Definitionen |
| **Sessions** | Konversationsverlauf durchsuchen und wiederherstellen |
| **OpenClaw workspace** | AGENTS.md, SOUL.md usw. bearbeiten |

---

## 4. Project Cockpit

Der Seitenleisteneintrag **Project Cockpit** (项目驾驶舱) ist ein **Multi-Repo-AI-Entwicklungs-Cockpit**, kein Drag-and-Drop-Task-Board.

### Projekte hinzufügen

1. Seitenleiste → **My Projects → Projekt hinzufügen** oder Project Cockpit → Projekt hinzufügen
2. Name und lokalen Git-Repository-Pfad eingeben
3. **Refresh metrics** klicken, um Codezeilen und Git-Statistiken zu scannen

### Metriken (7-Tage-Fenster)

Diese teilen sich dieselbe **7-Tage-Commit-Anzahl**:

- Übersichtskarte „Commits in den letzten 7 Tagen“
- Project-Cockpit-Matrix X-Achse
- KI-generierter Wochenbericht

Die Gesundheitsbewertung referenziert weiterhin **30-Tage**-Commits für Trendregeln.

Siehe [kanban.md](../kanban.md) für Architektur und Persistenz (SQLite + localStorage).

### KI-Einblicke

- Project-Cockpit-Zusammenfassung, Gesundheitsaufschlüsselung, Wochenbericht
- Erfordert konfigurierten KI-Anbieter unter Einstellungen → AI provider

---

## 5. Proxy & Failover

### Lokaler Routing-Proxy

- Formatkonvertierung zwischen API-Stilen (Anthropic ↔ OpenAI usw.)
- Request-Rectifier für Upstream-Kompatibilität
- Aktivieren unter Einstellungen → Proxy oder Anbieterpanel

### Failover

- Backup-Anbieter-Warteschlange mit automatischem Wechsel bei Fehler
- Circuit-Breaker-Schwellenwerte konfigurierbar
- Anbieter-Gesundheitsstatus in der UI

### App-spezifische Übernahme

Der Proxy kann Claude, Codex oder Gemini unabhängig ansprechen, bis auf einen einzelnen Anbieter.

---

## 6. Nutzung & Budget

### Nutzungs-Dashboard

- Ausgaben, Anfragenanzahl, Token-Nutzung über die Zeit
- Modellpreise pro Modell überschreibbar
- Datenquellen: Proxy-Logs, OpenCode-Sitzungen, optionale offizielle Abo-Kontingent-Vorlage

### Budgetwarnungen

Tägliche / monatliche USD-Limits pro Anbieter; Warnungen über Systemereignisse.

---

## 7. Sync & Collaboration

### Cloud-Sync

- **WebDAV** — Manueller Upload/Download + optional Auto-Sync
- **S3-kompatibel** — AWS, R2, MinIO, OSS, COS, OBS Presets
- Nur ein aktives Cloud-Backend gleichzeitig

### Konfigurationsverzeichnis

`~/.OpenSunstar` über Einstellungen → Directories auf Dropbox, iCloud, OneDrive oder NAS zeigen.

### Import / Export

- Vollständiges SQL-Backup exportieren (Anbieter, MCP, Prompts, Skills, Einstellungen)
- Import stellt aus Backup-Datei mit Bestätigung wieder her

---

## 8. Einstellungen & Datenpfade

| Pfad | Inhalt |
| ---- | ------ |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite — Anbieter, MCP, Prompts, Skills, Projekte, KI-Cache |
| `~/.OpenSunstar/settings.json` | UI-Einstellungen |
| `~/.OpenSunstar/backups/` | Auto-Backups (letzte 10) |
| `~/.OpenSunstar/skills/` | Skill-Speicher |
| `~/.OpenSunstar/skill-backups/` | Backups vor Deinstallation (letzte 20) |

### Sprachen

简体中文 · 繁體中文 · English · 日本語

### Themes

Dark · Light · Follow system

---

## 9. FAQ

**Terminal nach Wechsel neu starten?**
In der Regel ja. Claude Code Hot-Switch ist die Ausnahme.

**Aktiven Anbieter löschen?**
Mindestens eine aktive Konfiguration bleibt erhalten, damit die CLI nutzbar bleibt. Ungenutzte Apps stattdessen in den Einstellungen ausblenden.

**Zurück zur offiziellen Anmeldung?**
Official-Preset hinzufügen → wechseln → Logout/Login-Flow der CLI ausführen.

**Wo liegen Project-Cockpit-Daten?**
Projekte in SQLite-Tabelle `projects`; Phase/Fortschritt in localStorage (Migration geplant).

---

[← Handbuch-Index](../README.md) · [Release Notes v0.1.0](../release-notes/v0.1.0-de.md)
