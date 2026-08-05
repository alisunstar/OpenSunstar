# /rd:work — 路由命令（唯一入口）

> 通用路由命令：根据用户输入与当前 .specs/<change-id>/ 状态，自动推断意图并引导下一步。使用者事实上只需输入这一个命令。

## 先读规则

1. 读 `STATE.md`（active change）与 `.specs/<change-id>/` 已有产物清单
2. 读 knowledge/ROUTING.md（若存在）建立业务定位
3. 对照 rd-loop 阶段链：0-verify-prd → 1-clarify-analyze → 2-decompose → 3-verify-requirement → 4-apply → 5-validate → 6-review-release → 7-backfill

## 执行

按以下顺序推断下一步并给出「命令 + 推荐提示词」：

| 当前状态 | 引导下一步 |
|---------|-----------|
| 无 CHANGE.md | `/rd:verify-prd`（输入质量门禁） |
| 有 CHANGE.md，无 ANALYSIS.md | `/rd:clarify` 或 `/rd:analyze` |
| 有 ANALYSIS.md，无 DECOMPOSITION.md | `/rd:decompose` |
| 有 DECOMPOSITION/REQUIREMENT，未确认 | `/rd:verify-requirement`（人工确认） |
| 契约已确认，未实现 | `/rd:apply` |
| 已实现，无 IMPLEMENTATION-CHECK.md | `/rd:validate`（五状态对账） |
| 对账已确认，无 REVIEW.md | `/rd:code-review` |
| REVIEW 已确认 | `/rd:release-plan`，随后 `/rd:backfill` 收尾 |

## 何时澄清

- 用户输入无法唯一映射到阶段时，列出候选并暂停等选择。

## 何时停止

- 任一上游门禁未通过（verify-prd / verify-requirement / validate 存在 blocked 或未确认 changed）→ 停止并指向对应门禁，不自动绕过。

## 产物

- 无新增产物；本命令只产出「下一步引导」。

## 人工确认

**否**（引导本身不改变状态；被引导到的命令按其自身矩阵确认）。
