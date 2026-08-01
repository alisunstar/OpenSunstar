# opensunstar-os

npm wrapper for the **OpenSunstar CLI** (`os`) — downloads the platform binary from [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases) on install.

## Product context (source-aligned sidebar baseline)

**Positioning:** Local-first, all-in-one AI coding workflow engineering configuration platform.

**Source title:** 本地优先，一站式统一管理你的 AI 编程工作流工程化配置平台

**Source subtitle:** 跨多项目组合矩阵以AI驱动的项目驾驶舱，一站式帮你基于项目的AI资产配置&工作流编排和跨工具跨设备Agent扩展配置同步

**Source provider copy:** 预设22+供应商，支持用户自定义配置更多供应商（含聚合/中转站）

This README follows the real sidebar: Project Cockpit, My Projects, Project Config (AI Asset Config / Workflow Orchestration), Agent Config (MCP / Skills / Prompt & Rules / Commands / Hooks / Ignore / Permissions / Subagents / Convert), AI Models (Quick Start / Context / AI Tokens), Sync & Collaboration, and Settings. Provider copy is unified as: **22+ preset providers with user-defined custom providers, including aggregators/relays**.

The installer verifies the downloaded archive against the SHA256 manifest
included in the published npm package.

## Install

```bash
npm install -g opensunstar-os
# or
pnpm add -g opensunstar-os
```

Then:

```bash
os --version
os doctor --json
os   # full-screen TUI
```

## Pin a specific Release version

```bash
OPEN_SUNSTAR_OS_VERSION=1.1.4 npm install -g opensunstar-os
```

## Skip download (CI / offline)

```bash
OPEN_SUNSTAR_OS_SKIP_DOWNLOAD=1 npm install opensunstar-os
```

Place a prebuilt binary at `vendor/os` (or `vendor/os.exe` on Windows), then run `os` via `node node_modules/opensunstar-os/bin/os.js`.

## Supported platforms

| OS      | Arch  | Release asset                            |
| ------- | ----- | ---------------------------------------- |
| Windows | x64   | `OpenSunstar-v*-os-windows-x86_64.zip`   |
| Linux   | x64   | `OpenSunstar-v*-os-linux-x86_64.tar.gz`  |
| macOS   | arm64 | `OpenSunstar-v*-os-macos-aarch64.tar.gz` |
| macOS   | x64   | `OpenSunstar-v*-os-macos-x86_64.tar.gz`  |

## Notes

- This package is a **thin Node shim**; the CLI itself is a Rust binary with TUI.
- `postinstall` requires network access to GitHub Releases on first install.
- Downloads fail closed if `checksums.json` is missing or the archive hash does not match.
- If your environment uses `npm install --ignore-scripts`, run `npm rebuild opensunstar-os` after install.

## License

Apache-2.0. See the repository [LICENSE](../../LICENSE) and [licensing boundary](../../docs/LICENSING.md).
