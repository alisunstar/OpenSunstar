# Decomposition — CHG-p05-rd-loop-patch

> /rd:decompose 产物 · 单应用，无跨应用拆解

## 应用清单

| 应用 | 角色 | requirement |
|------|------|-------------|
| OpenSunstar（本仓库） | 唯一应用 | 见 REQUIREMENT.md |

## 模块拆解

| 模块 | 内容 | 挂载点 |
|------|------|--------|
| M1 A2 携带扩展 | recipe_composer projectTemplates（preview 审计+install 写入+路径校验+渲染）+3 单测 | 扩展点②（recipe 外插拔） |
| M2 A2 资产 | knowledge-baseline.recipe.md + build-knowledge-recipe.mjs | assets/recipes/knowledge/ |
| M3 B2 资产 | package-rd-protocol.mjs + work.md + SKILL/_meta 修正 | 扩展点①（skills SSOT/zip 安装） |
| M4 P0.5 | i18n×4、WikiPanel 条件化、FlowPanel 文案、rd-loop 声明 | 既有 UI/i18n 机械 |
| M5 Dogfood | recipe 安装实证 + 门禁实跑 + 回补候选 | 扩展点③（候选管道） |

## 依赖顺序

M1 → M2（recipe 依赖 schema）→ M5（安装实证）；M3/M4 独立并行。
