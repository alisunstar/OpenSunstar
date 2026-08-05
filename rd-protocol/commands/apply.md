# /rd:apply — 代码实现

> 阶段 4：按应用级 requirement 契约编码实现。

## 先读
- `.specs/<change-id>/applications/{app}/requirement.md`（契约）
- `knowledge/applications/{app}/tech/`（技术规范）

## 执行
- 按契约实现代码
- 遵循 tech 规范
- 不偏离契约——若需偏离，标记后回 validate 处理

## 产物
代码变更（git diff）

## 人工确认
否 — 实现后直接进入 validate 对账。
