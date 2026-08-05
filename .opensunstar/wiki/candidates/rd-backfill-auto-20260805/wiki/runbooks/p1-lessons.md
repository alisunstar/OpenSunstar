---
title: P1 四则工程经验
type: runbook
status: draft
source_files:
  - src-tauri/src/services/knowledge_routing.rs
  - src/components/projects/ProjectFlowOrchestratorPanel.tsx
  - src-tauri/src/ai/agent_readiness.rs
  - src-tauri/src/lib.rs
last_verified_commit: 419eda9
last_verified: 2026-08-05
tags:
  - rd-loop
  - readiness
---

# P1 四则工程经验

> CHG-p1-rd-adapters 回补 · 证据见 .specs/CHG-p1-rd-adapters/

1. **受管表区幂等合并**：索引类知识写回用 `<!-- opensunstar:routing-auto -->` 受管标记，人编区永不覆写；二次运行零写入。
2. **fullPreset 取数**：UI 渲染阶段/工件必须取 fullPreset（WorkflowPreset），selectedPreset 为 Summary 无 stages——隐式 any 陷阱。
3. **readiness 扩容语义**：未采纳维度记 not_required 满分，是唯一不破坏 100 分制的扩容方式。
4. **lib 暴露路径**：os CLI 消费 services 新模块走 `pub use services::X` 重导出；`open_sunstar_lib::services::…` 在 lib 内部不可用（E0603）。

## 使用前必须核对

- 本页为 draft，经验收导入后转 active。
