import type { ProjectContextInput } from "@/api/aiInsight";
import type { CodeLineResult, Contributor } from "@/api/codeMetrics";
import type { ProjectGitInfo } from "@/api/projectGit";
import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";
import type { StageKey } from "@/hooks/useProjectStages";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import type { ProjectView } from "@/types/projectView";

export type { ProjectView };

/**
 * 建 `ProjectView` 所需的全部原始表。
 *
 * 字段一个不少地列在这里，是刻意的：漏传一张表会被 TypeScript 当场拦下，
 * 而不是在某个组件里悄悄变成一片 `undefined`。
 */
export interface ProjectViewSources {
  projects: Project[];
  getStage: (projectId: string) => StageKey;
  progressMap: Map<string, number>;
  codeLinesMap: Map<string, CodeLineResult>;
  versionMap: Map<string, string>;
  gitInfoMap: Map<string, ProjectGitInfo>;
  commits7dMap: Map<string, number>;
  commits30dMap: Map<string, number>;
  contributorsMap: Map<string, Contributor[]>;
  weeklyCommitsMap: Map<string, number[]>;
  aiSummaryMap: Map<string, string>;
  aiLoadingMap: Map<string, boolean>;
  aiHealthMap: Map<string, number>;
  aiTrendInsightMap: Map<string, string>;
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  assetMap: Map<string, ProjectAssetCounts>;
  projectContextsMap: Map<string, ProjectContextInput>;
}

/**
 * 把 14 张平行表压成每个项目一行（审查报告 §6.1）。
 *
 * 唯一的规矩：**这里不给任何字段兜底**。`.get()` 返回什么就是什么，
 * 缺失一律 `undefined` 传下去。在这一层写 `?? 0` 看着无害，实际是把
 * 「还没扫到」偷换成「扫到了，是 0」—— UI 会理直气壮地画出「0 行代码 /
 * 近 7 天 0 次提交」，用户据此判断项目已死。与 §5.6「查询失败不等于 ¥0」
 * 是同一类错误，只是发生在更上游、更不容易被发现的地方。
 *
 * 两个例外，都是「缺失本身就有确定含义」：
 * - `aiSummaryLoading`：没有条目 = 没在转圈，`false` 是事实而非猜测。
 * - `aiTrendInsight` / `context`：下游按 `null` 判空，沿用原签名。
 */
export function buildProjectViews(src: ProjectViewSources): ProjectView[] {
  return src.projects.map((project) => {
    const id = project.id;
    return {
      id,
      project,
      stage: src.getStage(id),
      progress: src.progressMap.get(id),

      codeLines: src.codeLinesMap.get(id),
      version: src.versionMap.get(id),
      gitInfo: src.gitInfoMap.get(id),
      commits7d: src.commits7dMap.get(id),
      commits30d: src.commits30dMap.get(id),
      contributors: src.contributorsMap.get(id),
      weeklyCommits: src.weeklyCommitsMap.get(id),

      aiSummary: src.aiSummaryMap.get(id),
      aiSummaryLoading: src.aiLoadingMap.get(id) ?? false,
      aiHealthScore: src.aiHealthMap.get(id),
      aiTrendInsight: src.aiTrendInsightMap.get(id) ?? null,
      context: src.projectContextsMap.get(id) ?? null,

      readiness: src.agentReadinessMap.get(id),
      assets: src.assetMap.get(id),
    };
  });
}

/** 按 id 建索引，供「只要一个」的消费方（详情抽屉等）直接取。 */
export function indexProjectViews(
  views: ProjectView[],
): Map<string, ProjectView> {
  return new Map(views.map((view) => [view.id, view]));
}

/**
 * 按给定的项目顺序取回视图，索引里没有的静默跳过。
 *
 * 搜索与分组过滤后的 `Project[]` 与视图表可能短暂不同步（项目刚被移除、
 * 视图还没重建）。少画一行，好过抛异常把整块看板打空。
 */
export function pickProjectViews(
  viewMap: Map<string, ProjectView>,
  projects: Project[],
): ProjectView[] {
  const picked: ProjectView[] = [];
  for (const project of projects) {
    const view = viewMap.get(project.id);
    if (view) picked.push(view);
  }
  return picked;
}
