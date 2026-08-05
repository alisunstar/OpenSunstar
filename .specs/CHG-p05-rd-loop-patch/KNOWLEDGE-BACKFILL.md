# Knowledge Backfill — CHG-p05-rd-loop-patch

> /rd:backfill 产物 · 稳定结论回补清单（写入 wiki 候选管道）

## 回补项

| # | 稳定结论 | 证据 | 目标页 | 可信度 |
|---|---------|------|--------|--------|
| K1 | recipe 的 projectTemplates 是“外部模板经 recipe 分发”的标准通道（A2）：代码仅搬运、safe-install、路径校验、审计纳入 | 本变更 R1/R2 + 3 单测 + install 实证 | runbooks/rd-loop-p1-0-lessons.md | high |
| K2 | rd-protocol B2 分发三件套：distrib zip（package 脚本）→ Skills 安装（ZIP/仓库/手工 ~/.agents/skills）→ SSOT 同步各 CLI | 本变更 R3/R4 | runbooks/rd-loop-p1-0-lessons.md | high |
| K3 | rd-loop 采用业务语义阶段命名，不适用 standard preset S1—S5；门禁强制以工件必选性为准；独立语义规则集 P1 定义 | rd-loop.json description + 验收决策 C | runbooks/rd-loop-p1-0-lessons.md | high |
| K4 | i18n 治理纪律：新文案四语言同加；missing 计数为 zh∖locale 差集，同加不变属正常；defaultValue 仅兜底 | 本变更 R5 | 不回补（团队纪律，已在 KNOWLEDGE-RULES 语境） | medium |

## 候选写入

- .opensunstar/wiki/candidates/rd-backfill-20260805/（candidate.json + wiki/index.md + wiki/runbooks/rd-loop-p1-0-lessons.md）
- 待 GUI 验收导入后建立基线，供下一需求 ROUTING 命中。
