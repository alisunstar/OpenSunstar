# /rd:validate — 五状态对账

> 阶段 5：对照 requirement 契约，校验实现完成度，产接续产物。

## 先读规则

1. 读 `.specs/<change-id>/applications/{app}/requirement.md`（契约）
2. 读 `.specs/<change-id>/DECOMPOSITION.md`（拆解清单）
3. 读当前代码实现（知识提供上下文，代码是事实）

## 执行

逐项对照契约，标注五状态：
- **done** — 已实现且符合契约
- **partial** — 部分实现，有缺口
- **todo** — 未开始
- **changed** — 实现偏离契约（需确认是否更新契约）
- **blocked** — 被阻塞，需外部输入

## 何时澄清

- `changed` 项需确认：是更新契约还是修正实现
- `blocked` 项需外部输入

## 何时停止

- 存在 `blocked` 项 → 停止，等外部输入
- `changed` 项未确认 → 停止，等人工裁决

## 产物

- `.specs/<change-id>/IMPLEMENTATION-CHECK.md`（五状态对账表 + 汇总 + 后续行动）
- `.specs/<change-id>/CONTINUE-PROMPT.md`（跨会话接续：当前进度 + 关键上下文 + 未决问题 + 接续指令）

## 人工确认

**是** — 产出对账结果后暂停，等人工确认后继续 code-review 或回 apply 修复。

## 95% 原则

不追求 100% 全 AI——剩下两行手改更快就手改，不必强求对账全 done。
