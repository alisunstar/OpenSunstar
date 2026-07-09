# opensunstar-os

npm wrapper for the **OpenSunstar CLI** (`os`) — downloads the platform binary from [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases) on install.

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

| OS | Arch | Release asset |
|----|------|---------------|
| Windows | x64 | `OpenSunstar-v*-os-windows-x86_64.zip` |
| Linux | x64 | `OpenSunstar-v*-os-linux-x86_64.tar.gz` |
| macOS | arm64 | `OpenSunstar-v*-os-macos-aarch64.tar.gz` |
| macOS | x64 | `OpenSunstar-v*-os-macos-x86_64.tar.gz` |

## Notes

- This package is a **thin Node shim**; the CLI itself is a Rust binary with TUI.
- `postinstall` requires network access to GitHub Releases on first install.
- If your environment uses `npm install --ignore-scripts`, run `npm rebuild opensunstar-os` after install.

## License

MIT
