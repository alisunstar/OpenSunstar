# Knowledge Backfill — CHG-p1-rd-adapters

> /rd:backfill 产物 · 稳定结论回补清单

| # | 稳定结论 | 证据 | 目标 | confidence |
|---|---------|------|------|-----------|
| K1 | 受管表区幂等合并模式（routing-auto 标记）可复用于其他索引类知识 | knowledge_routing 单测+dogfood | wiki/runbooks 候选 | high |
| K2 | UI 取完整 preset 必须用 fullPreset（Summary 无 stages）——TS 隐式 any 陷阱 | tsc 报错修复史 | wiki/runbooks 候选 | high |
| K3 | readiness 未采纳维度 not_required 满分是扩容唯一不破坏 100 分制的语义 | agent_readiness 单测 | wiki/decisions 候选 | high |
| K4 | lib 私有 services 对 os CLI 暴露走 pub use 重导出，勿用 open_sunstar_lib::services 路径 | E0603 修复史 | wiki/runbooks 候选 | high |

## 候选写入

- .opensunstar/wiki/candidates/rd-backfill-auto-20260805/（/rd:backfill-auto 执行）
