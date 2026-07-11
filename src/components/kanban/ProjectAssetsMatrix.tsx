import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import {
  AlertTriangle,
  Check,
  ChevronRight,
  Clock,
  Globe2,
  Loader2,
  Minus,
  Search,
  Shield,
  X,
} from "lucide-react";

import type { Project } from "@/types/project";
import type { StageKey } from "@/hooks/useProjectStages";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { AgentReadinessItem } from "@/api/aiInsight";
import { readinessScoreTone } from "@/lib/readinessConstants";
import { GOVERNANCE_CHECK_LABELS } from "@/lib/governanceStats";
import { cn } from "@/lib/utils";
import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";

// ── asset column definitions ──────────────────────────

type CellState =
  | "normal"
  | "attention"
  | "abnormal"
  | "unscanned"
  | "not_applicable";

type MatrixFilter = "all" | "needs_action" | "abnormal" | "unscanned";

type CellStatusKind =
  | "effective"
  | "mismatch"
  | "missing"
  | "global"
  | "detected"
  | "partial"
  | "unscanned"
  | "not_applicable"
  | "attention";

interface AssetColumn {
  checkName: string;
  label: string;
  safetyCritical: boolean;
  width: string;
}

const ASSET_COLUMNS: AssetColumn[] = [
  { checkName: "mcp_enabled", label: "MCP", safetyCritical: false, width: "w-[72px]" },
  { checkName: "skills_configured", label: "Skills", safetyCritical: false, width: "w-[72px]" },
  { checkName: "prompt_files", label: "Prompts", safetyCritical: false, width: "w-[72px]" },
  { checkName: "commands_configured", label: "Cmds", safetyCritical: false, width: "w-[64px]" },
  { checkName: "hooks_configured", label: "Hooks", safetyCritical: true, width: "w-[64px]" },
  { checkName: "ignore_rules", label: "Ignore", safetyCritical: true, width: "w-[68px]" },
  { checkName: "permissions", label: "Perms", safetyCritical: true, width: "w-[64px]" },
  { checkName: "subagents_configured", label: "Subs", safetyCritical: false, width: "w-[64px]" },
];

// ── cell health determination ──────────────────────────

function getCellState(
  item: AgentReadinessItem | undefined,
  safetyCritical: boolean,
): CellState {
  if (!item) return "unscanned";

  // effective_state is the most authoritative signal
  if (item.effective_state) {
    switch (item.effective_state) {
      case "effective":
        return "normal";
      case "drifted":
        return "abnormal";
      case "not_applicable":
        if (item.configured_state === "unconfigured") break;
        return "not_applicable";
      case "unchecked":
        return "unscanned";
    }
  }

  // fall back to readiness status
  switch (item.status) {
    case "ready":
      return "normal";
    case "partial":
    case "global_only":
    case "detected_only":
      return "attention";
    case "not_applicable":
      return "not_applicable";
    case "missing":
      return safetyCritical ? "abnormal" : "attention";
  }

  // last resort: score
  return item.score > 0
    ? "normal"
    : safetyCritical
      ? "abnormal"
      : "attention";
}

// ── cell display helpers ──────────────────────────────

function getCellStatusKind(
  item: AgentReadinessItem | undefined,
  state: CellState,
): CellStatusKind {
  if (!item) return "unscanned";

  switch (item.effective_state) {
    case "effective":
      return "effective";
    case "drifted":
      return "mismatch";
    case "unchecked":
      return "unscanned";
    case "not_applicable":
      if (item.configured_state !== "unconfigured") {
        return "not_applicable";
      }
      break;
  }

  switch (item.status) {
    case "ready":
      return "effective";
    case "missing":
      return "missing";
    case "global_only":
      return "global";
    case "detected_only":
      return "detected";
    case "partial":
      return "partial";
    case "not_applicable":
      return "not_applicable";
  }

  switch (state) {
    case "normal":
      return "effective";
    case "attention":
      return "attention";
    case "abnormal":
      return "mismatch";
    case "unscanned":
      return "unscanned";
    case "not_applicable":
      return "not_applicable";
  }
}

