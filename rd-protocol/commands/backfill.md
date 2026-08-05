# /rd:backfill — 知识回补

> 阶段 7：把本次需求过程中产生的新知识回补到知识库候选区，闭环最后一环。

## 先读规则

1. 读 `.specs/<change-id>/IMPLEMENTATION-CHECK.md`（对账结果，识别新知识）
2. 读 `knowledge/KNOWLEDGE-RULES.md`（回补约定）
3. 读 `knowledge/candidate/`（现有候选，避免重复）

## 执行

- 识别本次需求产生的新知识（业务理解、技术决策、踩坑经验、模板更新）
- 每条知识标 source=evidence/confidence，写入候选清单
- 候选内容写入 `knowledge/candidate/` 目录（经候选管道）

## 回补路径

```
本命令产 KNOWLEDGE-BACKFILL.md
  → 候选写入 .opensunstar/wiki/candidates/
    → GUI 验收导入候选到正式 wiki
      → 建立知识基线（baseline synced）
        → 下一需求 ROUTING 命中已回补知识
```

## 何时澄清

- 候选与现有正式知识冲突 → 暂停，等 owner 裁决

## 何时停止

- 无新知识可回补 → 正常结束（不是所有需求都产回补）

## 产物

- `.specs/<change-id>/KNOWLEDGE-BACKFILL.md`（回补候选清单 + 内容）
- `knowledge/candidate/{candidate-id}.md`（候选文件，标 sourceType=backfill）

## 人工确认

**是** — 产出回补候选后暂停，等 GUI 验收导入后建立基线。

## 闭环意义

回补是三层资产闭环的最后一环——RD 过程产出的知识回补到 wiki，wiki 的 ROUTING 又指导下一轮 RD，形成复利。
