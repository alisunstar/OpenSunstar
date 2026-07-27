import { useMemo } from "react";
import type { TFunction } from "i18next";
import type { Project } from "@/types/project";
import type { StageKey } from "@/hooks/useProjectStages";
import { buildProjectContext, type ProjectContextInput } from "@/api/aiInsight";
import type { CodeLineResult, Contributor } from "@/api/codeMetrics";
import type { ProjectGitInfo } from "@/api/projectGit";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import {
  classifyReadinessLevel,
  shouldShowReadinessScore,
} from "@/lib/portfolioHealth";
import type { ProjectScoreKind } from "@/lib/kanban/projectScores";
import type { ProjectPoint } from "@/components/kanban/PortfolioMatrix";
import type { PortfolioOverviewWindowDays } from "@/lib/portfolioMetrics";
import {
  activityTierForWindow,
  formatCompactNumber,
} from "@/lib/portfolioMetrics";

interface PortfolioDerivedInput {
  projects: Project[];
  codeLinesMap: Map<string, CodeLineResult>;
  gitInfoMap: Map<string, ProjectGitInfo>;
  commits7dMap: Map<string, number>;
  commits30dMap: Map<string, number>;
  weeklyCommitsMap: Map<string, number[]>;
  contributorsMap: Map<string, Contributor[]>;
  versionMap: Map<string, string>;
  progressMap: Map<string, number>;
  aiHealthMap: Map<string, number>;
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  aiConfigured: boolean;
  scanning: boolean;
  overviewWindowDays: PortfolioOverviewWindowDays;
  getStage: (projectId: string) => StageKey;
  t: TFunction;
}

/**
 * 可上图的就绪分，拿不到就是 `undefined`。
 *
 * 「未纳管 / 未扫描」两种情况下后端本身就返回 `score: None`
 * （`cli_api.rs:404`），把它当 0 画等于凭空断言「这个项目一项配置都没有」。
 * 判定完全复用 `portfolioHealth.ts`，不在这里另起一套口径。
 *
 * 批量接口不带 `max_score`，分值恒为 100 分制，无需归一化。
 */
function readinessScoreOf(
  entry: AgentReadinessBatchEntry | undefined,
): number | undefined {
  if (!entry) return undefined;
  return shouldShowReadinessScore(classifyReadinessLevel(entry))
    ? entry.score
    : undefined;
}

