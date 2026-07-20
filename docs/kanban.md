# 工作区（Portfolio）模块说明

## 定位

**工作区**是多本地 Git 仓库的组合仪表盘，不是拖拽式任务看板。侧栏入口为「工作区」，包含今日工作台、项目看板、AI 资产总览三个 Tab。

## 统一指标窗口（7 天）

以下能力共用 **近 7 天 Git 提交数**（`git_commit_count_last_n_days(..., 7)`）：

| 能力 | 说明 |
|------|------|
| 总览卡片「近 7 天提交」 | 各项目 7 天提交求和 |
| 平均活跃度 | 基于 7 天提交分布 |
| 项目组合矩阵 X 轴 | 各项目 7 天提交数 |
| AI 生成周报 | Prompt 使用 `commit_count_7d` + `weekly_commits` 末项 |

健康评分规则仍参考 **30 天**提交（`commit_count_30d`），与更长窗口的趋势判断互补。

常量：`src/lib/portfolioMetrics.ts` → `PORTFOLIO_COMMIT_WINDOW_DAYS = 7`

## 数据持久化

| 数据 | 存储 |
|------|------|
| 项目列表 | **SQLite `projects` 表**（`localStorage` 仅作 UI 缓存） |
| 阶段（mvp / rapid / stable） | **SQLite `projects.stage`** |
| MVP 进度（0–100） | **SQLite `projects.mvp_progress`**（NULL = 未设置） |
| AI 洞察缓存 | SQLite `ai_insights` |

首次启动会将历史 `localStorage` 项目一次性迁移到 SQLite（`OpenSunstar-projects-db-sync-v1`）；阶段/进度通过 `OpenSunstar-board-metadata-db-v1` 迁入 SQLite。

## 主要文件

```
src/components/kanban/KanbanPage.tsx
src/components/kanban/ProjectDetailSheet.tsx
src/components/kanban/TodayWorkspace.tsx
src/hooks/kanban/useProjectMetricsScan.ts
src/hooks/useProjectStages.ts
src/hooks/useProjectProgress.ts
src/hooks/useProjects.ts
src/lib/migrateProjectBoardMetadata.ts
src/lib/portfolioMetrics.ts
src-tauri/src/database/dao/projects.rs
src-tauri/src/project_metrics.rs
src-tauri/src/ai/prompts.rs
```

## 手动验收

1. 添加 2+ 项目 → 刷新指标 →「近 7 天提交」与矩阵 X 轴一致
2. 修改项目阶段 / MVP 进度 → 重启应用 → 数据仍在 SQLite 中
3. 多项目同坐标 → 矩阵点错开 + 底部提示
4. 移除项目 → ConfirmDialog → 项目行从 `projects` 表删除
5. 生成周报 → 文案引用近 7 天总提交

## 后续

- 矩阵虚拟化（20+ 项目）
- KanbanPage 集成测试覆盖 AI 面板空态
