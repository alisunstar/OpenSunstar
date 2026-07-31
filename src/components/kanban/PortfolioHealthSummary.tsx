import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Loader2, Shield } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";
import type { ProjectAiConfigNavigationIntent } from "@/app/navigation";
import {
  classifyReadinessLevel,
  isActionableGap,
  shouldShowReadinessScore,
  type PortfolioHealthLevel,
} from "@/lib/portfolioHealth";
import { projectScoreTitle } from "@/lib/kanban/projectScores";
import { cn } from "@/lib/utils";

interface PortfolioHealthSummaryProps {
  projects: Project[];
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  /**
   * 保留以兼容调用方；等级与原因均以后端 readiness `details` 为准。
   * 资产计数是「已通过 OpenSunstar 关联多少」，与 `details` 的缺口判定同源，
   * 再叠加一遍只会在同一行里出现两句自相矛盾的话（审查报告 §3.2）。
   */
  assetMap: Map<string, ProjectAssetCounts>;
  loading?: boolean;
  /**
   * 打开项目详情抽屉（概览）。
   *
   * 它以前带一个 `options?: { assetsTab?: boolean }` —— 同一个动词后面挂个
   * boolean 决定去哪儿。而这个列表里**没有一处**用 `false` 调它：唯一的调用点
   * 永远传 `{ assetsTab: true }`，也就是说这个 prop 名说的是「打开项目」，做的
   * 是「去配 AI 资产」。现在两件事拆成两个回调，名字各自对得上。
   */
  onOpenProject: (project: Project) => void;
  /** 去「项目资产配置」页配这个项目。 */
  onOpenProjectAiConfig?: (
    project: Project,
    intent?: Pick<ProjectAiConfigNavigationIntent, "tab" | "section"> | null,
  ) => void;
  /**
   * 真正的漂移修复入口（`KanbanPage.handleRepairProjectDrift`：先拉预览、
   * 再由用户勾选确认，不会闷头写盘）。不传时 `alert` 的按钮会退回
   * 「查看详情」—— 宁可少一个入口，也不留一个说谎的按钮。
   */
  onRepairProject?: (project: Project) => void;
  /** 正在修复的项目 id，用于禁用按钮防重复提交 */
  repairingProjectId?: string | null;
}

interface HealthEntry {
  project: Project;
  level: PortfolioHealthLevel;
  reasons: string[];
  score: number | null;
}

/**
 * 六个等级分属两条轴，颜色不可混用：
 * - **健康轴**（绿/黄/红）：项目已纳管且已采纳资产，颜色反映真实健康度。
 * - **无判定轴**（灰）：尚未配置 / 未纳管 / 未扫描 —— 都不是「坏了」，
 *   一律灰色。把采纳度渲染成红色告警是本次修复的核心缺陷（审查报告 §1.3）。
 */
const LEVEL_META: Record<
  PortfolioHealthLevel,
  { dot: string; text: string; key: string; label: string }
> = {
  ok: {
    dot: "bg-emerald-500",
    text: "text-emerald-600 dark:text-emerald-400",
    key: "health.level.ok",
    label: "正常",
  },
  warn: {
    dot: "bg-amber-500",
    text: "text-amber-600 dark:text-amber-400",
    key: "health.level.warn",
    label: "需关注",
  },
  alert: {
    dot: "bg-red-500",
    text: "text-red-600 dark:text-red-400",
    key: "health.level.alert",
    label: "异常",
  },
  unconfigured: {
    dot: "bg-slate-400",
    text: "text-slate-600 dark:text-slate-300",
    key: "health.level.unconfigured",
    label: "尚未配置",
  },
  unmanaged: {
    dot: "bg-slate-400",
    text: "text-slate-600 dark:text-slate-300",
    key: "health.level.unmanaged",
    label: "未纳管",
  },
  unscanned: {
    dot: "bg-slate-500",
    text: "text-slate-600 dark:text-slate-300",
    key: "health.level.unscanned",
    label: "未扫描",
  },
};

/** 状态条展示顺序：健康轴由轻到重，无判定轴殿后。 */
const LEVEL_ORDER: PortfolioHealthLevel[] = [
  "ok",
  "warn",
  "alert",
  "unconfigured",
  "unmanaged",
  "unscanned",
];

