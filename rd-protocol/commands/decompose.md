# /rd:decompose — 按应用拆解

> 阶段 2：把需求按应用拆解，每个应用产独立开发契约。

## 先读规则

1. 读 `.specs/<change-id>/ANALYSIS.md`（分析结果）
2. 读 `knowledge/applications/` 现有应用清单
3. 读 `knowledge/ROUTING.md` 确认应用归属

## 执行

- 识别需求涉及的应用清单
- 每个应用拆出独立 requirement（开发契约）
- 标注跨应用依赖与顺序约束
- 识别风险与前置条件

## 何时澄清

- 应用归属不清 → 暂停
- 跨应用职责边界冲突 → 暂停

## 何时停止

- 无法明确应用清单 → 停止，回 analyze 补充

## 产物

- `.specs/<change-id>/DECOMPOSITION.md`（应用清单 + 依赖 + 风险）
- `.specs/<change-id>/applications/{app}/requirement.md`（每应用开发契约，最核心产物）

## 人工确认

**是** — 产出 DECOMPOSITION + 应用级 requirement 后暂停，等人工确认拆解合理后继续。
