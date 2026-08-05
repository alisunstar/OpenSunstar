# /rd:backfill-auto — 回补生成器适配器（外置）

> 知识×RD 连接适配器：读取本 change 的过程资产（REVIEW.md / IMPLEMENTATION-CHECK.md / .specs/LESSONS.md），自动产出**回补候选**写入 wiki 候选管道。核心只承接候选（发现/导入/lint/验收），生成智能在外侧（K1/K2）。

## 先读规则

1. 读 `.specs/<change-id>/REVIEW.md`、`IMPLEMENTATION-CHECK.md`、`.specs/LESSONS.md`
2. 读 knowledge/KNOWLEDGE-RULES.md 与 wiki SCHEMA（候选页格式）
3. 只提取“稳定结论”：被证据支撑、与单次需求解耦、可复用的业务/技术事实

## 执行

1. 从对账 changed/blocked 的裁决记录、REVIEW 的 4R 发现、LESSONS 的失败记录中抽取稳定结论；
2. 每条结论标：证据（产物路径+条目）、目标页类型（runbook/decision/flow）、confidence；
3. 一次性需求特例、未裁决推断一律剔除；
4. 写候选目录（generator-adapter 协议）：
   ```text
   .opensunstar/wiki/candidates/rd-backfill-auto-<YYYYMMDD>/
   ├── candidate.json   # engine: "rd-backfill-auto", source_commit, created_at
   └── wiki/
       ├── index.md
       └── runbooks/<page>.md   # wiki schema frontmatter
   ```
5. 不直写正式 wiki/，不宣称已同步（控制面专属权力）。

## 何时停止

- 三类过程资产均缺失 → 停止，提示先跑 /rd:validate 与 /rd:backfill（人工版）。
- 无可复用稳定结论 → 产出空索引候选并说明理由，不凑数。

## 人工确认

**是** — 候选经 GUI 验收导入后才建基线；comparable=false 时不下优劣结论。

## 与 /rd:backfill 的关系

- /rd:backfill = 人工版（人主导列清单）；本命令 = 自动版（Agent 起草）；
- 两者产物同管道同门禁，可并存对照（同 commit 质量对照机制既有）。
