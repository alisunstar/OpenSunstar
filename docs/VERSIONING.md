# OpenSunstar 版本治理

OpenSunstar 使用 **两套独立版本号**，请勿混为一谈。

## 1. 应用版本（SemVer）

- **位置：** `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`
- **格式：** `MAJOR.MINOR.PATCH`（如 `1.1.2`）
- **含义：** 面向用户的发布版本，对应 GitHub Release、更新器、README 徽章
- **何时递增：**
  - **PATCH：** Bug 修复、文档、无行为变化的内部重构
  - **MINOR：** 新功能、UI 改进、向后兼容的数据迁移
  - **MAJOR：** 破坏性变更、大规模架构调整

**发布命令（自动递增，无需手写版本号）：**

```bash
# 自动读取 package.json 当前版本并递增
pnpm release:patch     # 1.1.2 -> 1.1.3（bug 修复）
pnpm release:minor     # 1.1.2 -> 1.2.0（新功能）
pnpm release:major     # 1.1.2 -> 2.0.0（破坏性变更）

# 也支持显式版本或预览
node scripts/release-bump.mjs 1.4.0        # 指定版本
node scripts/release-bump.mjs patch --dry-run  # 只预览不写入
node scripts/release-bump.mjs patch --commit   # 递增并自动 git commit
```

`release:bump` 会同步更新：

- **package.json / tauri.conf.json / Cargo.toml / Cargo.lock / README 徽章**
- **CHANGELOG.md**：将 `## [Unreleased]` 提升为 `## [X.Y.Z] - 日期`，并插入空的 `[Unreleased]`
- **docs/release-notes/vX.Y.Z-en.md / -zh.md**（从 Unreleased 内容生成初稿）
- **docs/release-notes/README.md** 索引表

### CHANGELOG 日常写法

开发期间把变更写在 `CHANGELOG.md` 顶部的 `[Unreleased]` 下（[Keep a Changelog](https://keepachangelog.com/) 格式）：

```markdown
## [Unreleased]

### Fixed
- 修复某某问题

### Changed
- 调整某某行为
```

发版时执行 `pnpm release:patch`（或 minor/major），脚本会自动归档并生成 release notes 初稿；发版前可再润色 highlights。

随后：

```bash
pnpm release:tag       # 不带参数时自动读取 package.json 版本打 tag
```

## 2. 数据库 Schema 版本

- **位置：** `src-tauri/src/database/mod.rs` → `SCHEMA_VERSION`
- **格式：** 整数（如 `28`）
- **含义：** SQLite 迁移链版本；每次表结构变更 +1
- **与用户版本无关：** 内部迭代 20+ 次 schema 迁移而应用仍为 `0.1.0` 是正常现象，但应在发版时同步更新应用版本与 release notes

**规则：**

- 每个 schema 变更必须有 `migrate_vN_to_vN+1` 函数与测试
- 不可逆迁移（DROP TABLE）必须在迁移前自动备份（已有逻辑）

## 3. 查询当前版本

应用内可通过 Tauri 命令 `get_build_info` 获取：

```json
{ "appVersion": "1.1.2", "schemaVersion": 28 }
```

## 4. 发版检查清单

1. 在 `CHANGELOG.md` 的 `[Unreleased]` 中写好本次变更
2. `pnpm release:patch`（或 `release:minor` / `release:major` / 显式版本）
3. 审阅自动生成的 `docs/release-notes/vX.Y.Z-*.md`，按需补充 highlights / 升级说明
4. （可选）补充 zh-TW / ja / de release notes
5. `pnpm typecheck && pnpm test:unit && cargo test`
6. `pnpm release:tag` 并发布 GitHub Release

## 5. 为何曾出现 v0.1.0 + schema v27 错位

早期策略是「首版公开前集中迭代 schema」，应用 SemVer 停留在 `0.1.0`。这导致：

- 用户以为产品未更新，实际 DB 已迁移 27 次
- release notes 写「首版无迁移」，与 schema 历史矛盾

**改进策略（自 v1.1.2 起）：**

- 每 2–4 周或每批用户可见功能合并后发 **MINOR** 版本
- schema 变更在 CHANGELOG 中单列「Database」小节
- `get_build_info` 在关于页展示，便于支持排障
