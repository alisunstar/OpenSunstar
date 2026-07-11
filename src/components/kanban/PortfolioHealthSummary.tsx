import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Shield } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";
import {
  READINESS_OK_THRESHOLD,
  READINESS_WARN_THRESHOLD,
} from "@/lib/readinessConstants";
import { cn } from "@/lib/utils";

interface PortfolioHealthSummaryProps {
  projects: Project[];
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  assetMap: Map<string, ProjectAssetCounts>;
  loading?: boolean;
  onOpenProject: (project: Project, options?: { assetsTab?: boolean }) => void;
}

interface HealthEntry {
  project: Project;
  level: "ok" | "warn" | "alert" | "unscanned";
  reasons: string[];
  score: number | null;
}

function classifyProject(
  project: Project,
  readiness: AgentReadinessBatchEntry | undefined,
  assets: ProjectAssetCounts | undefined,
  t: (key: string, opts?: Record<string, unknown>) => string,
): HealthEntry {
  const score = readiness?.score ?? null;
  const driftCount = readiness?.driftCount ?? 0;
  const reasons: string[] = [];

  if (!readiness) {
    return {
      project,
      level: "unscanned",
      reasons: [
        t("health.reason.unscanned", {
          defaultValue: "尚未完成 AI 配置状态扫描",
        }),
      ],
      score,
    };
  }

  // Check for inconsistent live config items (RED)
  if (driftCount > 0) {
    reasons.push(
      t("health.reason.drifted", {
        count: driftCount,
        defaultValue: `${driftCount} 处配置与预期不一致`,
      }),
    );
  }

  // Check for detected_only items (YELLOW)
  if (readiness?.details) {
    const detectedOnly = readiness.details.filter(
      (d) => d.status === "detected_only",
    );
    if (detectedOnly.length > 0) {
      reasons.push(
        t("health.reason.detectedOnly", {
          items: detectedOnly.map((d) => d.label).join("、"),
          defaultValue: `${detectedOnly.map((d) => d.label).join("、")} — 发现配置文件但未纳入管理`,
        }),
      );
    }

    const globalOnly = readiness.details.filter(
      (d) => d.status === "global_only",
    );
    if (globalOnly.length > 0) {
      reasons.push(
        t("health.reason.globalOnly", {
          items: globalOnly.map((d) => d.label).join("、"),
          defaultValue: `${globalOnly.map((d) => d.label).join("、")} — 使用全局默认配置`,
        }),
      );
    }
  }

  // Check missing key assets
  if (assets) {
    if (assets.mcp === 0) {
      reasons.push(
        t("health.reason.noMcp", {
          defaultValue: "未关联 MCP 服务器",
        }),
      );
    }
    if (assets.skills === 0) {
      reasons.push(
        t("health.reason.noSkills", {
          defaultValue: "未配置 Skills",
        }),
      );
    }
    if (assets.prompts === 0) {
      reasons.push(
        t("health.reason.noPrompts", {
          defaultValue: "未关联 Prompts",
        }),
      );
    }
  }

  // Classify level
  let level: HealthEntry["level"] = "ok";
  if (
    (score !== null && score < READINESS_WARN_THRESHOLD) ||
    driftCount > 0
  ) {
    level = "alert";
  } else if (
    (score !== null && score < READINESS_OK_THRESHOLD) ||
    reasons.length > 0
  ) {
    level = "warn";
  }

  // Low score reason
  if (score !== null && score < READINESS_WARN_THRESHOLD) {
    reasons.unshift(
      t("health.reason.lowScore", {
        score,
        defaultValue: `配置完整度低（${score} 分）`,
      }),
    );
  }

  return { project, level, reasons, score };
}

