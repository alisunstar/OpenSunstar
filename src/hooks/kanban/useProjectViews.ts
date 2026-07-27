import { useMemo } from "react";

import {
  buildProjectViews,
  indexProjectViews,
  type ProjectViewSources,
} from "@/lib/kanban/projectView";
import type { ProjectView } from "@/types/projectView";

/**
 * 把 `KanbanPage` 里那 14 张平行表合成一张视图表（审查报告 §6.1）。
 *
 * 依赖数组把每张表都列上去，是因为上游那五个 hook 各自用 `useState` 持有
 * 自己的 Map：某张表刷新时换引用，其余保持不变，`useMemo` 才只在真变了的
 * 时候重建。反过来，**这里返回的 `views` / `viewMap` 引用必须稳定** ——
 * `KanbanPage.tsx` 有一个把 readiness 数据放进依赖数组的 effect，每次渲染
 * 换引用会直接把它变成无限循环（见 `workspaceTabLayout.test.tsx` 的注释）。
 */
export function useProjectViews(src: ProjectViewSources): {
  views: ProjectView[];
  viewMap: Map<string, ProjectView>;
} {
  const views = useMemo(
    () => buildProjectViews(src),
    [
      src.projects,
      src.getStage,
      src.progressMap,
      src.codeLinesMap,
      src.versionMap,
      src.gitInfoMap,
      src.commits7dMap,
      src.commits30dMap,
      src.contributorsMap,
      src.weeklyCommitsMap,
      src.aiSummaryMap,
      src.aiLoadingMap,
      src.aiHealthMap,
      src.aiTrendInsightMap,
      src.agentReadinessMap,
      src.assetMap,
      src.projectContextsMap,
    ],
  );

  const viewMap = useMemo(() => indexProjectViews(views), [views]);

  return { views, viewMap };
}
