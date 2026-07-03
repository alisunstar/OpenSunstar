import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Activity, CheckCircle2, LayoutDashboard, ShieldAlert } from "lucide-react";

import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import { aggregateGovernanceStats } from "@/lib/governanceStats";
import { cn } from "@/lib/utils";

export interface GovernanceDashboardProps {
  projects: Project[];
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  targetApp?: string;
  loading?: boolean;
  className?: string;
}

function StatCard({
  label,
  value,
  sub,
  tone = "default",
}: {
  label: string;
  value: string | number;
  sub?: string;
  tone?: "default" | "success" | "warning";
}) {
  const toneClass =
    tone === "success"
      ? "border-emerald-500/25 bg-emerald-500/5"
      : tone === "warning"
        ? "border-amber-500/30 bg-amber-500/5"
        : "border-border/60 bg-card/40";

  return (
    <div className={cn("rounded-xl border px-4 py-3", toneClass)}>
      <p className="text-[11px] text-muted-foreground">{label}</p>
      <p className="text-xl font-bold tabular-nums mt-0.5">{value}</p>
      {sub && <p className="text-[10px] text-muted-foreground/80 mt-0.5">{sub}</p>}
    </div>
  );
}

export function GovernanceDashboard({
  projects,
  agentReadinessMap,
  targetApp = "claude",
  loading,
  className,
}: GovernanceDashboardProps) {
  const { t } = useTranslation();
  const stats = useMemo(
    () => aggregateGovernanceStats(projects, agentReadinessMap),
    [projects, agentReadinessMap],
  );

  const effectiveRate =
    stats.comparableItems > 0
      ? Math.round((stats.effectiveItems / stats.comparableItems) * 100)
      : null;

  return (
    <section
      className={cn("rounded-xl border border-border/60 bg-card/30 p-4", className)}
      aria-labelledby="governance-dashboard-title"
    >
      <div className="flex flex-wrap items-start justify-between gap-2 mb-3">
        <div className="flex items-center gap-2">
          <LayoutDashboard className="h-4 w-4 text-primary shrink-0" />
          <h2
            id="governance-dashboard-title"
            className="text-sm font-semibold text-foreground"
          >
            {t("kanban.governance.title", { defaultValue: "治理总览" })}
          </h2>
        </div>
        <span className="text-[11px] text-muted-foreground">
          {t("kanban.governance.forApp", {
            app: targetApp,
            defaultValue: `目标 CLI：${targetApp}`,
          })}
          {loading && (
            <span className="ml-2 text-muted-foreground/70">
              {t("common.loading", { defaultValue: "加载中…" })}
            </span>
          )}
        </span>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
        <StatCard
          label={t("kanban.governance.scannedProjects", {
            defaultValue: "已扫描项目",
          })}
          value={stats.scannedProjects}
          sub={t("kanban.governance.ofTotal", {
            total: stats.totalProjects,
            defaultValue: `共 ${stats.totalProjects} 个`,
          })}
        />
        <StatCard
          label={t("kanban.governance.driftProjects", {
            defaultValue: "漂移项目",
          })}
          value={stats.driftProjects}
          sub={t("kanban.governance.driftItems", {
            count: stats.totalDriftItems,
            defaultValue: `${stats.totalDriftItems} 项待修复`,
          })}
          tone={stats.driftProjects > 0 ? "warning" : "success"}
        />
        <StatCard
          label={t("kanban.governance.effectiveRate", {
            defaultValue: "生效率",
          })}
          value={effectiveRate != null ? `${effectiveRate}%` : "—"}
          sub={t("kanban.governance.comparableItems", {
            count: stats.comparableItems,
            defaultValue: `${stats.comparableItems} 项可比对`,
          })}
          tone={
            effectiveRate != null && effectiveRate >= 90
              ? "success"
              : effectiveRate != null && effectiveRate < 70
                ? "warning"
                : "default"
          }
        />
        <StatCard
          label={t("kanban.governance.effectiveItems", {
            defaultValue: "已生效项",
          })}
          value={stats.effectiveItems}
          sub={t("kanban.governance.portfolioScope", {
            defaultValue: "组合层汇总",
          })}
          tone="success"
        />
      </div>

      {stats.driftByCheck.length > 0 ? (
        <div>
          <div className="flex items-center gap-1.5 text-xs font-medium text-amber-800 dark:text-amber-200 mb-2">
            <ShieldAlert className="h-3.5 w-3.5" />
            {t("kanban.governance.driftBreakdown", {
              defaultValue: "漂移分布（按资产类型）",
            })}
          </div>
          <ul className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            {stats.driftByCheck.map((row) => (
              <li
                key={row.checkName}
                className="rounded-lg border border-amber-500/20 bg-amber-500/5 px-2.5 py-1.5 text-xs flex items-center justify-between gap-2"
              >
                <span className="truncate text-foreground/90">{row.label}</span>
                <span className="shrink-0 tabular-nums font-semibold text-amber-700 dark:text-amber-400">
                  {row.count}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <div className="flex items-center gap-2 text-xs text-emerald-700 dark:text-emerald-300">
          <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
          {t("kanban.governance.allEffective", {
            defaultValue: "可比对资产均已生效，组合层治理状态良好。",
          })}
        </div>
      )}

      {stats.scannedProjects === 0 && projects.length > 0 && !loading && (
        <p className="mt-3 text-[11px] text-muted-foreground flex items-center gap-1.5">
          <Activity className="h-3.5 w-3.5" />
          {t("kanban.governance.awaitingScan", {
            defaultValue: "就绪度扫描完成后将显示治理指标。",
          })}
        </p>
      )}
    </section>
  );
}
