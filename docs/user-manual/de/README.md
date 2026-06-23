# OpenSunstar Benutzerhandbuch (Deutsch)

**Version:** v0.1.0 · **Lizenz:** MIT

> Native Desktop-Steuerzentrale für Claude Code, Claude Desktop, Codex, Gemini CLI, OpenCode, OpenClaw und Hermes.

---

## Inhaltsverzeichnis

1. [Erste Schritte](#1-erste-schritte)
2. [Simple Connect & Anbieter](#2-simple-connect--anbieter)
3. [Agent-Konfiguration](#3-agent-konfiguration)
4. [Portfolio-Dashboard](#4-portfolio-dashboard)
5. [Proxy & Failover](#5-proxy--failover)
6. [Nutzung & Budget](#6-nutzung--budget)
7. [Sync & Backup](#7-sync--backup)
8. [Einstellungen & Datenpfade](#8-einstellungen--datenpfade)
9. [FAQ](#9-faq)

Verwandt: [v0.1.0 Release Notes](../release-notes/v0.1.0-de.md) · [Portfolio-Modul](../kanban.md)

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
2. Nutzen Sie **Simple Connect** (Seitenleiste → 快速接入 / API Connect) für die geführte Einrichtung.
3. Wechseln Sie Anbieter in der Hauptoberfläche oder über die **Systemleiste**.
4. Starten Sie das Terminal für die meisten CLIs neu (Claude Code unterstützt **Hot-Switch**).

### Seitenleiste im Überblick

| Bereich | Zweck |
| ------- | ----- |
| **API Connect** | Simple-Connect-Assistent + Experten-Anbieterpanel |
| **Agent config** | MCP, Skills, Prompts, Befehle, Hooks usw. |
| **Portfolio** | Multi-Repo-Git-Dashboard |
| **Sync & backup** | WebDAV / S3 / Export |
| **Settings** | Sprache, Proxy, Verzeichnisse, Über |

---

## 2. Simple Connect & Anbieter

### Simple Connect (3 Schritte)

1. **Anbieter** — Preset wählen (DeepSeek, GLM, benutzerdefiniert OpenAI-kompatibel usw.)
2. **Schlüssel** — API-Key speichern (Keychain unter macOS, wo unterstützt)
3. **Anwenden** — CLI-Tool und Modell wählen, Konfiguration schreiben

Wechseln Sie zum Tab **Expert** für die vollständige Anbieterverwaltung.

### Anbieter-Operationen

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

## 4. Portfolio-Dashboard

Der Seitenleisteneintrag **Portfolio** (项目组合) ist ein **Multi-Repo-Entwicklungs-Cockpit**, kein Drag-and-Drop-Task-Board.

### Projekte hinzufügen

1. Seitenleiste → **+** oder Portfolio → Projekt hinzufügen
2. Name und lokalen Git-Repository-Pfad eingeben
3. **Refresh metrics** klicken, um Codezeilen und Git-Statistiken zu scannen

### Metriken (7-Tage-Fenster)

Diese teilen sich dieselbe **7-Tage-Commit-Anzahl**:

- Übersichtskarte „Commits in den letzten 7 Tagen“
- Portfolio-Matrix X-Achse
- KI-generierter Wochenbericht

Die Gesundheitsbewertung referenziert weiterhin **30-Tage**-Commits für Trendregeln.

Siehe [kanban.md](../kanban.md) für Architektur und Persistenz (SQLite + localStorage).

### KI-Einblicke

- Portfolio-Zusammenfassung, Gesundheitsaufschlüsselung, Wochenbericht
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

## 7. Sync & Backup

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

**Wo liegen Portfolio-Daten?**  
Projekte in SQLite-Tabelle `projects`; Phase/Fortschritt in localStorage (Migration geplant).

---

[← Handbuch-Index](../README.md) · [Release Notes v0.1.0](../release-notes/v0.1.0-de.md)
