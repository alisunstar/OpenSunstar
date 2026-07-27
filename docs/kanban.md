# 工作区（Portfolio）模块说明

## 定位

**工作区**是多本地 Git 仓库的组合仪表盘，不是拖拽式任务看板。侧栏入口为「工作区」，包含今日工作台、项目看板、AI 资产总览三个 Tab。

三个 Tab 各回答一个问题，不重复挂载同一个块：

| Tab | 回答的问题 | 主要内容 |
|-----|-----------|---------|
| 今日工作台 | 今天该动哪个项目 | 待办队列、成本条、AI 周报、自然语言查询 |
| 项目看板 | 这些项目彼此是什么关系 | 项目卡片网格、组合矩阵 |
| AI 资产总览 | 配置在各项目里落地了多少 | 项目 × 资产矩阵、配置生效率 |

成本条（`AICostStrip`）归「今日工作台」：它答的是「本期烧了多少、还剩多少预算」，属于每天开机第一眼要扫的数。它和 `AINLQueryBar` 曾在两个 Tab 下各挂一份，同一份数据每切一次 Tab 拉两趟，现在全应用各只此一处。

「AI 资产总览」与侧栏「Agent 配置」是同一批实体（8 类）的两个作用域，不是「可写 vs 只读」：Agent 配置管全局资产本身，AI 资产总览看这些资产在各项目里的关联与生效情况。这层区别名字本身没表达，只写在这里和界面 subtitle 上——属于已知欠账（对抗式审查 §2.5 曾建议改名「资产库 / 项目落地」，产品决定保留旧名）。

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

## 项目组合矩阵的 Y 轴

矩阵不调用任何模型，**不需要 API Key**。Y 轴画哪个分数由数据决定，整张图只用一种，绝不混画：

| 条件 | Y 轴 | 象限措辞 |
|------|------|---------|
| 已配 AI 且已算出健康分 | AI 健康评分 | 明星项目 / 需关注 / 稳定维护 / 可能废弃 |
| 其余情况 | Agent 配置就绪分 | 活跃·配置齐全 / 活跃·配置待补 / 低频·配置齐全 / 低频·配置待补 |

两套措辞不能互换：健康分评价的是工程质量，就绪分衡量的是「AI 配置落地了多少」——一个刚加进来还没接 Claude 的项目就绪分接近 0，但它不是「可能废弃」。

**拿不到分数的项目不上图**（未纳管、未扫描、AI 尚未算出结果），不再按代码行数/活跃度编一个兜底值填进 Y 轴。

实现：`src/hooks/kanban/usePortfolioDerivedMetrics.ts` → `portfolioMatrix`；组件 `src/components/kanban/PortfolioMatrix.tsx`

## AI 资产矩阵的规模（为什么暂不虚拟化）

`ProjectAssetsMatrix` 一次渲染全部项目，不做窗口化。这是量过之后的决定，不是遗漏。

实测（jsdom，每行结构恒定：1 个行头 + 阶段 + 9 个资产格 + 分数）：

| 项目数 | DOM 节点 | 节点/行 | button | svg |
|-------|---------|--------|--------|-----|
| 25 | 1,386 | 55 | 254 | 250 |
| 50 | 2,729 | 55 | 504 | 500 |
| 100 | 5,411 | 54 | 1,004 | 1,000 |
| 200 | 10,779 | 54 | 2,004 | 2,000 |
| 500 | 26,879 | 54 | 5,004 | 5,000 |

每行 **54 个节点**是常数，浏览器的舒适区大致到 1 万节点 —— 也就是 **200 个项目左右才到拐点**。本地开发工作区的常见规模是 5–50 个，此时不到 3k 节点。

虚拟化的代价是实打实的，且和刚补上的无障碍语义直接冲突：

- `Ctrl+F` 搜不到未渲染的行；
- 表格语义（`<th scope="row">` 行头、读屏器的表格导航）在窗口化 `tbody` 上必须手工维护 `aria-rowcount` / `aria-rowindex` 才不塌；
- 首列 `position: sticky` 叠加窗口化，在各浏览器上表现不一致。

守门在 `ProjectAssetsMatrix.test.tsx` 的「矩阵规模」一节：断言渲染行数恒等于项目数，防止将来有人用 `slice()` 悄悄截断当成「优化」。真出现 200+ 项目的用户反馈再回来做。

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
src/components/kanban/PortfolioMatrix.tsx
src/components/kanban/ProjectAssetsMatrix.tsx
src/components/kanban/GovernanceDashboard.tsx
src/hooks/kanban/usePortfolioDerivedMetrics.ts
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

- 矩阵虚拟化：已量过，拐点在 ~200 个项目，暂不做（见上文「AI 资产矩阵的规模」）
- KanbanPage 集成测试覆盖 AI 面板空态
- `AssetDetailPanel` 与 `ProjectDetailSheet` 两处右侧滑出面板各自手写了焦点环，将来一起换成 Radix Dialog
