# Requirement — CHG-p05-rd-loop-patch（OpenSunstar 应用级开发契约）

> /rd:decompose 产物 · 编码前经 /rd:verify-requirement 人工确认（2026-08-05 产品评审：B2/A2/保持命名）

## 目标

关闭 P0 验收问题 1—7 中的 P1-0 范围：分发路径 A/B 落地、P0.5 补丁、dogfood 实证、提交落库。

## 非目标与边界

- 不做 P1 适配器与 UI P1-a/b；不引入 agent 循环/搜索硬编码（K1/K2）；核心改动仅限“搬运”语义。

## 契约项

| # | 契约项 | 验收标准 | 编码前入口 |
|---|--------|---------|-----------|
| R1 | projectTemplates schema + preview/install 携带 + safe-install + 路径校验 | cargo test 新增 3 例全绿；二次安装 skip；`../` 路径报错 | recipe_composer.rs preview/install |
| R2 | knowledge-baseline.recipe.md + 生成脚本 | 脚本可复跑；os recipe install 落出 knowledge/ 8 文件 | assets/recipes/knowledge/ |
| R3 | B2 打包脚本 + zip 布局 | zip 根含 SKILL.md；脚本幂等 | scripts/package-rd-protocol.mjs |
| R4 | rd-protocol 11 命令 + SKILL 分发修正 | work.md 存在；_meta commands=11 | rd-protocol/ |
| R5 | i18n 四键×4 语言 + backfillRdLoopHint | 四语言键齐；i18n:check 基线 OK | src/i18n/locales/ |
| R6 | WikiPanel 条件化 i18n 渲染 | savedProfile.presetId 驱动；tsc 绿 | ProjectWikiPanel.tsx |
| R7 | FlowPanel 超述修正 | 文案与 B2 现状一致 | ProjectFlowOrchestratorPanel.tsx |
| R8 | rd-loop S1—S5 声明 | description 含治理声明 | rd-loop.json |
| R9 | 回归全绿 | tsc/cargo(flow+recipe)/i18n | — |
| R10 | 门禁实跑 + 回补候选 | 7-backfill exit 0；candidates 含 index.md | os flow validate |

## 待确认项

- 无（OQ 全关闭）；GUI 安装点击穿透记入 CONTINUE-PROMPT（不阻塞）。

## 回补预告

- recipe projectTemplates=A2 标准通道；B2 三件套；rd-loop 治理声明；i18n 四语言同加纪律。
