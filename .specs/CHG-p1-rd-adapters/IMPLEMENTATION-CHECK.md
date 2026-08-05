# Implementation Check — CHG-p1-rd-adapters

> /rd:validate 产物 · 五状态对账（对照 REQUIREMENT.md）· os rd validate schema 校验通过（exit 0）

## 对账结果

| 应用 | 契约项 | 状态 | 证据 | 缺口/偏离说明 |
|------|--------|------|------|-------------|
| opensunstar | R1 适配器协议 | done | 13 命令齐备；B2 安装后技能列表可发现 rd-protocol | — |
| opensunstar | R2 os rd validate | done | 单测 4/4；本文件 schema 校验 exit0；diffStats 20 项 | — |
| opensunstar | R3 os wiki routing | done | 单测 2/2（含幂等与人编保留）；dogfood 3 锚点入表、二次 routingWritten:false | — |
| opensunstar | R4 G-RD1—4 | done | strict 实跑：G-RD 无警告（ROUTING 在、rd-protocol 已装、STATE 一致、schema 过）；exit2 仅来自标准治理项（DESIGN.md 等），属预期 | 标准治理项警告非本变更范围 |
| opensunstar | R5 UI P1-a | done | fullPreset 数据驱动；tsc 绿 | M1 点击穿透待人工验收 |
| opensunstar | R6 UI P1-b | done | engine 下拉+透传；默认 builtin | 点击穿透待人工验收 |
| opensunstar | R7 readiness 扩容 | done | 93+7=100；not_required 语义；单测 8/8 | — |
| opensunstar | R8 回归 | done | tsc 0 错；i18n 基线 OK；cargo 串行 2184/2184 | 并行 flake=sync_protocol 既有缺陷，记待修 |

## 汇总

- done: 8
- partial: 0
- todo: 0
- changed: 0
- blocked: 0

## 后续行动

- M1/M2 人工点击验收；sync_protocol 测试隔离修复（P2 或独立修复单）。
