# /rd:code-review — 代码审查

> 阶段 6a：多维度代码审查（4R lenses）。

## 先读
- `.specs/<change-id>/IMPLEMENTATION-CHECK.md`
- git diff

## 执行
按 4R lenses 审查：
- **review-risk** — 风险点（边界/异常/安全）
- **resilience** — 健壮性（容错/恢复）
- **readability** — 可读性（命名/结构/注释）
- **reliability** — 可靠性（测试/一致性）

## 产物
- `.specs/<change-id>/REVIEW.md`

## 人工确认
**是** — 审查问题需修复后确认。
