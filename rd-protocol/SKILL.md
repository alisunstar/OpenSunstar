---
name: rd-protocol
version: 1.0.0
description: RD 交付协议包——/rd:* 命令链定义 AI 研发交付全流程：需求验证→知识加载→拆解→契约→实现→校验→回补。补全三层资产的命令协议层业务语义。
author: OpenSunstar
license: Apache-2.0
---

# RD Protocol — AI 研发交付命令协议

> 本 skill 定义 `/rd:*` 命令链，让外部 CLI（Claude Code/Codex/Gemini CLI 等）按协议跑通 AI 研发交付全链路。
> OpenSunstar 只做协议分发与产物门禁，不持有 agent 运行时（C1 约束）。

## 七步闭环

```
verify-prd → clarify/analyze → decompose → verify-requirement → apply → validate → code-review/release → backfill
```

## 命令清单

| 命令 | 阶段 | 产物 | 人工确认 |
|------|------|------|---------|
| `/rd:work` | 路由入口 | 下一步引导（命令+推荐提示词） | 否 |
| `/rd:verify-prd` | 输入质量门禁 | CHANGE.md | 是 |
| `/rd:clarify` | 澄清 | clarification.md | 是 |
| `/rd:analyze` | 分析（读 ROUTING 加载知识） | analysis.md | 否 |
| `/rd:decompose` | 按应用拆解 | DECOMPOSITION.md + applications/{app}/requirement.md | 是 |
| `/rd:verify-requirement` | 契约确认 | — | 是 |
| `/rd:apply` | 代码实现 | — | 否 |
| `/rd:validate` | 五状态对账 | IMPLEMENTATION-CHECK.md + CONTINUE-PROMPT.md | 是 |
| `/rd:code-review` | 代码审查 | REVIEW.md | 是 |
| `/rd:release-plan` | 发布计划 | — | 是 |
| `/rd:backfill` | 知识回补 | KNOWLEDGE-BACKFILL.md → candidates | 是 |

## 通用协议规则

每条命令遵循以下约定：

1. **先读规则**：执行前先读 knowledge/ROUTING.md 定位相关知识，按渐进式加载层级读取
2. **何时澄清**：遇到歧义/缺失/冲突时暂停，产出 clarification，等人工确认后继续
3. **何时停止**：门禁未通过（verify-prd/verify-requirement/validate）时停止，不自动绕过
4. **产物写哪**：所有产物落盘到 `.specs/<change-id>/`
5. **需人工确认**：上表"人工确认=是"的命令，产出后暂停等确认
6. **fail-fast**：能在早期阶段暴露的问题不拖到后期

## 与 rd-loop preset 的关系

- 本协议包是命令协议层（定义 /rd:* 怎么工作）
- rd-loop preset 是 RD 过程资产层（定义阶段/工件/门禁）
- knowledge recipe 是知识资产层（定义五层目录/ROUTING/回补）
- 三者共同构成三层资产 MVP 闭环

## 分发方式

rd-protocol 作为**独立可发布的 skill 包**分发（B2）：

- 发布物：`distrib/rd-protocol-<version>.zip`（`node scripts/package-rd-protocol.mjs` 生成）；
- 安装：经 OpenSunstar Skills 管理界面「从 ZIP 安装」/仓库安装，或手工将本目录放入 `~/.agents/skills/rd-protocol/`；
- 安装后经 skills SSOT（`~/.OpenSunstar/skills/`）同步到各目标 CLI，并由 managed 标记 + agent-context 桥向 AGENTS.md 注入读取指引。

## 约束

- 本协议是 Markdown 资产，不含可执行代码（C2 约束）
- 命令由外部 CLI 执行，OpenSunstar 不持有运行时（C1 约束）
- 不依赖额外基础设施，纯文件协议（C3 约束）
