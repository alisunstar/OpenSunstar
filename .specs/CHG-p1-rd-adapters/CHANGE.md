# Change — CHG-p1-rd-adapters

> /rd:verify-prd 产物

## 背景

P1 按《P1 实施排期与验收清单》开工：S1 适配器资产、S2 os rd validate、S3 os wiki routing、S4 G-RD1—4、S5 UI P1-a/b、S6 readiness 扩容。

## 功能点

1. /rd:extract-routing 与 /rd:backfill-auto 协议命令（外置适配器，rd-protocol 13 命令）。
2. os rd validate：五状态 schema 校验 + git diff 统计（确定性段）。
3. os wiki routing：正式区锚点合并入 ROUTING 受管表区（幂等、不覆写人编）。
4. flow validate --strict 在 rd-loop 时追加 G-RD1—4。
5. UI：rd-loop 选中时七步条+四工件清单（fullPreset 数据驱动）；wiki 引擎选择（builtin/openwiki）。
6. readiness：wiki 基线健康 4 分 + RD 工件完整度 3 分（未采纳 not_required）。

## Open Items

- OQ-1 sync_protocol 全量并行 flake（既有缺陷）→ 待修清单，不阻塞 P1。
- OQ-2 M1/M2 人工点击验收（UI 运行时）→ 用户验收批次。

## 非目标

- P2 Harness 化内容；适配器自动触发（仍为协议驱动人工/Agent 显式执行）。

## 验收标准

- 单测：rd_validate 4/4、knowledge_routing 2/2、agent_readiness 8/8、cargo 串行 2184/2184。
- tsc 0 错；i18n 基线 OK。
- dogfood2：routing 合并+幂等；rd validate exit0；7-backfill 门禁 exit0；strict 下 G-RD 全过；backfill-auto 候选入管道。