function cellStatusLabel(kind: CellStatusKind, t: TFunction): string {
  switch (kind) {
    case "effective":
      return t("assetsMatrix.cellEffective", { defaultValue: "已生效" });
    case "mismatch":
      return t("assetsMatrix.cellMismatch", { defaultValue: "不一致" });
    case "missing":
      return t("assetsMatrix.cellMissing", { defaultValue: "缺失" });
    case "global":
      return t("assetsMatrix.cellGlobal", { defaultValue: "全局" });
    case "detected":
      return t("assetsMatrix.cellDetected", { defaultValue: "探测" });
    case "partial":
      return t("assetsMatrix.cellPartial", { defaultValue: "部分" });
    case "unscanned":
      return t("assetsMatrix.cellUnscanned", { defaultValue: "未扫" });
    case "not_applicable":
      return t("assetsMatrix.cellNA", { defaultValue: "不适用" });
    case "attention":
      return t("assetsMatrix.cellWarn", { defaultValue: "需关注" });
  }
}

function cellClasses(state: CellState): string {
  switch (state) {
    case "normal":
      return "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400";
    case "attention":
      return "bg-amber-500/10 text-amber-600 dark:text-amber-400";
    case "abnormal":
      return "bg-red-500/10 text-red-600 dark:text-red-400";
    case "unscanned":
      return "bg-slate-500/10 text-slate-600 dark:text-slate-300";
    case "not_applicable":
      return "bg-muted/40 text-muted-foreground";
  }
}

function CellStatusIcon({ kind }: { kind: CellStatusKind }) {
  switch (kind) {
    case "effective":
      return <Check className="h-3 w-3 shrink-0" />;
    case "mismatch":
      return <X className="h-3 w-3 shrink-0" />;
    case "missing":
      return <Minus className="h-3 w-3 shrink-0" />;
    case "global":
      return <Globe2 className="h-3 w-3 shrink-0" />;
    case "detected":
      return <Search className="h-3 w-3 shrink-0" />;
    case "partial":
    case "attention":
      return <AlertTriangle className="h-3 w-3 shrink-0" />;
    case "unscanned":
      return <Clock className="h-3 w-3 shrink-0" />;
    case "not_applicable":
      return <Minus className="h-3 w-3 shrink-0" />;
  }
}

// ── detail label for slide-over ───────────────────────

function cellDetailLabel(
  item: AgentReadinessItem | undefined,
  safetyCritical: boolean,
  t: TFunction,
): string {
  if (!item)
    return t("assetsMatrix.scanPending", {
      defaultValue: "尚未完成扫描，当前状态不能判定为正常。",
    });

  if (item.effective_state === "effective")
    return t("assetsMatrix.detailEffective", {
      defaultValue: "配置已生效，与 OpenSunstar 库一致",
    });
  if (item.effective_state === "drifted")
    return t("assetsMatrix.detailDrifted", {
      defaultValue: "配置与预期不一致，可能需要修复",
    });
  if (item.effective_state === "unchecked")
    return (
      item.effective_detail ||
      t("assetsMatrix.detailUnscanned", {
        defaultValue: "已发现配置，但暂未完成目标 CLI 生效状态比对",
      })
    );
  if (
    item.effective_state === "not_applicable" &&
    item.configured_state !== "unconfigured"
  )
    return (
      item.effective_detail ||
      t("assetsMatrix.detailNA", {
        defaultValue: "当前目标 CLI 不支持此项",
      })
    );
  if (item.status === "global_only")
    return t("assetsMatrix.detailGlobal", {
      defaultValue: "使用全局默认配置，项目级未自定义",
    });
  if (item.status === "detected_only")
    return t("assetsMatrix.detailDetected", {
      defaultValue: "检测到仓库中有配置，但未被 OpenSunstar 管理",
    });
  if (item.status === "missing" && safetyCritical)
    return t("assetsMatrix.detailMissingSafety", {
      defaultValue: "安全关键项未配置，建议尽快设置",
    });
  if (item.status === "missing")
    return t("assetsMatrix.detailMissing", {
      defaultValue: "未配置此项，如果不需要可忽略",
    });
  if (item.status === "partial")
    return t("assetsMatrix.detailPartial", {
      defaultValue: "部分配置，建议完善",
    });
  if (item.status === "not_applicable")
    return t("assetsMatrix.detailNA", {
      defaultValue: "当前目标 CLI 不支持此项",
    });
  if (item.effective_state === "not_applicable")
    return t("assetsMatrix.detailNA", {
      defaultValue: "当前目标 CLI 不支持此项",
    });

  return item.detail || t("assetsMatrix.noData", { defaultValue: "暂无扫描数据" });
}