/** 待办列表排序：真实故障优先，无判定态靠后。 */
const LEVEL_RANK: Record<PortfolioHealthLevel, number> = {
  alert: 0,
  warn: 1,
  unconfigured: 2,
  unmanaged: 3,
  unscanned: 4,
  ok: 5,
};

/**
 * 按钮文案 = 对用户的承诺，必须与它真正触发的动作一致（审查报告 §4.3）。
 *
 * 除 `alert` 外都是「带你去能做这件事的地方」。
 * `alert` 是唯一一个原本承诺「修复」却只打开抽屉的：它现在要么接上真修复
 * 链路，要么退回 `ACTION_INSPECT_DETAIL` 老实说自己只是「查看详情」。
 *
 * `to` 是这条承诺的落点。以前六个等级不分青红皂白全都 `{ assetsTab: true }`，
 * 于是「查看项目」按下去落在资产勾选页 —— 文案和落点又对不上了，只是这次
 * 反向。现在把落点写进这张表：说「配置 / 纳管」就去配置页，说「查看」就开抽屉。
 */
const LEVEL_ACTION: Record<
  PortfolioHealthLevel,
  { key: string; label: string; to: "detail" | "aiConfig" }
> = {
  alert: { key: "health.action.repair", label: "修复漂移", to: "aiConfig" },
  warn: { key: "health.action.configure", label: "配置资产", to: "aiConfig" },
  unconfigured: { key: "health.action.setup", label: "去配置", to: "aiConfig" },
  unmanaged: { key: "health.action.adopt", label: "纳入管理", to: "aiConfig" },
  unscanned: { key: "health.action.inspect", label: "查看项目", to: "detail" },
  ok: { key: "health.action.inspect", label: "查看项目", to: "detail" },
};

/** 没接上修复回调时 `alert` 的退路：不许出现「修复」二字。 */
const ACTION_INSPECT_DETAIL = {
  key: "health.action.inspectDetail",
  label: "查看详情",
  to: "detail" as const,
};

function classifyProject(
  project: Project,
  readiness: AgentReadinessBatchEntry | undefined,
  t: (key: string, opts?: Record<string, unknown>) => string,
): HealthEntry {
  const level = classifyReadinessLevel(readiness);
  const details = readiness?.details ?? [];
  const driftCount = readiness?.driftCount ?? 0;
  // 未纳管 / 未扫描不展示分数，与 CLI `score: None`（cli_api.rs:404）一致
  const score = shouldShowReadinessScore(level)
    ? (readiness?.score ?? null)
    : null;
  const reasons: string[] = [];

  if (level === "unscanned") {
    return {
      project,
      level,
      score,
      reasons: [
        t("health.reason.unscanned", {
          defaultValue: "尚未完成 AI 配置状态扫描",
        }),
      ],
    };
  }

  if (level === "unmanaged") {
    return {
      project,
      level,
      score,
      reasons: [
        t("health.reason.unmanaged", {
          defaultValue: "项目尚未纳入 OpenSunstar，不能判定为缺失",
        }),
      ],
    };
  }

  // 漂移是唯一真正「坏了」的信号，永远排在最前
  if (driftCount > 0) {
    reasons.push(
      t("health.reason.drifted", {
        count: driftCount,
        defaultValue: `${driftCount} 处配置与预期不一致`,
      }),
    );
  }

  if (level === "unconfigured") {
    reasons.push(
      t("health.reason.unconfigured", {
        defaultValue: "尚未通过 OpenSunstar 关联任何 AI 资产",
      }),
    );
  }

  const detectedOnly = details.filter((d) => d.status === "detected_only");
  if (detectedOnly.length > 0) {
    const items = detectedOnly.map((d) => d.label).join("、");
    reasons.push(
      t("health.reason.detectedOnly", {
        items,
        defaultValue: `${items} — 发现配置文件但未纳入管理`,
      }),
    );
  }

  const globalOnly = details.filter((d) => d.status === "global_only");
  if (globalOnly.length > 0) {
    const items = globalOnly.map((d) => d.label).join("、");
    reasons.push(
      t("health.reason.globalOnly", {
        items,
        defaultValue: `${items} — 使用全局默认配置`,
      }),
    );
  }

  // 已采纳但仍有缺口时点名缺什么。上面两类已单独成句，此处排除避免重复；
  // not_required / unmanaged / unknown 由 isActionableGap 兜底排除。
  if (level === "warn") {
    const gaps = details.filter(
      (d) =>
        isActionableGap(d) &&
        d.status !== "detected_only" &&
        d.status !== "global_only",
    );
    if (gaps.length > 0) {
      const items = gaps.map((d) => d.label).join("、");
      reasons.push(
        t("health.reason.gaps", {
          items,
          defaultValue: `待补齐：${items}`,
        }),
      );
    }
  }

  return { project, level, reasons, score };
}

