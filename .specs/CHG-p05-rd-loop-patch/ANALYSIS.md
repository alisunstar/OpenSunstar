# Analysis — CHG-p05-rd-loop-patch

> /rd:clarify + /rd:analyze 产物 · 先读 ROUTING，再回代码核对

## ROUTING 命中（渐进式加载）

- 业务身份路由：OpenSunstar 产品研发 → 应用=OpenSunstar（单应用，无跨应用拆解）
- 知识入口：knowledge/ROUTING.md（本变更经 A2 安装脚手架首次落盘）→ knowledge/applications/INDEX.md → 回当前代码核对（代码是事实）

## 影响面（代码入口核对结果）

| 模块 | 入口 | 结论 |
|------|------|------|
| recipe 携带扩展 | src-tauri/src/services/recipe_composer.rs（preview/install/generate/parse） | frontmatter 由 serde_yaml 全结构序列化 → 新字段自动往返；扩展点=preview 临时目录审计段 + install 写入段 |
| skills 安装链路 | services/skill.rs install_from_zip（commands/skill.rs:549 暴露 GUI） | B2 zip 布局要求 SKILL.md 位于 zip 根，scan_skills_in_dir 已支持 |
| 条件化文案数据源 | flowOrchestratorApi.scanProject → SpecsWorkflowIndex.savedProfile.presetId | 零新增 Rust 命令 |
| i18n 治理 | src/i18n/locales/*.json | missing 为 zh∖locale 差集；新键须四语言同加 |

## 澄清记录

- C-1：A2 的 recipe.md 格式不能携带任意模板（artifacts 仅跟踪不创建）→ 最小 schema 扩展 projectTemplates，保持“代码只搬运”边界（K1 合规）。
- C-2：dogfood 的 STATE.md/.specs/ 入库是验收证据，非污染（.gitignore 未排除）。
- C-3：recipe install 刷新 AGENTS.md managed 桥为设计行为，随 dogfood 提交。

## 失败归因预案

若门禁误判 → 归因“纯编码/工件命名”；若 recipe 解析失败 → 归因“schema 扩展”，回退预案=移除 projectTemplates 字段（serde default 向后兼容）。