// ── main component ────────────────────────────────────

export interface ProjectAssetsMatrixProps {
  projects: Project[];
  getStage: (projectId: string) => StageKey;
  progressMap?: Map<string, number>;
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  assetMap?: Map<string, ProjectAssetCounts>;
  loading?: boolean;
  onOpenProject: (project: Project, options?: { assetsTab?: boolean }) => void;
}

interface SelectedCell {
  project: Project;
  column: AssetColumn;
  item: AgentReadinessItem | undefined;
  state: CellState;
}

const STAGE_LABEL: Record<StageKey, string> = {
  mvp: "MVP",
  rapid: "迭代",
  stable: "稳定",
};

export function ProjectAssetsMatrix({
  projects,
  getStage,
  agentReadinessMap,
  loading,
  onOpenProject,
}: ProjectAssetsMatrixProps) {
  const { t } = useTranslation();
  const [filterMode, setFilterMode] = useState<MatrixFilter>("all");
  const [selectedCell, setSelectedCell] = useState<SelectedCell | null>(null);

  // find readiness item by check_name for a given project
  const getItem = useCallback(
    (projectId: string, checkName: string): AgentReadinessItem | undefined => {
      const entry = agentReadinessMap.get(projectId);
      return entry?.details.find((d) => d.check_name === checkName);
    },
    [agentReadinessMap],
  );

  // compute per-project state summary for filtering
  const projectState = useMemo(() => {
    const map = new Map<string, { state: CellState }>();
    for (const project of projects) {
      const counts: Record<CellState, number> = {
        normal: 0,
        attention: 0,
        abnormal: 0,
        unscanned: 0,
        not_applicable: 0,
      };
      for (const col of ASSET_COLUMNS) {
        const item = getItem(project.id, col.checkName);
        counts[getCellState(item, col.safetyCritical)] += 1;
      }
      const state: CellState =
        counts.abnormal > 0
          ? "abnormal"
          : counts.attention > 0
            ? "attention"
            : counts.unscanned > 0
              ? "unscanned"
              : counts.normal > 0
                ? "normal"
                : "not_applicable";
      map.set(project.id, { state });
    }
    return map;
  }, [projects, getItem]);

  const filteredProjects = useMemo(() => {
    if (filterMode === "all") return projects;
    return projects.filter((p) => {
      const state = projectState.get(p.id)?.state;
      if (filterMode === "needs_action") {
        return (
          state === "abnormal" ||
          state === "attention" ||
          state === "unscanned"
        );
      }
      return state === filterMode;
    });
  }, [projects, filterMode, projectState]);

  // counts for header
  const projectStateCounts = useMemo(() => {
    const counts: Record<CellState, number> = {
      normal: 0,
      attention: 0,
      abnormal: 0,
      unscanned: 0,
      not_applicable: 0,
    };
    for (const project of projects) {
      const state = projectState.get(project.id)?.state ?? "unscanned";
      counts[state] += 1;
    }
    return counts;
  }, [projects, projectState]);

  const needsActionCount =
    projectStateCounts.abnormal +
    projectStateCounts.attention +
    projectStateCounts.unscanned;

  const filterOptions: Array<{
    id: MatrixFilter;
    label: string;
    count: number;
    className: string;
  }> = [
    {
      id: "all",
      label: t("assetsMatrix.filterAll", { defaultValue: "全部" }),
      count: projects.length,
      className: "text-foreground",
    },
    {
      id: "needs_action",
      label: t("assetsMatrix.filterNeedsAction", { defaultValue: "需处理" }),
      count: needsActionCount,
      className: "text-amber-700 dark:text-amber-300",
    },
    {
      id: "abnormal",
      label: t("assetsMatrix.filterAbnormal", { defaultValue: "异常" }),
      count: projectStateCounts.abnormal,
      className: "text-red-700 dark:text-red-300",
    },
    {
      id: "unscanned",
      label: t("assetsMatrix.filterUnscanned", { defaultValue: "未扫" }),
      count: projectStateCounts.unscanned,
      className: "text-slate-700 dark:text-slate-300",
    },
  ];

  const handleCellClick = useCallback(
    (project: Project, column: AssetColumn) => {
      const item = getItem(project.id, column.checkName);
      const state = getCellState(item, column.safetyCritical);
      setSelectedCell({ project, column, item, state });
    },
    [getItem],
  );

  const handleCloseDetail = useCallback(() => setSelectedCell(null), []);

  if (loading && projects.length > 0 && agentReadinessMap.size === 0) {
    return (
      <div className="flex items-center justify-center py-16 text-muted-foreground text-sm">
        <Loader2 className="w-4 h-4 animate-spin mr-2" />
        {t("assetsMatrix.loading", { defaultValue: "正在扫描项目配置状态…" })}
      </div>
    );
  }

  if (projects.length === 0) return null;

  return (
    <div className="rounded-xl border border-border/60 bg-card/30 overflow-hidden">
      {/* header */}
      <div className="px-4 py-3 border-b border-border/40 flex items-center justify-between gap-3 flex-wrap">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {t("assetsMatrix.title", { defaultValue: "AI 配置状态" })}
          </h3>
          <p className="text-[11px] text-muted-foreground mt-0.5">
            {t("assetsMatrix.subtitle", {
              defaultValue:
                "每个格子显示一类 AI 配置的短状态：已生效、不一致、缺失、全局、探测、未扫、不适用。点击查看详情。",
            })}
          </p>
        </div>
        <div className="flex items-center gap-4">
          {/* traffic light summary */}
          <div className="flex flex-wrap items-center gap-3 text-xs">
            <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
              <span className="h-2 w-2 rounded-full bg-emerald-500" />
              {projectStateCounts.normal}{" "}
              {t("assetsMatrix.normal", { defaultValue: "正常" })}
            </span>
            <span className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
              <span className="h-2 w-2 rounded-full bg-amber-500" />
              {projectStateCounts.attention}{" "}
              {t("assetsMatrix.attention", { defaultValue: "需关注" })}
            </span>
            <span className="flex items-center gap-1 text-red-600 dark:text-red-400">
              <span className="h-2 w-2 rounded-full bg-red-500" />
              {projectStateCounts.abnormal}{" "}
              {t("assetsMatrix.abnormal", { defaultValue: "异常" })}
            </span>
            <span className="flex items-center gap-1 text-slate-600 dark:text-slate-300">
              <span className="h-2 w-2 rounded-full bg-slate-500" />
              {projectStateCounts.unscanned}{" "}
              {t("assetsMatrix.unscanned", { defaultValue: "未扫描" })}
            </span>
          </div>
          <div
            className="inline-flex rounded-md border border-border/60 bg-background/60 p-0.5"
            role="tablist"
            aria-label={t("assetsMatrix.filterLabel", {
              defaultValue: "项目状态筛选",
            })}
          >
            {filterOptions.map((option) => {
              const active = filterMode === option.id;
              return (
                <button
                  key={option.id}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  className={cn(
                    "h-7 rounded px-2.5 text-[11px] font-medium tabular-nums transition-colors",
                    active
                      ? "bg-card text-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                  )}
                  onClick={() => setFilterMode(option.id)}
                >
                  <span className={active ? option.className : undefined}>
                    {option.label}
                  </span>
                  <span className="ml-1 text-muted-foreground/80">
                    {option.count}
                  </span>
                </button>
              );
            })}
          </div>
          {loading && agentReadinessMap.size > 0 && (
            <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground shrink-0">
              <Loader2 className="h-3 w-3 animate-spin" />
              {t("assetsMatrix.refreshing", { defaultValue: "更新中" })}
            </span>
          )}
        </div>
      </div>

      {/* matrix body */}
      <div className="overflow-x-auto">
        <table className="w-full min-w-[800px] text-xs">
          <thead>
            <tr className="border-b border-border/40 bg-muted/20 text-muted-foreground">
              <th className="text-left font-medium px-4 py-2.5 sticky left-0 bg-muted/20 z-10 min-w-[140px]">
                {t("assetsMatrix.project", { defaultValue: "项目" })}
              </th>
              <th className="text-center font-medium px-2 py-2.5 w-12">
                {t("assetsMatrix.stage", { defaultValue: "阶段" })}
              </th>
              {ASSET_COLUMNS.map((col) => (
                <th
                  key={col.checkName}
                  className={cn(
                    "text-center font-medium px-1 py-2.5",
                    col.width,
                  )}
                  title={GOVERNANCE_CHECK_LABELS[col.checkName] ?? col.label}
                >
                  {col.label}
                </th>
              ))}
              <th className="text-center font-medium px-2 py-2.5 w-14">
                {t("assetsMatrix.score", { defaultValue: "分数" })}
              </th>
            </tr>
          </thead>
          <tbody>
            {filteredProjects.length === 0 && filterMode !== "all" && (
              <tr>
                <td
                  colSpan={ASSET_COLUMNS.length + 3}
                  className="text-center py-10 text-muted-foreground text-sm"
                >
                  <Check className="w-5 h-5 mx-auto mb-2 text-emerald-500" />
                  {t("assetsMatrix.allClear", {
                    defaultValue: "当前筛选下没有需要处理的项目",
                  })}
                </td>
              </tr>
            )}
            {filteredProjects.map((project) => {
              const stage = getStage(project.id);
              const readiness = agentReadinessMap.get(project.id);
              const score = readiness?.score;

              return (
                <tr
                  key={project.id}
                  className="border-b border-border/30 hover:bg-muted/10 transition-colors"
                >
                  {/* project name — click opens project detail */}
                  <td
                    className="px-4 py-2 sticky left-0 bg-card/95 z-10 cursor-pointer"
                    onClick={() => onOpenProject(project)}
                  >
                    <p className="font-medium text-foreground truncate max-w-[160px] hover:underline">
                      {project.name}
                    </p>
                  </td>

                  {/* stage */}
                  <td className="text-center px-2 py-2 text-muted-foreground">
                    {STAGE_LABEL[stage]}
                  </td>

                  {/* asset cells */}
                  {ASSET_COLUMNS.map((col) => {
                    const item = getItem(project.id, col.checkName);
                    const state = getCellState(item, col.safetyCritical);
                    const statusKind = getCellStatusKind(item, state);
                    return (
                      <td
                        key={col.checkName}
                        className={cn(
                          "text-center px-1 py-1.5 cursor-pointer",
                          "hover:brightness-110 transition-all",
                        )}
                        onClick={() => handleCellClick(project, col)}
                      >
                        <div
                          className={cn(
                            "inline-flex min-w-[56px] items-center justify-center gap-1 px-1.5 py-1 rounded text-[11px] leading-none font-medium",
                            cellClasses(state),
                          )}
                        >
                          <CellStatusIcon kind={statusKind} />
                          <span className="truncate">
                            {cellStatusLabel(statusKind, t)}
                          </span>
                        </div>
                      </td>
                    );
                  })}

                  {/* readiness score */}
                  <td className="text-center px-2 py-2">
                    {typeof score === "number" ? (
                      <span
                        className={cn(
                          "inline-flex items-center gap-0.5 font-semibold tabular-nums",
                          readinessScoreTone(score),
                        )}
                      >
                        <Shield className="h-3 w-3" />
                        {score}
                      </span>
                    ) : (
                      <span className="text-muted-foreground/40">—</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* slide-over detail panel */}
      {selectedCell && (
        <AssetDetailPanel
          cell={selectedCell}
          onClose={handleCloseDetail}
          onViewProject={() => {
            onOpenProject(selectedCell.project, { assetsTab: true });
            setSelectedCell(null);
          }}
          t={t}
        />
      )}
    </div>
  );
}

// ── detail slide-over panel ───────────────────────────

function AssetDetailPanel({
  cell,
  onClose,
  onViewProject,
  t,
}: {
  cell: SelectedCell;
  onClose: () => void;
  onViewProject: () => void;
  t: TFunction;
}) {
  const { project, column, item, state } = cell;
  const statusKind = getCellStatusKind(item, state);
  const assetLabel =
    GOVERNANCE_CHECK_LABELS[column.checkName] ?? column.label;

  return (
    <>
      {/* backdrop */}
      <div
        className="fixed inset-0 z-40 bg-black/20 backdrop-blur-[1px]"
        onClick={onClose}
      />
      {/* panel */}
      <div className="fixed inset-y-0 right-0 z-50 w-full max-w-sm bg-card border-l border-border shadow-xl flex flex-col animate-in slide-in-from-right duration-200">
        {/* header */}
        <div className="px-5 py-4 border-b border-border/60 flex items-center justify-between">
          <div>
            <p className="text-xs text-muted-foreground">{project.name}</p>
            <h3 className="text-sm font-semibold text-foreground mt-0.5">
              {assetLabel}
              {column.safetyCritical && (
                <span className="ml-2 inline-flex items-center gap-0.5 text-[10px] font-medium text-amber-600 dark:text-amber-400 bg-amber-500/10 px-1.5 py-0.5 rounded">
                  <AlertTriangle className="h-2.5 w-2.5" />
                  {t("assetsMatrix.safetyBadge", {
                    defaultValue: "安全关键",
                  })}
                </span>
              )}
            </h3>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-muted text-muted-foreground"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* body */}
        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
          {/* status section */}
          <div
            className={cn(
              "flex items-start gap-3 p-3 rounded-lg",
              state === "normal" &&
                "bg-emerald-500/10 border border-emerald-500/20",
              state === "attention" &&
                "bg-amber-500/10 border border-amber-500/20",
              state === "abnormal" &&
                "bg-red-500/10 border border-red-500/20",
              state === "unscanned" &&
                "bg-slate-500/10 border border-slate-500/20",
              state === "not_applicable" &&
                "bg-muted/30 border border-border/30",
            )}
          >
            {state === "normal" && (
              <Check className="h-5 w-5 text-emerald-500 shrink-0 mt-0.5" />
            )}
            {state === "attention" && (
              <AlertTriangle className="h-5 w-5 text-amber-500 shrink-0 mt-0.5" />
            )}
            {state === "abnormal" && (
              <X className="h-5 w-5 text-red-500 shrink-0 mt-0.5" />
            )}
            {state === "unscanned" && (
              <Clock className="h-5 w-5 text-slate-500 shrink-0 mt-0.5" />
            )}
            {state === "not_applicable" && (
              <Minus className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
            )}
            <div>
              <p className="text-sm font-medium">
                {cellStatusLabel(statusKind, t)}
              </p>
              <p className="text-xs text-muted-foreground mt-1 leading-relaxed">
                {cellDetailLabel(item, column.safetyCritical, t)}
              </p>
            </div>
          </div>

          {/* state detail */}
          {item?.effective_detail && (
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-1.5">
                {t("assetsMatrix.stateDetail", {
                  defaultValue: "状态详情",
                })}
              </p>
              <pre className="text-xs bg-muted/50 rounded-md p-3 whitespace-pre-wrap break-words leading-relaxed max-h-40 overflow-y-auto">
                {item.effective_detail}
              </pre>
            </div>
          )}

          {/* effective detail (from readiness item.detail) */}
          {item?.detail && item.detail !== item?.effective_detail && (
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-1.5">
                {t("assetsMatrix.checkDetail", {
                  defaultValue: "检查详情",
                })}
              </p>
              <p className="text-xs text-foreground/80 leading-relaxed">
                {item.detail}
              </p>
            </div>
          )}

          {/* live path */}
          {item?.live_path && (
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-1.5">
                {t("assetsMatrix.filePath", { defaultValue: "配置文件路径" })}
              </p>
              <code className="text-[11px] bg-muted/50 rounded px-2 py-1 block truncate">
                {item.live_path}
              </code>
            </div>
          )}
        </div>

        {/* footer actions */}
        <div className="px-5 py-3 border-t border-border/60 flex items-center gap-2">
          <button
            onClick={onViewProject}
            className="flex-1 flex items-center justify-center gap-1 text-xs py-2 rounded-md border border-border hover:bg-muted transition-colors"
          >
            {t("assetsMatrix.viewProject", {
              defaultValue: "查看项目资产",
            })}
            <ChevronRight className="h-3 w-3" />
          </button>
        </div>
      </div>
    </>
  );
}
