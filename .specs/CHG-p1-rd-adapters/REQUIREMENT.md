# Requirement — CHG-p1-rd-adapters（OpenSunstar 应用级开发契约）

> /rd:decompose 产物 · 经 /rd:verify-requirement 人工确认（排期评审）

## 目标 / 非目标

- 目标：A1—A8 验收项落地；非目标：P2 内容。

## 契约项

| # | 契约项 | 验收标准 |
|---|--------|---------|
| R1 | extract-routing/backfill-auto 协议 | 命令文档齐备；_meta 13 命令；B2 安装后可发现 |
| R2 | os rd validate | 4 类 fixture 单测；dogfood exit0 |
| R3 | os wiki routing | 合并+幂等单测；dogfood 3 锚点入表 |
| R4 | G-RD1—4 | strict+rd-loop 实跑：G-RD 全过（其余治理项警告属预期） |
| R5 | UI P1-a | fullPreset 数据驱动；tsc 绿；M1 点击待验 |
| R6 | UI P1-b | engine 下拉+透传；默认 builtin 不改变现有行为 |
| R7 | readiness 扩容 | 93+7=100；未采纳 not_required；8/8 单测 |
| R8 | 回归 | tsc/i18n/cargo 串行全绿 |

## 回补预告

- 受管表区幂等合并模式；fullPreset 取数陷阱；readiness not_required 语义。
