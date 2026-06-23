# 项目组合（Kanban）模块说明

> OpenSunstar v0.1.0 · 自 AIControls v0.2.1 移植并演进

## 定位

**项目组合**是多本地 Git 仓库的组合仪表盘，不是拖拽式任务看板。侧栏入口与页标题统一为「项目组合」。

## 统一指标窗口（7 天）

以下能力共用 **近 7 天 Git 提交数**（`git_commit_count_last_n_days(..., 7)`）：

| 能力 | 说明 |
|------|------|
| 总览卡片「近 7 天提交」 | 各项目 7 天提交求和 |
| 平均活跃度 | 基于 7 天提交分级 |
| 项目组合矩阵 X 轴 | 近 7 天提交数 |
| AI 生成周报 | Prompt 使用 `commit_count_7d` + `weekly_commits` 末项 |

健康评分规则仍参考 **30 天**提交（`commit_count_30d`），与更长窗口的趋势判断互补。

常量：`src/lib/portfolioMetrics.ts` → `PORTFOLIO_COMMIT_WINDOW_DAYS = 7`

## 数据持久化

| 数据 | 存储 |
|------|------|
| 项目列表 | **SQLite `projects` 表**（主）+ `localStorage` 缓存 |
| 阶段 / MVP 进度 | `localStorage`（`OpenSunstar-project-stages` / `-progress`） |
| AI 洞察缓存 | SQLite `ai_insights` |

首次启动会将历史 `localStorage` 项目一次性迁移到 SQLite（`OpenSunstar-projects-db-sync-v1`）。

## 主要文件

```
src/components/kanban/KanbanPage.tsx      # 编排页（~560 行，逻辑在 hooks）
src/components/kanban/ProjectDetailSheet.tsx
src/components/kanban/SummaryCard.tsx
src/hooks/kanban/useProjectMetricsScan.ts # Git/tokei 扫描
src/hooks/kanban/usePortfolioDerivedMetrics.ts
src/hooks/kanban/usePortfolioAIAnalysis.ts
src/components/kanban/AIPortfolioMatrix.tsx
src/hooks/useProjects.ts                  # SQLite + 本地双写
src/lib/portfolioMetrics.ts               # 7 天窗口常量
src-tauri/src/project_metrics.rs          # tokei + git
src-tauri/src/ai/prompts.rs               # 周报 Prompt
tests/hooks/useProjectMetricsScan.test.ts
src/components/kanban/KanbanPage.test.tsx # 空态集成测试
```

## 手动验收

1. 添加 2+ 项目 → 刷新指标 → 「近 7 天提交」与矩阵 X 轴一致  
2. 多项目同坐标 → 矩阵点错开 + 底部提示  
3. 移除项目 → ConfirmDialog → 阶段/进度 localStorage 清除  
4. 生成周报 → 文案引用近 7 天总提交

## 后续

- 阶段/进度迁入 SQLite metadata 列  
- Vitest + MSW 覆盖 KanbanPage 空态  
- 矩阵虚拟化（20+ 项目）
