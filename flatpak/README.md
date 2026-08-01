# Flatpak Build Guide

This directory contains the Flatpak manifest (`com.OpenSunstar.desktop`) for OpenSunstar, used to convert the generated `.deb` artifact into an installable `.flatpak` package via CI or local builds.

## Product context (source-aligned sidebar baseline)

**Positioning:** Local-first, all-in-one AI coding workflow engineering configuration platform.

**Source title:** 本地优先，一站式统一管理你的 AI 编程工作流工程化配置平台

**Source subtitle:** 跨多项目组合矩阵以AI驱动的项目驾驶舱，一站式帮你基于项目的AI资产配置&工作流编排和跨工具跨设备Agent扩展配置同步

**Source provider copy:** 预设22+供应商，支持用户自定义配置更多供应商（含聚合/中转站）

This README follows the real sidebar: Project Cockpit, My Projects, Project Config (AI Asset Config / Workflow Orchestration), Agent Config (MCP / Skills / Prompt & Rules / Commands / Hooks / Ignore / Permissions / Subagents / Convert), AI Models (Quick Start / Context / AI Tokens), Sync & Collaboration, and Settings. Provider copy is unified as: **22+ preset providers with user-defined custom providers, including aggregators/relays**.

## Dependencies

- `flatpak`
- `flatpak-builder`
- Flathub remote (for installing `org.gnome.Platform//46` runtime)

For Ubuntu/Debian:

```bash
sudo apt install flatpak flatpak-builder
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install -y --user flathub org.gnome.Platform//46 org.gnome.Sdk//46
```

## Local Build (Generate .flatpak from .deb)

1) Build the deb on Linux first:

```bash
pnpm tauri build -- --bundles deb
```

2) Copy the generated deb to this directory:

```bash
cp "$(find src-tauri/target/release/bundle -name '*.deb' | head -n 1)" flatpak/OpenSunstar.deb
```

3) Build the local Flatpak repository and export the `.flatpak`:

```bash
flatpak-builder --force-clean --user --disable-cache --repo flatpak-repo flatpak-build flatpak/com.OpenSunstar.desktop.yml
flatpak build-bundle --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo flatpak-repo OpenSunstar-Linux.flatpak com.OpenSunstar.desktop
```

4) Install and run:

```bash
flatpak install --user ./OpenSunstar-Linux.flatpak
flatpak run com.OpenSunstar.desktop
```

## Permissions Note

The current manifest uses `--filesystem=home` by default for "download and run" convenience, allowing the app to directly read/write CLI configuration files and app data on the host (and supporting the "directory override" feature).

If you prefer minimal permissions (e.g., for Flathub submission or security concerns), you can replace `--filesystem=home` in `flatpak/com.OpenSunstar.desktop.yml` with more precise grants:

```yaml
  - --filesystem=~/.OpenSunstar:create
  - --filesystem=~/.claude:create
  - --filesystem=~/.claude.json
  - --filesystem=~/.codex:create
  - --filesystem=~/.gemini:create
  - --filesystem=~/.config/opencode:create
  - --filesystem=~/.openclaw:create
```

Note: Flatpak's `:create` modifier only works with directories, not files. Therefore, `~/.claude.json` cannot use `:create`. If this file doesn't exist on the user's machine, the app may not be able to create it with restricted permissions. Users should either run Claude Code once to generate it, or manually create an empty JSON file (content: `{}`).

If you plan to publish on Flathub or want stricter permission control, adjust the `finish-args` in `flatpak/com.OpenSunstar.desktop.yml` accordingly.
