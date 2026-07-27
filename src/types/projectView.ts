import type { ProjectContextInput } from "@/api/aiInsight";
import type { CodeLineResult, Contributor } from "@/api/codeMetrics";
import type { ProjectGitInfo } from "@/api/projectGit";
import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";
import type { StageKey } from "@/hooks/useProjectStages";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";

/**
 * 一个项目在「跨项目工作区」里的全部已知信息。
 *
 * 在此之前，这些字段是 `KanbanPage.tsx` 里 14 个按 `project.id` 索引的平行 Map
 * （审查报告 §6.1），分散在五个 hook 的返回值里。每个消费方各自 `.get(id)`、
 * 各自起名、各自决定缺失时怎么办 —— §5.2 那个「一个分数，三个名字」正是这么
 * 长出来的：三处 `agentReadinessMap.get(id)`，谁都不知道对方在说同一件事。
 *
 * 聚合成一个对象换来三件事：
 *
 * 1. **id 只出现一次。** 串号从「小心点」变成类型层面不可能。
 * 2. **字段名是唯一的。** `aiHealthScore` 和 `readiness.score` 摆在同一个对象里，
 *    想混也得先看见它们不一样。
 * 3. **缺失语义统一。** `undefined` 一路传到 UI，由 UI 决定画「—」还是骨架屏；
 *    聚合层不许 `?? 0`，否则「还没扫到」会被画成「0 行代码」。
 */
export interface ProjectView {
  /** = `project.id`，供 `key` / 索引直接取用 */
  id: string;
  project: Project;
  stage: StageKey;
  /** 仅 MVP 阶段有意义；未设置时为 `undefined` */
  progress: number | undefined;

  // ── 仓库扫描（useProjectMetricsScan）──────────────
  codeLines: CodeLineResult | undefined;
  version: string | undefined;
  gitInfo: ProjectGitInfo | undefined;
  commits7d: number | undefined;
  commits30d: number | undefined;
  contributors: Contributor[] | undefined;
  weeklyCommits: number[] | undefined;

  // ── AI 分析（usePortfolioAIAnalysis）──────────────
  aiSummary: string | undefined;
  aiSummaryLoading: boolean;
  /** AI 工程健康度 0-100。**不是** `readiness.score`，见 `projectScores.ts` */
  aiHealthScore: number | undefined;
  aiTrendInsight: string | null;
  /** 喂给 AI 的项目上下文；`null` 表示尚未构建 */
  context: ProjectContextInput | null;

  // ── 配置落地（useAgentReadinessBatch / usePortfolioAssetSummary）──
  /** Agent 配置就绪度。`undefined` = 尚未扫描，与「扫了但 0 分」不是一回事 */
  readiness: AgentReadinessBatchEntry | undefined;
  assets: ProjectAssetCounts | undefined;
}
