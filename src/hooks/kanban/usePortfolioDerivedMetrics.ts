import { useMemo } from "react";
import type { TFunction } from "i18next";
import type { Project } from "@/types/project";
import type { StageKey } from "@/hooks/useProjectStages";
import {
  buildProjectContext,
  type ProjectContextInput,
} from "@/api/aiInsight";
import type { CodeLineResult, Contributor } from "@/api/codeMetrics";
import type { ProjectGitInfo } from "@/api/projectGit";
import type { PortfolioOverviewWindowDays } from "@/lib/portfolioMetrics";
import { activityTierForWindow, formatCompactNumber } from "@/lib/portfolioMetrics";

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
  aiConfigured: boolean;
  scanning: boolean;
  overviewWindowDays: PortfolioOverviewWindowDays;
  getStage: (projectId: string) => StageKey;
  t: TFunction;
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
      const mvpProg = stage === "mvp" ? progressMap.get(p.id) ?? null : null;
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

  const portfolioPoints = useMemo(() => {
    if (!aiConfigured || scanning || projects.length === 0) return [];
    return projects.map((p) => {
      const activity = commits7dMap.get(p.id) ?? 0;
      const codeLines = codeLinesMap.get(p.id)?.code_lines ?? 0;
      const aiHealth = aiHealthMap.get(p.id);
      const fallbackHealth =
        codeLines > 0 ? (activity > 0 ? 52 : 42) : activity > 0 ? 48 : 35;
      return {
        projectId: p.id,
        name: p.name,
        stage: getStage(p.id),
        activity,
        health: aiHealth ?? fallbackHealth,
        codeLines,
      };
    });
  }, [
    projects,
    aiConfigured,
    scanning,
    aiHealthMap,
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
        averageActivityLabel: t("kanban.activity.veryHigh", { defaultValue: "很高" }),
        averageActivityColor: "text-emerald-500",
      };
    if (avg >= 2.5)
      return {
        averageActivityLabel: t("kanban.activity.high", { defaultValue: "高" }),
        averageActivityColor: "text-emerald-400",
      };
    if (avg >= 1.5)
      return {
        averageActivityLabel: t("kanban.activity.medium", { defaultValue: "中等" }),
        averageActivityColor: "text-amber-500",
      };
    return {
      averageActivityLabel: t("kanban.activity.low", { defaultValue: "低" }),
      averageActivityColor: "text-muted-foreground",
    };
  }, [commitsInWindowMap, overviewWindowDays, t]);

  return {
    projectContextsMap,
    portfolioPoints,
    totalCodeLines,
    totalCommitsInWindow,
    overviewWindowDays,
    averageActivityLabel,
    averageActivityColor,
    formatCompactNumber,
  };
}
