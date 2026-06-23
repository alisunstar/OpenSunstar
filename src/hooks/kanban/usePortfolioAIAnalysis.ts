import { useState, useEffect } from "react";
import type { Project } from "@/types/project";
import type { StageKey } from "@/hooks/useProjectStages";
import type { AIProviderConfig } from "@/api/aiInsight";
import {
  buildProjectContext,
  getAIInsight,
  getAIHealthScore,
} from "@/api/aiInsight";
import { useAICostOptional } from "@/contexts/AICostContext";import type { CodeLineResult, Contributor } from "@/api/codeMetrics";
import type { ProjectGitInfo } from "@/api/projectGit";

interface PortfolioAIAnalysisInput {
  projects: Project[];
  scanning: boolean;
  aiConfigured: boolean;
  getConfig: () => AIProviderConfig | null;
  getStage: (projectId: string) => StageKey;
  progressMap: Map<string, number>;
  codeLinesMap: Map<string, CodeLineResult>;
  gitInfoMap: Map<string, ProjectGitInfo>;
  commits7dMap: Map<string, number>;
  commits30dMap: Map<string, number>;
  contributorsMap: Map<string, Contributor[]>;
  versionMap: Map<string, string>;
  weeklyCommitsMap: Map<string, number[]>;
  scanEpoch: number;
}

export function usePortfolioAIAnalysis({
  projects,
  scanning,
  aiConfigured,
  getConfig,
  getStage,
  progressMap,
  codeLinesMap,
  gitInfoMap,
  commits7dMap,
  commits30dMap,
  contributorsMap,
  versionMap,
  weeklyCommitsMap,
  scanEpoch,
}: PortfolioAIAnalysisInput) {
  const costCtx = useAICostOptional();
  const [aiSummaryMap, setAiSummaryMap] = useState<Map<string, string>>(
    new Map(),
  );
  const [aiHealthMap, setAiHealthMap] = useState<Map<string, number>>(
    new Map(),
  );
  const [aiLoadingMap, setAiLoadingMap] = useState<Map<string, boolean>>(
    new Map(),
  );
  const [aiTrendInsightMap, setAiTrendInsightMap] = useState<
    Map<string, string>
  >(new Map());

  useEffect(() => {
    if (scanning || !aiConfigured || projects.length === 0) return;
    const config = getConfig();
    if (!config) return;

    let cancelled = false;

    const runAIAnalysis = async () => {
      for (const p of projects) {
        if (cancelled) break;

        setAiLoadingMap((m) => new Map(m).set(p.id, true));

        try {
          const stage = getStage(p.id);
          const code = codeLinesMap.get(p.id) ?? null;
          const gitInfo = gitInfoMap.get(p.id) ?? null;
          const commits7d = commits7dMap.get(p.id) ?? 0;
          const commits30d = commits30dMap.get(p.id) ?? 0;
          const weekly = weeklyCommitsMap.get(p.id) ?? [];
          const contribs = contributorsMap.get(p.id) ?? [];
          const version = versionMap.get(p.id) ?? null;
          const mvpProg =
            stage === "mvp" ? progressMap.get(p.id) ?? null : null;

          const ctx = buildProjectContext(
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
          );

          const [insight, health] = await Promise.all([
            getAIInsight(p.id, "summary", config, ctx),
            getAIHealthScore(p.id, config, ctx),
          ]);

          if (cancelled) break;

          if (insight?.content) {
            setAiSummaryMap((m) => new Map(m).set(p.id, insight.content));
            costCtx?.recordCall({
              cost: insight.cost_estimate,
              tokens: insight.tokens_used,
              insightType: "summary",
              isCached: insight.is_cached,
            });
          }
          if (typeof health?.score === "number") {
            setAiHealthMap((m) => new Map(m).set(p.id, health.score));
          }

          if (weekly.length > 0 && weekly.some((c) => c > 0)) {
            const trend = await getAIInsight(p.id, "trend_analysis", config, ctx);
            if (!cancelled && trend?.content) {
              setAiTrendInsightMap((m) => new Map(m).set(p.id, trend.content));
              costCtx?.recordCall({
                cost: trend.cost_estimate,
                tokens: trend.tokens_used,
                insightType: "trend_analysis",
                isCached: trend.is_cached,
              });
            }
          }
        } catch {
          /* 单个项目 AI 分析失败不影响其他 */
        } finally {
          if (!cancelled) {
            setAiLoadingMap((m) => new Map(m).set(p.id, false));
          }
        }
      }
    };

    void runAIAnalysis();
    return () => {
      cancelled = true;
    };
  }, [
    scanning,
    aiConfigured,
    projects,
    codeLinesMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    contributorsMap,
    versionMap,
    weeklyCommitsMap,
    scanEpoch,
    getConfig,
    getStage,
    progressMap,
    costCtx,
  ]);

  return {
    aiSummaryMap,
    aiHealthMap,
    aiLoadingMap,
    aiTrendInsightMap,
  };
}
