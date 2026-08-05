# Change — CHG-p05-rd-loop-patch

> /rd:verify-prd 产物 · 输入质量门禁

## 背景

P0 独立验收「有条件通过」，暴露 7 项问题；产品评审决策：分发路径 rd-protocol 采纳 B2（独立可发布 skill 包）、knowledge recipe 采纳 A2（转 .recipe.md 复用 recipe_composer 外插拔路径）、rd-loop 保持业务语义命名并显式声明脱离 S1—S5。本变更 = P1-0（分发路径 A/B + dogfood 实证 + P0.5 补丁 + 提交落库）。

## 功能点

1. A2：recipe_composer 增加 projectTemplates 携带扩展（纯搬运、safe-install、路径安全校验、审计纳入）+ 3 单测。
2. A2：knowledge-baseline.recipe.md 资产 + 生成脚本（SSOT=assets/recipes/knowledge/*.tmpl）。
3. B2：rd-protocol 打包脚本（distrib/rd-protocol-<ver>.zip）+ SKILL.md 分发表述修正 + work.md 路由命令（11 命令齐备）。
4. P0.5：i18n 四键×四语言 + projectWiki.backfillRdLoopHint；WikiPanel 回补说明改 i18n 且按 savedProfile.presetId===rd-loop 条件渲染；FlowPanel 超述文案修正。
5. 治理：rd-loop.json description 声明不适用 S1—S5。
6. Dogfood：本仓库真实跑通七步闭环（recipe 安装实证 + 门禁实跑 + 回补候选）。

## Open Items（已全部关闭）

- OQ-1 分发路径 → B2（zip + Skills 安装 + SSOT 同步）
- OQ-2 recipe 消费路径 → A2；发现 recipe schema 不能携带任意模板，需最小 schema 扩展（projectTemplates），代码仅搬运，合规
- OQ-3 S1—S5 → 保持命名 + description 声明，P1 定独立规则集
- OQ-4/OQ-5 本地化与 work 命令 → 本变更关闭
- OQ-6 dogfood → 本变更即实证
- OQ-7 提交 → 分组提交

## 非目标

- P1 适配器（routing-extractor / backfill 生成器）、UI P1-a（七步图/四工件清单）、P1-b（wiki 引擎选择控件）。
- Skills GUI 安装点击穿透（留人工验收，R5 记 partial）。

## 验收标准

- tsc 0 错；cargo test recipe/flow 全绿；i18n 基线回归 OK。
- `os recipe install --name knowledge-baseline` 在本仓库落出 knowledge/ 五层 + ROUTING（A2 实证）。
- `os flow validate --preset-id rd-loop --target-stage 7-backfill` 放行（exit 0）；缺工件时阻断（exit 1）。
- 回补候选进入 .opensunstar/wiki/candidates/ 且含 index.md。
