import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Clock, Loader2, Wrench } from "lucide-react";

import { Button } from "@/components/ui/button";
import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import { cn } from "@/lib/utils";
import {
  RepairDriftConfirmDialog,
} from "./RepairDriftConfirmDialog";

export interface PortfolioDriftSummaryProps {
  projects: Project[];
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  targetApp?: string;
  lastUpdatedAt?: number | null;
  className?: string;
  onOpenProject?: (project: Project) => void;
  onRepairProject?: (project: Project) => void;
  repairingProjectId?: string | null;
}

export function PortfolioDriftSummary({
  projects,
  agentReadinessMap,
  targetApp = "claude",
  lastUpdatedAt,
  className,
  onOpenProject,
  onRepairProject,
  repairingProjectId = null,
}: PortfolioDriftSummaryProps) {
  const { t } = useTranslation();
  const [pendingRepair, setPendingRepair] = useState<{
    project: Project;
    driftCount: number;
  } | null>(null);

  const { driftProjects, totalDriftItems, latestScan } = useMemo(() => {
    const driftProjects: Array<{ project: Project; entry: AgentReadinessBatchEntry }> =
      [];
    let totalDriftItems = 0;
    let latestScan: number | null = null;

    for (const project of projects) {
      const entry = agentReadinessMap.get(project.id);
      if (!entry || entry.driftCount <= 0) continue;
      driftProjects.push({ project, entry });
      totalDriftItems += entry.driftCount;
      if (entry.scannedAt != null) {
        latestScan =
          latestScan == null ? entry.scannedAt : Math.max(latestScan, entry.scannedAt);
      }
    }

    driftProjects.sort((a, b) => b.entry.driftCount - a.entry.driftCount);
    return { driftProjects, totalDriftItems, latestScan };
  }, [projects, agentReadinessMap]);

  const scanLabel = useMemo(() => {
    const ts = lastUpdatedAt ?? latestScan;
    if (ts == null) return null;
    return new Date(ts * (ts > 1e12 ? 1 : 1000)).toLocaleString();
  }, [lastUpdatedAt, latestScan]);

  if (driftProjects.length === 0) {
    return (
      <div
        className={cn(
          "rounded-xl border border-emerald-500/25 bg-emerald-500/5 px-4 py-3 text-sm text-emerald-800 dark:text-emerald-300",
          className,
        )}
      >
        {t("kanban.portfolioDrift.allClear", {
          defaultValue: "组合层生效态巡检：未发现配置漂移。",
        })}
        {scanLabel && (
          <span className="ml-2 text-xs text-muted-foreground inline-flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {scanLabel}
          </span>
        )}
      </div>
    );
  }

  return (
    <div
      className={cn(
        "rounded-xl border border-amber-500/30 bg-amber-500/5 px-4 py-3",
        className,
      )}
    >
      <div className="flex flex-wrap items-start justify-between gap-2 mb-2">
        <div className="flex items-center gap-2 text-sm font-semibold text-amber-900 dark:text-amber-200">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          {t("kanban.portfolioDrift.title", {
            projects: driftProjects.length,
            items: totalDriftItems,
            defaultValue: `${driftProjects.length} 个项目存在配置漂移（共 ${totalDriftItems} 项）`,
          })}
        </div>
        <div className="text-[11px] text-muted-foreground">
          {t("kanban.portfolioDrift.forApp", {
            app: targetApp,
            defaultValue: `目标 CLI：${targetApp}`,
          })}
          {scanLabel && (
            <span className="ml-2 inline-flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {scanLabel}
            </span>
          )}
        </div>
      </div>
      <ul className="space-y-1.5 max-h-40 overflow-y-auto">
        {driftProjects.slice(0, 8).map(({ project, entry }) => (
          <li key={project.id}>
            <div className="flex items-center gap-1">
              <button
                type="button"
                className="flex-1 min-w-0 text-left text-xs rounded-md px-2 py-1 hover:bg-amber-500/10 transition-colors flex items-center justify-between gap-2"
                onClick={() => onOpenProject?.(project)}
              >
                <span className="truncate font-medium text-foreground/90">
                  {project.name}
                </span>
                <span className="shrink-0 tabular-nums text-amber-700 dark:text-amber-400">
                  {t("kanban.portfolioDrift.itemCount", {
                    count: entry.driftCount,
                    defaultValue: `${entry.driftCount} 项漂移`,
                  })}
                </span>
              </button>
              {onRepairProject && (
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="h-7 w-7 shrink-0"
                  title={t("kanban.portfolioDrift.repairAll", {
                    defaultValue: "修复全部漂移",
                  })}
                  disabled={repairingProjectId === project.id}
                  onClick={() =>
                    setPendingRepair({
                      project,
                      driftCount: entry.driftCount,
                    })
                  }
                >
                  {repairingProjectId === project.id ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Wrench className="h-3.5 w-3.5" />
                  )}
                </Button>
              )}
            </div>
          </li>
        ))}
      </ul>
      {onRepairProject && (
        <RepairDriftConfirmDialog
          pending={
            pendingRepair
              ? {
                  kind: "project",
                  projectName: pendingRepair.project.name,
                  driftCount: pendingRepair.driftCount,
                  targetApp,
                }
              : null
          }
          zIndex="alert"
          onConfirm={() => {
            if (pendingRepair) onRepairProject(pendingRepair.project);
            setPendingRepair(null);
          }}
          onCancel={() => setPendingRepair(null)}
        />
      )}
    </div>
  );
}
