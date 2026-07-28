import {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import {
  Activity,
  AlertTriangle,
  Check,
  ChevronRight,
  Clock,
  Globe2,
  History,
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
import { projectScoreTitle } from "@/lib/kanban/projectScores";
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
  | "attention"
  | "active"
  | "stale";

interface AssetColumn {
  checkName: string;
  label: string;
  safetyCritical: boolean;
  width: string;
  /**
   * `asset` 是磁盘上的资产，有生效态可比对；`metric` 是维护度指标，只有
   * 「是/否」，没有可写回磁盘的东西（`project_config_sync.rs:310` 明确拒绝
   * 修复它）。两者的判定逻辑不能共用，见 `getCellState`。
   */
  kind: "asset" | "metric";
  /**
   * 该列在 `ProjectAllAssetCounts` 里的计数字段。metric 列没有可数实体，
   * 因此为空 —— 「维护度：3 个」不是一句人话。
   */
  countKey?: keyof ProjectAssetCounts;
}

const ASSET_COLUMNS: AssetColumn[] = [
  {
    checkName: "mcp_enabled",
    label: "MCP",
    safetyCritical: false,
    width: "w-[72px]",
    kind: "asset",
    countKey: "mcp",
  },
  {
    checkName: "skills_configured",
    label: "Skills",
    safetyCritical: false,
    width: "w-[72px]",
    kind: "asset",
    countKey: "skills",
  },
  {
    checkName: "prompt_files",
    label: "Prompts",
    safetyCritical: false,
    width: "w-[72px]",
    kind: "asset",
    countKey: "prompts",
  },
  {
    checkName: "commands_configured",
    label: "Commands",
    safetyCritical: false,
    width: "w-[86px]",
    kind: "asset",
    countKey: "commands",
  },
  {
    checkName: "hooks_configured",
    label: "Hooks",
    safetyCritical: true,
    width: "w-[64px]",
    kind: "asset",
    countKey: "hooks",
  },
  {
    checkName: "ignore_rules",
    label: "Ignore",
    safetyCritical: true,
    width: "w-[68px]",
    kind: "asset",
    countKey: "ignore",
  },
  {
    checkName: "permissions",
    label: "Permissions",
    safetyCritical: true,
    width: "w-[98px]",
    kind: "asset",
    countKey: "permissions",
  },
  {
    checkName: "subagents_configured",
    label: "Subagents",
    safetyCritical: false,
    width: "w-[92px]",
    kind: "asset",
    countKey: "subagents",
  },
  /**
   * 第 9 项（审查报告 §5.3）。`agent_readiness.rs:339-358` 一直在给它计
   * 9 分，矩阵却只有 8 列 ——「8 格全绿但只有 91 分」在界面上无处解释。
   */
  {
    checkName: "recent_updates",
    label: "维护度",
    safetyCritical: false,
    width: "w-[72px]",
    kind: "metric",
  },
];

/** 只有资产列参与「这个项目状态如何」的聚合，理由见 `projectState`。 */
const AGGREGATED_COLUMNS = ASSET_COLUMNS.filter((c) => c.kind === "asset");

/**
 * 表头文案与 tooltip。
 *
 * 8 个资产列是产品名（MCP / Skills / Hooks…），各语言通用，不进 i18n；
 * 「维护度」是普通名词，硬编码会把中文漏进 en/ja 界面。
 *
 * 三个缩写列已改回全称（审查报告 §2.5）：`Cmds` / `Perms` / `Subs` 是为了
 * 迁就 `w-[64px]` 硬编码列宽才砍出来的，而侧栏里同一批实体一直写的是全称
 * （`Sidebar.tsx` 的 Commands / Permissions / Subagents）—— 同一个东西在
 * 两个界面上两个名字。列宽随之放宽到能装下全称，`w-[64px]` 不再出现。
 */
function columnHeader(
  col: AssetColumn,
  t: TFunction,
): { text: string; title: string } {
  if (col.kind === "metric") {
    return {
      text: t("assetsMatrix.colUpkeep", { defaultValue: "维护度" }),
      title: t("assetsMatrix.colUpkeepTitle", {
        defaultValue: "近 90 天项目资产关联更新",
      }),
    };
  }
  return {
    text: col.label,
    title: GOVERNANCE_CHECK_LABELS[col.checkName] ?? col.label,
  };
}

// ── cell health determination ──────────────────────────

/**
 * 维护度只有两种结局：近 90 天动过（满分）或没动过（0 分）。
 *
 * 它必须绕开资产列的整套判定：`asset_effective_state.rs:1627-1633` 给它写死
 * `effective_state: "not_applicable"`，走资产分支会让「有更新」落进
 * 「不适用」那条灰色分支 —— 那 9 分就更没人看得懂了。
 *
 * 「没更新」是琥珀色而不是红色：项目 90 天没改配置是一个事实，不是一个缺陷。
 */
function getMetricCellState(item: AgentReadinessItem): CellState {
  switch (item.status) {
    case "ready":
      return "normal";
    case "missing":
      return "attention";
    case "not_required":
      return "not_applicable";
    default:
      // unmanaged / unknown / 其余：不可判定，一律按未判定处理
      return "unscanned";
  }
}

function getCellState(
  item: AgentReadinessItem | undefined,
  column: Pick<AssetColumn, "kind" | "safetyCritical">,
): CellState {
  if (!item) return "unscanned";
  if (column.kind === "metric") return getMetricCellState(item);
  const safetyCritical = column.safetyCritical;

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
  // 必须穷举 agent_readiness.rs:11-19 的全部 9 个取值：漏一个就会落到下面
  // `score > 0` 的兜底，把「不适用」渲染成绿色、把「不可判定」渲染成红色。
  switch (item.status) {
    case "ready":
      return "normal";
    case "partial":
    case "global_only":
    case "detected_only":
      return "attention";
    case "not_required":
      // 目标 CLI 不支持此项，后端给满分并从评分中排除（agent_readiness.rs:85-87）
      return "not_applicable";
    case "unhealthy":
      // 检出漂移时由 asset_effective_state.rs:1667 覆写
      return "abnormal";
    case "unmanaged":
    case "unknown":
      // 零计数不能证明缺失（agent_readiness.rs:387-413），一律按未判定处理
      return "unscanned";
    case "missing":
      return safetyCritical ? "abnormal" : "attention";
  }

  // last resort: score
  return item.score > 0 ? "normal" : safetyCritical ? "abnormal" : "attention";
}

// ── cell display helpers ──────────────────────────────

function getCellStatusKind(
  item: AgentReadinessItem | undefined,
  state: CellState,
  column: Pick<AssetColumn, "kind"> = { kind: "asset" },
): CellStatusKind {
  if (!item) return "unscanned";

  // 维护度不说「已生效 / 缺失」——它没有生效态，也没有东西可缺
  if (column.kind === "metric") {
    switch (state) {
      case "normal":
        return "active";
      case "attention":
        return "stale";
      case "not_applicable":
        return "not_applicable";
      default:
        return "unscanned";
    }
  }

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
    case "not_required":
      return "not_applicable";
    case "unhealthy":
      return "mismatch";
    case "unmanaged":
    case "unknown":
      return "unscanned";
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
    case "active":
      return t("assetsMatrix.cellRecent", { defaultValue: "有更新" });
    case "stale":
      return t("assetsMatrix.cellStale", { defaultValue: "无更新" });
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
    case "active":
      return <Activity className="h-3 w-3 shrink-0" />;
    case "stale":
      return <History className="h-3 w-3 shrink-0" />;
  }
}

// ── detail label for slide-over ───────────────────────

function cellDetailLabel(
  item: AgentReadinessItem | undefined,
  column: Pick<AssetColumn, "kind" | "safetyCritical">,
  t: TFunction,
): string {
  if (!item)
    return t("assetsMatrix.scanPending", {
      defaultValue: "尚未完成扫描，当前状态不能判定为正常。",
    });

  // 维护度的后端 detail 已经是完整的一句话（agent_readiness.rs:348-352），
  // 直接透传比再包一层「不适用」准确
  if (column.kind === "metric")
    return (
      item.detail ||
      t("assetsMatrix.detailRecentUpdates", {
        defaultValue: "统计近 90 天内是否有项目级 AI 资产配置变更",
      })
    );

  const safetyCritical = column.safetyCritical;

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
  // 不可判定态优先于「缺失」类文案：未纳管时零计数不是缺失的证据
  // （agent_readiness.rs:387-413）。后端已给出解释性 detail，优先透传。
  if (item.status === "unmanaged" || item.status === "unknown")
    return (
      item.detail ||
      t("assetsMatrix.detailUnmanaged", {
        defaultValue: "项目尚未纳入 OpenSunstar，不能判定为缺失",
      })
    );
  // 目标 CLI 不支持此项，后端给满分并附带说明（agent_readiness.rs:85-87）
  if (item.status === "not_required")
    return (
      item.detail ||
      t("assetsMatrix.detailNA", {
        defaultValue: "当前目标 CLI 不支持此项",
      })
    );
  if (item.status === "unhealthy")
    return (
      item.effective_detail ||
      item.detail ||
      t("assetsMatrix.detailDrifted", {
        defaultValue: "配置与预期不一致，可能需要修复",
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
  if (item.effective_state === "not_applicable")
    return t("assetsMatrix.detailNA", {
      defaultValue: "当前目标 CLI 不支持此项",
    });

  return (
    item.detail || t("assetsMatrix.noData", { defaultValue: "暂无扫描数据" })
  );
}

// ── main component ────────────────────────────────────

export interface ProjectAssetsMatrixProps {
  projects: Project[];
  getStage: (projectId: string) => StageKey;
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  /**
   * 各项目「通过 OpenSunstar 关联了多少」的计数。注意它回答的不是磁盘上
   * 有几个 —— 缺省或计数为 0 都不构成「缺失」的证据（审查报告 §5.4）。
   */
  assetMap?: Map<string, ProjectAssetCounts>;
  loading?: boolean;
  /** 点项目名 → 打开项目详情抽屉（概览）。 */
  onOpenProject: (project: Project) => void;
  /**
   * 单元格详情里的「查看项目资产」→ 去「项目资产配置」页。
   *
   * 这两件事以前挤在同一个 `onOpenProject(project, { assetsTab?: boolean })`
   * 里，靠一个 boolean 分流。落点已经是两个页面了，回调也就该是两个。
   */
  onOpenProjectAiConfig: (project: Project) => void;
}

interface SelectedCell {
  project: Project;
  column: AssetColumn;
  item: AgentReadinessItem | undefined;
  state: CellState;
  /** 该格对应的已关联数量；undefined = 没取到，不等于 0 */
  count: number | undefined;
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
  assetMap,
  loading,
  onOpenProject,
  onOpenProjectAiConfig,
}: ProjectAssetsMatrixProps) {
  const { t } = useTranslation();
  const [filterMode, setFilterMode] = useState<MatrixFilter>("all");
  const [selectedCell, setSelectedCell] = useState<SelectedCell | null>(null);

  /**
   * projectId → checkName → item 的两级索引（审查报告 §6.5）。
   *
   * 原来 `getItem` 是 `details.find(...)` 线性查找，而整张表要算三遍
   * （projectState → projectStateCounts → 渲染），复杂度 O(P×C×D)。
   * 索引把每次查找摊平成 O(1)，建索引本身只在 `agentReadinessMap` 变化时
   * 跑一次 O(P×D)。
   */
  const readinessIndex = useMemo(() => {
    const index = new Map<string, Map<string, AgentReadinessItem>>();
    for (const [projectId, entry] of agentReadinessMap) {
      const byCheck = new Map<string, AgentReadinessItem>();
      for (const detail of entry.details)
        byCheck.set(detail.check_name, detail);
      index.set(projectId, byCheck);
    }
    return index;
  }, [agentReadinessMap]);

  const getItem = useCallback(
    (projectId: string, checkName: string): AgentReadinessItem | undefined =>
      readinessIndex.get(projectId)?.get(checkName),
    [readinessIndex],
  );

  /**
   * 取该格「已关联多少」。返回 undefined 有两种含义，都不是 0：
   * 该列没有可数实体（维护度），或这一轮根本没取到 assetMap。
   */
  const getCount = useCallback(
    (projectId: string, column: AssetColumn): number | undefined =>
      column.countKey ? assetMap?.get(projectId)?.[column.countKey] : undefined,
    [assetMap],
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
      // 只聚合资产列：「需处理」回答的是「今天该动哪个项目」，而「90 天没改
      // 过配置」是一个事实、不是一个缺陷。把维护度算进去会让所有稳定项目
      // 永远挂在告警里 —— 正是第一梯队刚修掉的那种狼来了。
      for (const col of AGGREGATED_COLUMNS) {
        const item = getItem(project.id, col.checkName);
        counts[getCellState(item, col)] += 1;
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
          state === "abnormal" || state === "attention" || state === "unscanned"
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
      const state = getCellState(item, column);
      setSelectedCell({
        project,
        column,
        item,
        state,
        count: getCount(project.id, column),
      });
    },
    [getItem, getCount],
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
          {/*
           * 这一排原来是 `role="tablist"` + `role="tab"`（审查报告 §7）。
           *
           * 它们不是 Tab：Tab 的含义是「切换到另一块内容」，读屏器据此承诺
           * 存在一个对应的 tabpanel，并且用户会按方向键在 Tab 之间走。这里
           * 既没有 tabpanel（切的是同一张表格的行），方向键也什么都不做 ——
           * 一个说到做不到的 role 比没有 role 更坏。
           *
           * 它们的真身是一组互斥的筛选开关，因此 `role="group"` +
           * `aria-pressed`：读屏器念「全部，已按下，按钮」，字字属实，
           * 也不需要额外的键盘逻辑（button 本来就走 Tab 键 + Enter/Space）。
           */}
          <div
            className="inline-flex rounded-md border border-border/60 bg-background/60 p-0.5"
            role="group"
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
                  aria-pressed={active}
                  className={cn(
                    "h-7 rounded px-2.5 text-[11px] font-medium tabular-nums transition-colors",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
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
            {/*
             * `scope` 不是装饰：读屏器在表格导航模式下靠它把「当前格子」和
             * 「它属于哪一列 / 哪一行」关联起来。没有 scope 时，一个孤立的
             * 「已生效」被念出来，用户不知道是哪个项目的哪一项。
             */}
            <tr className="border-b border-border/40 bg-muted/20 text-muted-foreground">
              <th
                scope="col"
                className="text-left font-medium px-4 py-2.5 sticky left-0 bg-muted/20 z-10 min-w-[140px]"
              >
                {t("assetsMatrix.project", { defaultValue: "项目" })}
              </th>
              <th
                scope="col"
                className="text-center font-medium px-2 py-2.5 w-12"
              >
                {t("assetsMatrix.stage", { defaultValue: "阶段" })}
              </th>
              {ASSET_COLUMNS.map((col) => {
                const header = columnHeader(col, t);
                return (
                  <th
                    key={col.checkName}
                    scope="col"
                    className={cn(
                      "text-center font-medium px-1 py-2.5",
                      col.width,
                    )}
                    title={header.title}
                  >
                    {header.text}
                  </th>
                );
              })}
              <th
                scope="col"
                className="text-center font-medium px-2 py-2.5 w-14"
              >
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
                  {/*
                   * 项目名格子 —— 点开项目详情。
                   *
                   * 两处都不是样式改动（审查报告 §7）：
                   * 1. `<td>` → `<th scope="row">`：这一格是整行的行头，读屏器
                   *    在表格里横向走的时候会自动带上它，右边 9 个格子才有主语。
                   * 2. `<p onClick>` → `<button>`：原来整个 `<td>` 挂 onClick，
                   *    键盘走不到、读屏器不报「可点击」，鼠标之外的用户看得见
                   *    但打不开。用原生 button 而不是给 `<td>` 加
                   *    `role="button" tabIndex={0}` —— 后者会把这一格从「表格
                   *    单元」变成「按钮」，表格结构当场散掉。
                   */}
                  <th
                    scope="row"
                    className="px-4 py-2 sticky left-0 bg-card/95 z-10 text-left font-normal"
                  >
                    <button
                      type="button"
                      onClick={() => onOpenProject(project)}
                      className="font-medium text-foreground truncate max-w-[160px] hover:underline text-left rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      {project.name}
                    </button>
                  </th>

                  {/* stage */}
                  <td className="text-center px-2 py-2 text-muted-foreground">
                    {STAGE_LABEL[stage]}
                  </td>

                  {/* asset cells */}
                  {ASSET_COLUMNS.map((col) => {
                    const item = getItem(project.id, col.checkName);
                    const state = getCellState(item, col);
                    const statusKind = getCellStatusKind(item, state, col);
                    const count = getCount(project.id, col);
                    return (
                      <td
                        key={col.checkName}
                        className="text-center px-1 py-1.5"
                      >
                        {/*
                         * onClick 从 `<td>` 挪到这个 button 上（审查报告 §7）。
                         *
                         * `aria-label` 把主语补回来：格子里可见的只有「已生效」
                         * 三个字，读屏器把焦点停在这里时念出的就是这三个字 ——
                         * 哪个项目、哪一项，全靠这条标签。行头 + 列头只在**表格
                         * 导航模式**下自动播报，而绝大多数人是按 Tab 走过来的。
                         */}
                        <button
                          type="button"
                          onClick={() => handleCellClick(project, col)}
                          aria-label={`${project.name} · ${columnHeader(col, t).title} · ${cellStatusLabel(statusKind, t)}`}
                          className={cn(
                            "inline-flex min-w-[56px] items-center justify-center gap-1 px-1.5 py-1 rounded text-[11px] leading-none font-medium",
                            "hover:brightness-110 transition-all",
                            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                            cellClasses(state),
                          )}
                        >
                          <CellStatusIcon kind={statusKind} />
                          <span className="truncate">
                            {cellStatusLabel(statusKind, t)}
                          </span>
                          {/* 只在 > 0 时出数：0 与「没取到」在界面上必须同样
                              沉默，否则就是替后端下了「缺失」的结论 */}
                          {count !== undefined && count > 0 && (
                            <span className="tabular-nums opacity-70">
                              {count}
                            </span>
                          )}
                        </button>
                      </td>
                    );
                  })}

                  {/* readiness score —— 名字由 `projectScoreTitle` 统一给（§5.2） */}
                  <td className="text-center px-2 py-2">
                    {typeof score === "number" ? (
                      <span
                        className={cn(
                          "inline-flex items-center gap-0.5 font-semibold tabular-nums",
                          readinessScoreTone(score),
                        )}
                        title={projectScoreTitle("agentReadiness", score, t)}
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
            onOpenProjectAiConfig(selectedCell.project);
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
  const { project, column, item, state, count } = cell;
  const statusKind = getCellStatusKind(item, state, column);
  // metric 列走 i18n，资产列沿用产品名（`columnHeader` 里说明了理由）
  const assetLabel =
    column.kind === "metric"
      ? columnHeader(column, t).text
      : (GOVERNANCE_CHECK_LABELS[column.checkName] ?? column.label);

  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();

  /**
   * 这块面板原来是一个纯 `<div>`（审查报告 §7）：没有 dialog 语义、Esc
   * 关不掉、焦点留在背后那张表上、关闭后焦点掉回 `<body>`。
   *
   * 三件事在这一个 effect 里做完，因为它们是同一件事的三个面：**焦点在
   * 打开期间必须待在这块面板里**。
   *
   * 没有直接换成 `@/components/ui/dialog`（Radix）：那套是居中弹窗，
   * 这块是右侧滑出的 slide-over，Radix 的定位与进出场动画得整体覆写，
   * 改动面比这个 effect 大得多。同样的欠账也在 `ProjectDetailSheet` 上，
   * 产品已明确「拆完再改，本轮不动」—— 两处将来一起换。
   */
  useEffect(() => {
    const previouslyFocused = document.activeElement as HTMLElement | null;
    // 打开时把焦点送进面板：否则读屏器还停在表格上，念不到刚弹出的内容。
    panelRef.current?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key !== "Tab") return;

      const panel = panelRef.current;
      if (!panel) return;
      const focusable = panel.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      // 焦点环：不闭合的话 Tab 会走到背后那张表上，而那张表此刻被遮罩盖着，
      // 视觉上焦点就凭空消失了。
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      // 关闭后焦点回到打开它的那个格子，而不是掉回 <body> 从头再 Tab 一遍。
      previouslyFocused?.focus?.();
    };
  }, [onClose]);

  return (
    <>
      {/* backdrop */}
      <div
        className="fixed inset-0 z-40 bg-black/20 backdrop-blur-[1px]"
        onClick={onClose}
        aria-hidden="true"
      />
      {/* panel */}
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        className="fixed inset-y-0 right-0 z-50 w-full max-w-sm bg-card border-l border-border shadow-xl flex flex-col animate-in slide-in-from-right duration-200 focus:outline-none"
      >
        {/* header */}
        <div className="px-5 py-4 border-b border-border/60 flex items-center justify-between">
          <div>
            <p className="text-xs text-muted-foreground">{project.name}</p>
            <h3
              id={titleId}
              className="text-sm font-semibold text-foreground mt-0.5"
            >
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
              state === "abnormal" && "bg-red-500/10 border border-red-500/20",
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
                {cellDetailLabel(item, column, t)}
              </p>
            </div>
          </div>

          {/* 已关联数量。措辞必须点明口径：这是 OpenSunstar 里的关联数，
              不是磁盘上扫到的数量，两者可以不一致（审查报告 §5.4）。
              0 也照常展示 —— 详情页有空间把话说完整，格子里没有。 */}
          {count !== undefined && (
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-1.5">
                {t("assetsMatrix.linkedCount", {
                  defaultValue: "OpenSunstar 已关联",
                })}
              </p>
              <p className="text-xs text-foreground/80 tabular-nums">
                {t("assetsMatrix.linkedCountValue", {
                  // 刻意不叫 `count`：那是 i18next 的复数魔法键，会去找
                  // `_one`/`_other` 变体，中文根本不需要这套。
                  value: count,
                  defaultValue: `${count} 项`,
                })}
              </p>
            </div>
          )}

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