export function usePortfolioDerivedMetrics({
  projects,
  codeLinesMap,
  gitInfoMap,
  commits7dMap,
  commits30dMap,
  weeklyCommitsMap,
  contributorsMap,
  versionMap,
  progressMap,
  aiHealthMap,
  agentReadinessMap,
  aiConfigured,
  scanning,
  overviewWindowDays,
  getStage,
  t,
}: PortfolioDerivedInput) {
  const projectContextsMap = useMemo(() => {
    const m = new Map<string, ProjectContextInput>();
    for (const p of projects) {
      const stage = getStage(p.id);
      const code = codeLinesMap.get(p.id) ?? null;
      const gitInfo = gitInfoMap.get(p.id) ?? null;
      const commits7d = commits7dMap.get(p.id) ?? 0;
      const commits30d = commits30dMap.get(p.id) ?? 0;
      const weekly = weeklyCommitsMap.get(p.id) ?? [];
      const contribs = contributorsMap.get(p.id) ?? [];
      const version = versionMap.get(p.id) ?? null;
      const mvpProg = stage === "mvp" ? (progressMap.get(p.id) ?? null) : null;
      m.set(
        p.id,
        buildProjectContext(
          p,
          stage,
          code,
          gitInfo,
          commits7d,
          commits30d,
          weekly,
          contribs,
          version,
          mvpProg ?? undefined,
        ),
      );
    }
    return m;
  }, [
    projects,
    codeLinesMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    weeklyCommitsMap,
    contributorsMap,
    versionMap,
    progressMap,
    getStage,
  ]);

  /**
   * 组合矩阵的散点。**整张图只画一种分数**，由这里选定后随点一起交出去。
   *
   * 改动前这里有两个问题，且互相掩护（审查报告 §2.5 + §5.6）：
   *
   * 1. `if (!aiConfigured) return []` —— 报告只看到组件侧的 `aiConfigured &&`
   *    门控，没看到数据侧还有第二道。矩阵本身不调用任何模型（纯 recharts 画
   *    本地数字），没配 API Key 的用户被挡在一张纯本地图表外面，毫无道理。
   * 2. `fallbackHealth = codeLines > 0 ? (activity > 0 ? 52 : 42) : ...`
   *    —— AI 健康分缺失时**编一个数**填进 Y 轴。这四个值（52/42/48/35）
   *    全部低于矩阵的 60 分横切线，于是「AI 还没分析出结果」会被画成
   *    「所有项目都在下半区」，读图的人以为看到了结论，其实看到的是占位符。
   *
   * 只删第 1 条会把第 2 条的后果放大到全体无 Key 用户身上，所以两条一起改：
   * 分数不再编造，缺分的项目直接不上图（点会变少，但每个点都是真的）；
   * 没有 AI 健康分时改用就绪分 —— 后端配置扫描真实测出来的 0-100，
   * 正是 §5.2 刚统一命名的那一个。
   *
   * 两种分数**永不混画**：来源毫不相干，混在一条 Y 轴上比不画更糟。
   */
  const portfolioMatrix = useMemo<{
    points: ProjectPoint[];
    scoreKind: ProjectScoreKind;
  }>(() => {
    // 扫描期间 activity / codeLines 只填了一部分，此时出图会边看边跳。
    if (scanning || projects.length === 0)
      return { points: [], scoreKind: "agentReadiness" };

    const scoreKind: ProjectScoreKind =
      aiConfigured && aiHealthMap.size > 0 ? "aiHealth" : "agentReadiness";

    const points: ProjectPoint[] = [];
    for (const p of projects) {
      const score =
        scoreKind === "aiHealth"
          ? aiHealthMap.get(p.id)
          : readinessScoreOf(agentReadinessMap.get(p.id));
      // 没有分数就没有 Y 坐标，这个项目就不该出现在图上。
      if (score == null) continue;
      points.push({
        projectId: p.id,
        name: p.name,
        stage: getStage(p.id),
        activity: commits7dMap.get(p.id) ?? 0,
        score,
        codeLines: codeLinesMap.get(p.id)?.code_lines ?? 0,
      });
    }
    return { points, scoreKind };
  }, [
    projects,
    aiConfigured,
    scanning,
    aiHealthMap,
    agentReadinessMap,
    commits7dMap,
    codeLinesMap,
    getStage,
  ]);

  const totalCodeLines = useMemo(() => {
    let sum = 0;
    for (const [, result] of codeLinesMap) sum += result.code_lines;
    return sum;
  }, [codeLinesMap]);

  const commitsInWindowMap = useMemo(() => {
    return overviewWindowDays === 30 ? commits30dMap : commits7dMap;
  }, [overviewWindowDays, commits7dMap, commits30dMap]);

  const totalCommitsInWindow = useMemo(() => {
    let sum = 0;
    for (const count of commitsInWindowMap.values()) sum += count;
    return sum;
  }, [commitsInWindowMap]);

  const { averageActivityLabel, averageActivityColor } = useMemo(() => {
    if (commitsInWindowMap.size === 0)
      return { averageActivityLabel: "—", averageActivityColor: "" };
    let total = 0;
    for (const count of commitsInWindowMap.values()) {
      total += activityTierForWindow(count, overviewWindowDays);
    }
    const avg = total / commitsInWindowMap.size;
    if (avg >= 3.5)
      return {
        averageActivityLabel: t("kanban.activity.veryHigh", {
          defaultValue: "很高",
        }),
        averageActivityColor: "text-emerald-500",
      };
    if (avg >= 2.5)
      return {
        averageActivityLabel: t("kanban.activity.high", { defaultValue: "高" }),
        averageActivityColor: "text-emerald-400",
      };
    if (avg >= 1.5)
      return {
        averageActivityLabel: t("kanban.activity.medium", {
          defaultValue: "中等",
        }),
        averageActivityColor: "text-amber-500",
      };
    return {
      averageActivityLabel: t("kanban.activity.low", { defaultValue: "低" }),
      averageActivityColor: "text-muted-foreground",
    };
  }, [commitsInWindowMap, overviewWindowDays, t]);

  return {
    projectContextsMap,
    portfolioPoints: portfolioMatrix.points,
    /** Y 轴画的是哪个分数 —— 组件据此取轴标签与象限措辞，不自己猜。 */
    portfolioScoreKind: portfolioMatrix.scoreKind,
    totalCodeLines,
    commitsInWindowMap,
    totalCommitsInWindow,
    overviewWindowDays,
    averageActivityLabel,
    averageActivityColor,
    formatCompactNumber,
  };
}