export function PortfolioHealthSummary({
  projects,
  agentReadinessMap,
  assetMap,
  loading,
  onOpenProject,
}: PortfolioHealthSummaryProps) {
  const { t } = useTranslation();

  const entries = useMemo(() => {
    return projects.map((p) =>
      classifyProject(p, agentReadinessMap.get(p.id), assetMap.get(p.id), t),
    );
  }, [projects, agentReadinessMap, assetMap, t]);

  const okCount = entries.filter((e) => e.level === "ok").length;
  const warnCount = entries.filter((e) => e.level === "warn").length;
  const alertCount = entries.filter((e) => e.level === "alert").length;
  const unscannedCount = entries.filter((e) => e.level === "unscanned").length;

  // Show projects needing action first (alert → warn → unscanned), limited to 6
  const problemEntries = entries
    .filter((e) => e.level !== "ok")
    .sort((a, b) => {
      if (a.level === "alert" && b.level !== "alert") return -1;
      if (b.level === "alert" && a.level !== "alert") return 1;
      if (a.level === "warn" && b.level === "unscanned") return -1;
      if (b.level === "warn" && a.level === "unscanned") return 1;
      return (a.score ?? 999) - (b.score ?? 999);
    })
    .slice(0, 6);

  return (
    <div className="space-y-3">
      {/* Traffic light status bar */}
      <div className="flex flex-wrap items-center gap-4 rounded-xl border border-border/60 bg-card/30 px-4 py-3">
        <span className="text-xs font-medium text-muted-foreground">
          {t("health.title", { defaultValue: "配置状态" })}
          {loading && (
            <span className="ml-1 text-muted-foreground/60">
              {t("common.loading", { defaultValue: "检查中…" })}
            </span>
          )}
        </span>
        <div className="flex flex-wrap items-center gap-4">
          <span className="inline-flex items-center gap-1.5 text-xs font-semibold tabular-nums">
            <span className="w-2.5 h-2.5 rounded-full bg-emerald-500" />
            <span className="text-emerald-600 dark:text-emerald-400">
              {okCount}
            </span>
            <span className="text-muted-foreground font-normal">
              {t("health.level.ok", { defaultValue: "正常" })}
            </span>
          </span>
          {warnCount > 0 && (
            <span className="inline-flex items-center gap-1.5 text-xs font-semibold tabular-nums">
              <span className="w-2.5 h-2.5 rounded-full bg-amber-500" />
              <span className="text-amber-600 dark:text-amber-400">
                {warnCount}
              </span>
              <span className="text-muted-foreground font-normal">
                {t("health.level.warn", { defaultValue: "需关注" })}
              </span>
            </span>
          )}
          {alertCount > 0 && (
            <span className="inline-flex items-center gap-1.5 text-xs font-semibold tabular-nums">
              <span className="w-2.5 h-2.5 rounded-full bg-red-500" />
              <span className="text-red-600 dark:text-red-400">
                {alertCount}
              </span>
              <span className="text-muted-foreground font-normal">
                {t("health.level.alert", { defaultValue: "异常" })}
              </span>
            </span>
          )}
          {unscannedCount > 0 && (
            <span className="inline-flex items-center gap-1.5 text-xs font-semibold tabular-nums">
              <span className="w-2.5 h-2.5 rounded-full bg-slate-500" />
              <span className="text-slate-600 dark:text-slate-300">
                {unscannedCount}
              </span>
              <span className="text-muted-foreground font-normal">
                {t("health.level.unscanned", { defaultValue: "未扫描" })}
              </span>
            </span>
          )}
        </div>
      </div>

      {/* Problem projects list */}
      {problemEntries.length > 0 && (
        <div className="rounded-xl border border-border/50 bg-card/20 divide-y divide-border/30">
          {problemEntries.map(({ project, level, reasons, score }) => (
            <div
              key={project.id}
              className="group flex flex-wrap items-center gap-2 px-4 py-2.5 hover:bg-muted/20 transition-colors"
            >
              <span
                className={cn(
                  "w-2 h-2 rounded-full shrink-0",
                  level === "alert"
                    ? "bg-red-500"
                    : level === "warn"
                      ? "bg-amber-500"
                      : "bg-slate-500",
                )}
              />
              <div className="flex-1 min-w-[160px]">
                <p className="text-sm font-medium text-foreground truncate">
                  {project.name}
                </p>
                <p className="text-[11px] text-muted-foreground mt-0.5 leading-relaxed">
                  {reasons.join(" · ")}
                </p>
              </div>
              {score !== null && (
                <span
                  className={cn(
                    "inline-flex items-center gap-0.5 text-[10px] font-semibold tabular-nums shrink-0",
                    score >= READINESS_OK_THRESHOLD
                      ? "text-emerald-500"
                      : score >= READINESS_WARN_THRESHOLD
                        ? "text-amber-500"
                        : "text-red-500",
                  )}
                >
                  <Shield className="h-3 w-3" />
                  {score}
                </span>
              )}
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs shrink-0 opacity-80 group-hover:opacity-100"
                onClick={() => onOpenProject(project, { assetsTab: true })}
              >
                {level === "alert"
                  ? t("health.action.repair", { defaultValue: "查看修复" })
                  : level === "unscanned"
                    ? t("health.action.inspect", { defaultValue: "查看项目" })
                    : t("health.action.configure", {
                        defaultValue: "配置资产",
                      })}
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* All clear message */}
      {problemEntries.length === 0 && !loading && (
        <div className="flex items-center gap-2 rounded-xl border border-emerald-500/25 bg-emerald-500/5 px-4 py-3 text-sm text-emerald-700 dark:text-emerald-300">
          <CheckCircle2 className="h-4 w-4 shrink-0" />
          {t("health.allClear", {
            defaultValue: "所有项目 AI 配置状态正常。",
          })}
        </div>
      )}
    </div>
  );
}
