# OpenSunstar CLI (`os`) — P1 分发方案

> 目标：在保持 **Rust 原生二进制** 的前提下，提供 Agent/开发者熟悉的安装入口（npm、包管理器、一键脚本）。

## 现状（v1.1.4）

| 方式 | 状态 | 说明 |
|------|------|------|
| GitHub Release 直链 | ✅ 已上线 | `OpenSunstar-v*-os-*.zip` / `.tar.gz` |
| `npm install -g opensunstar-os` | ✅ 已上线 v1.1.4 | Node 薄包装，postinstall 拉 Release 二进制 |
| Scoop | 📋 模板就绪 | `distrib/scoop/opensunstar-os.json` |
| Winget | 📋 模板就绪 | `distrib/winget/OpenSunstar.OpenSunstarCLI/` |
| Homebrew formula（仅 CLI） | ⏳ P2 | GUI 已有 `brew install --cask OpenSunstar` |
| `optionalDependencies` 多包 | ⏳ P2 | 免 postinstall，适配 `--ignore-scripts` 企业环境 |

## P1：npm 包装（`packages/os-cli`）

### 设计

```
opensunstar-os (npm)
├── bin/os.js          → spawn vendor/os[.exe]
├── scripts/postinstall.js → 从 GitHub Release 下载对应平台附件
└── vendor/            → 安装时生成，不入库
```

### 用户命令

```bash
npm install -g opensunstar-os
os doctor --json
```

### 发布流程

1. 打 tag / Release CI 产出 `os-*` 附件（已有）
2. 将 `packages/os-cli/package.json` 的 `version` 与 Release 对齐
3. `cd packages/os-cli && npm publish --access public`
4. （可选）CI 在 `build-os-cli` 成功后用 `NPM_TOKEN` 自动 publish

### 环境变量

| 变量 | 作用 |
|------|------|
| `OPEN_SUNSTAR_OS_VERSION` | 覆盖下载的 Release 版本（默认读 package.json） |
| `OPEN_SUNSTAR_OS_SKIP_DOWNLOAD` | 跳过 postinstall（离线/CI 预置 vendor） |

### 限制

- 首次安装需访问 GitHub Releases
- `pnpm install --ignore-scripts` 默认跳过 postinstall → 需 `npm rebuild opensunstar-os`
- Intel Mac CLI 附件 CI 仍待修复

## P1：Scoop（Windows）

模板：`distrib/scoop/opensunstar-os.json`

```powershell
# 维护者：放入 scoop bucket 后
scoop bucket add opensunstar https://github.com/alisunstar/scoop-bucket  # 待建
scoop install opensunstar-os
```

发布前更新 manifest 中的 `hash`（`scoop hash <zip>`）。

## P1：Winget（Windows）

模板：`distrib/winget/OpenSunstar.OpenSunstarCLI/1.1.4/`

```powershell
# 提交到 microsoft/winget-pkgs 前本地验证
winget validate distrib/winget/OpenSunstar.OpenSunstarCLI/1.1.4
```

更新 `InstallerSha256` 后向 [winget-pkgs](https://github.com/microsoft/winget-pkgs) 提 PR。

用户侧（合并后）：

```powershell
winget install OpenSunstar.OpenSunstarCLI
```

## P2 路线图

| 项 | 收益 |
|----|------|
| `@opensunstar/cli` 作用域包 + optionalDependencies 平台子包 | 企业 CI 友好，无 postinstall |
| `brew install opensunstar-os` formula | macOS 开发者习惯 |
| scoop 官方 bucket / winget-pkgs 合入 | Windows 一键安装 |
| `curl \| sh` / `irm` 安装脚本 | 文档与 Agent 快速引导 |
| Release CI 自动 `npm publish` + hash 回填脚本 | 减少手工步骤 |
| 修复 `os-macos-x86_64` 交叉编译 | Intel Mac 覆盖 |

## 与 GUI 分发关系

- **GUI**：Tauri 安装包（msi/dmg/deb/AppImage）— 不变
- **CLI `os`**：独立二进制，可与 GUI 并存；数据共享 `~/.OpenSunstar/OpenSunstar.db`
- 安装 GUI **不会**自动安装 `os`；各通道独立

## 版本对齐检查清单

- [ ] `src-tauri/Cargo.toml` / `package.json` / `packages/os-cli/package.json` 版本一致
- [ ] GitHub Release 含对应 `os-*` 附件
- [ ] `distrib/scoop` hash、`distrib/winget` SHA256 已更新
- [ ] `npm publish` 完成（若走 npm 通道）
- [ ] README / website 安装说明已更新