export function PortfolioHealthSummary({
  projects,
  agentReadinessMap,
  loading,
  onOpenProject,
  onOpenProjectAiConfig,
  onRepairProject,
  repairingProjectId = null,
}: PortfolioHealthSummaryProps) {
  const { t } = useTranslation();

  const entries = useMemo(
    () =>
      projects.map((p) => classifyProject(p, agentReadinessMap.get(p.id), t)),
    [projects, agentReadinessMap, t],
  );

  const counts = useMemo(() => {
    const acc = {
      ok: 0,
      warn: 0,
      alert: 0,
      unconfigured: 0,
      unmanaged: 0,
      unscanned: 0,
    } as Record<PortfolioHealthLevel, number>;
    for (const e of entries) acc[e.level] += 1;
    return acc;
  }, [entries]);

  // 待办列表 = 需要人动手的项目，不等于「异常项目」。
  // 灰色三态（尚未配置/未纳管/未扫描）也在列，但排在真实故障之后。
  const problemEntries = useMemo(
    () =>
      entries
        .filter((e) => e.level !== "ok")
        .sort(
          (a, b) =>
            LEVEL_RANK[a.level] - LEVEL_RANK[b.level] ||
            (a.score ?? Number.MAX_SAFE_INTEGER) -
              (b.score ?? Number.MAX_SAFE_INTEGER) ||
            a.project.name.localeCompare(b.project.name),
        )
        .slice(0, 6),
    [entries],
  );

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
          {LEVEL_ORDER.filter((level) => counts[level] > 0).map((level) => {
            const meta = LEVEL_META[level];
            return (
              <span
                key={level}
                className="inline-flex items-center gap-1.5 text-xs font-semibold tabular-nums"
              >
                <span className={cn("w-2.5 h-2.5 rounded-full", meta.dot)} />
                <span className={meta.text}>{counts[level]}</span>
                <span className="text-muted-foreground font-normal">
                  {t(meta.key, { defaultValue: meta.label })}
                </span>
              </span>
            );
          })}
        </div>
      </div>

      {/* Projects needing action */}
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
                  LEVEL_META[level].dot,
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
                // 分数按等级着色，不按阈值着色：score 是采纳度指标，
                // 低分本身不构成告警（审查报告 §1.3）。
                // 名字由 `projectScoreTitle` 统一给：在此之前这里是一个光秃秃
                // 的数字加一个盾牌图标，悬停什么也不说，读屏器念出来就是
                // 「42」——「一个分数三个名字」的第二处（§5.2）。
                // 点击分数直接跳到「项目资产配置」的 readiness tab，落点精确。
                <button
                  type="button"
                  className={cn(
                    "inline-flex items-center gap-0.5 text-[10px] font-semibold tabular-nums shrink-0 hover:opacity-80 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded",
                    LEVEL_META[level].text,
                  )}
                  title={projectScoreTitle("agentReadiness", score, t)}
                  onClick={() =>
                    onOpenProjectAiConfig?.(project, { tab: "readiness" })
                  }
                >
                  <Shield className="h-3 w-3" />
                  {score}
                </button>
              )}
              {(() => {
                // 只有「真实漂移」有东西可修；其余等级点下去是去配置/去查看。
                const canRepair = level === "alert" && !!onRepairProject;
                const action = canRepair
                  ? LEVEL_ACTION.alert
                  : level === "alert"
                    ? ACTION_INSPECT_DETAIL
                    : LEVEL_ACTION[level];
                const repairing = repairingProjectId === project.id;
                return (
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs shrink-0 opacity-80 group-hover:opacity-100"
                    disabled={canRepair && repairing}
                    onClick={() => {
                      if (canRepair) {
                        onRepairProject?.(project);
                        return;
                      }
                      if (action.to === "aiConfig" && onOpenProjectAiConfig) {
                        onOpenProjectAiConfig(project);
                        return;
                      }
                      onOpenProject(project);
                    }}
                  >
                    {canRepair && repairing && (
                      <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                    )}
                    {t(action.key, { defaultValue: action.label })}
                  </Button>
                );
              })()}
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
