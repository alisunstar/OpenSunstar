import { useMemo } from "react";

import { useTranslation } from "react-i18next";

import { Clock, Loader2 } from "lucide-react";

import type { Project } from "@/types/project";

import type { StageKey } from "@/hooks/useProjectStages";

import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";

import { cn } from "@/lib/utils";

import {
  classifyReadinessLevel,
  shouldShowReadinessScore,
} from "@/lib/portfolioHealth";

import {
  PORTFOLIO_OVERVIEW_WINDOW_OPTIONS,
  formatCompactNumber,
  type PortfolioOverviewWindowDays,
} from "@/lib/portfolioMetrics";

import { SummaryCard } from "./SummaryCard";

import { AGENT_READINESS_MAX, isReadinessOk } from "@/lib/readinessConstants";

const MVP_PROGRESS_WARN = 50;

/**
 * 「今日工作台」的摘要区：一句话交代范围 + 一排指标卡。
 *
 * 它以前还挂着一份「建议优先处理」列表 —— 而同一个 Tab 里 `PortfolioHealthSummary`
 * 已经在画一份「需要动手的项目」列表。两份列表读同一个 `agentReadinessMap`，却
 * 各自算各自的理由、各自排各自的序、各自截断在 8 条和 6 条，于是同一个项目在
 * 上下两块里能给出不一样的说法。这正是审查报告 §3.1 那类重复挂载：不是「显示了
 * 两次」，是「两处开始说不一样的话」。
 *
 * 保留的是 `PortfolioHealthSummary`（它有交通灯分级、有修复入口、有六档状态），
 * 删掉的是这里这份。相应地，本组件不再需要 `assetMap`，也不再需要任何项目级
 * 动作回调 —— 它现在只出数，不出列表。
 */
export interface TodayWorkspaceProps {
  projects: Project[];

  getStage: (projectId: string) => StageKey;

  progressMap: Map<string, number>;

  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;

  /**
   * 已按 `overviewWindowDays` 选好口径的提交数（见
   * `usePortfolioDerivedMetrics.ts:124-126`）。这里刻意不叫 `commits7dMap`：
   * 之前固定读 7 天数据却用 `overviewWindowDays` 渲染标签，切到 30 天只有标题
   * 变了、数字没变（审查报告 §4.1）。窗口选择只允许有一个来源。
   */
  commitsInWindowMap: Map<string, number>;

  overviewWindowDays: number;

  /**
   * 窗口切换器跟着「近 N 天活跃」卡一起搬到这里。它原来挂在下方「项目总览」
   * 标题旁，而受它影响的指标同时分布在上下两排卡里 —— 改一个开关，屏幕上两个
   * 相隔半屏的区块一起变，中间还夹着两块与窗口无关的面板。
   */
  onOverviewWindowDaysChange: (days: PortfolioOverviewWindowDays) => void;

  totalCodeLines: number;

  totalCommitsInWindow: number;

  averageActivityLabel: string;

  averageActivityColor: string;

  lastUpdatedAt?: number | null;

  isRefreshing?: boolean;
}

function formatDashboardUpdatedAt(
  timestampMs: number,

  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  const diff = Date.now() - timestampMs;

  const minutes = Math.floor(diff / 60_000);

  const hours = Math.floor(diff / 3_600_000);

  if (minutes < 1) {
    return t("workspace.dashboard.updatedJustNow", {
      defaultValue: "刚刚更新",
    });
  }

  if (minutes < 60) {
    return t("workspace.dashboard.updatedMinutesAgo", {
      count: minutes,

      defaultValue: `${minutes} 分钟前更新`,
    });
  }

  if (hours < 24) {
    return t("workspace.dashboard.updatedHoursAgo", {
      count: hours,

      defaultValue: `${hours} 小时前更新`,
    });
  }

  return t("workspace.dashboard.updatedAt", {
    time: new Date(timestampMs).toLocaleString(),

    defaultValue: `更新于 ${new Date(timestampMs).toLocaleString()}`,
  });
}

export function TodayWorkspace({
  projects,

  getStage,

  progressMap,

  agentReadinessMap,

  commitsInWindowMap,

  overviewWindowDays,

  onOverviewWindowDaysChange,

  totalCodeLines,

  totalCommitsInWindow,

  averageActivityLabel,

  averageActivityColor,

  lastUpdatedAt,

  isRefreshing,
}: TodayWorkspaceProps) {
  const { t } = useTranslation();

  const stats = useMemo(() => {
    let readinessSum = 0;

    let readinessCount = 0;

    let activeProjects = 0;

    let mvpBehind = 0;

    for (const project of projects) {
      const readinessEntry = agentReadinessMap.get(project.id);

      // 未纳管 / 未扫描的项目：后端 classify_unmanaged_readiness
      // （agent_readiness.rs:387-413）已把分数归零并改判 status，此时零值是
      // 「不能判定」而不是「等于 0」，CLI 侧干脆给 `score: None`
      // （cli_api.rs:404）。把它当低分算进平均就是拿「还没配」冒充「坏了」——
      // 与 PortfolioHealthSummary 保持同一口径。
      const level = classifyReadinessLevel(readinessEntry);
      const judgeable = shouldShowReadinessScore(level);
      const readiness = judgeable ? readinessEntry?.score : undefined;

      if (typeof readiness === "number") {
        readinessSum += readiness;

        readinessCount += 1;
      }

      const commits = commitsInWindowMap.get(project.id) ?? 0;

      if (commits > 0) activeProjects += 1;

      const stage = getStage(project.id);

      const progress = progressMap.get(project.id);

      if (
        stage === "mvp" &&
        typeof progress === "number" &&
        progress < MVP_PROGRESS_WARN
      ) {
        mvpBehind += 1;
      }
    }

    return {
      avgReadiness:
        readinessCount > 0 ? Math.round(readinessSum / readinessCount) : null,

      activeProjects,

      mvpBehind,
    };
  }, [projects, agentReadinessMap, commitsInWindowMap, progressMap, getStage]);

  const showLoadingPlaceholder =
    Boolean(isRefreshing) && agentReadinessMap.size === 0;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          {/*
           * 这里原来还有一个 `<h3>今日工作台</h3>`，正上方的 Tab 按钮已经写着
           * 「今日工作台」—— 同一个词在一屏里连着出现两次，第二次不带任何新
           * 信息。留下副标题：它说的是「范围」，Tab 名说的是「你在哪儿」。
           */}
          <p className="text-xs text-muted-foreground max-w-xl">
            {t("workspace.dashboard.subtitle", {
              count: projects.length,

              days: overviewWindowDays,

              defaultValue: `共 ${projects.length} 个项目 · 优先关注进度与 AI 资产配置`,
            })}
          </p>
        </div>

        <div className="flex items-center gap-3 shrink-0">
          <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
            {isRefreshing ? (
              <>
                <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />

                <span>
                  {t("workspace.dashboard.refreshing", {
                    defaultValue: "正在刷新…",
                  })}
                </span>
              </>
            ) : lastUpdatedAt ? (
              <>
                <Clock className="h-3.5 w-3.5" />

                <span title={new Date(lastUpdatedAt).toLocaleString()}>
                  {formatDashboardUpdatedAt(lastUpdatedAt, t)}
                </span>
              </>
            ) : null}
          </div>

          <div
            className="flex items-center gap-1 rounded-lg border border-border/50 bg-muted/20 p-0.5"
            role="group"
            aria-label={t("board.summary.windowLabel", {
              defaultValue: "Git 活跃统计周期",
            })}
          >
            {PORTFOLIO_OVERVIEW_WINDOW_OPTIONS.map((days) => (
              <button
                key={days}
                type="button"
                className={cn(
                  "rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors",
                  overviewWindowDays === days
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
                onClick={() => onOverviewWindowDaysChange(days)}
                aria-pressed={overviewWindowDays === days}
              >
                {days} {t("board.summary.daysUnit", { defaultValue: "天" })}
              </button>
            ))}
          </div>
        </div>
      </div>

      {showLoadingPlaceholder ? (
        <div className="flex items-center justify-center py-10 text-sm text-muted-foreground rounded-xl border border-border/50 bg-card/20">
          <Loader2 className="w-4 h-4 animate-spin mr-2" />

          {t("workspace.dashboard.loading", {
            defaultValue: "正在汇总项目就绪与资产数据…",
          })}
        </div>
      ) : (
        <>
          {/*
           * 一排卡，不是两排。
           *
           * 这个 Tab 以前有两排四张：上排（待关注 / 平均就绪分 / 缺 MCP /
           * 近 N 天活跃）和下排（总项目数 / 总代码行数 / 平均活跃度 /
           * 近 N 天提交），中间隔着两块面板。八张里有三张在回答同一个问题：
           * 「近 N 天活跃」数的是有提交的项目数、「近 N 天提交」数的是提交
           * 总数、「平均活跃度」是把提交数分档 —— 而「近 N 天提交」的数值
           * 恰好就是「平均活跃度」那张卡的副标题，同一个数字在相邻两张卡上
           * 各画一次。现在三张合成一张：主数是活跃项目数，提交总数与活跃档
           * 落在副标题里。
           *
           * 另外两张删掉的是「待关注项目」和「缺 MCP 项目」：前者被下方
           * `PortfolioHealthSummary` 的交通灯分级完全覆盖（它还多给了四档
           * 状态和修复入口），后者把 8 类资产里的一类单独提成头条指标 ——
           * 缺哪类资产是逐项目的事，那份清单也在下面那块里。
           */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <SummaryCard
              label={t("board.summary.totalProjects", {
                defaultValue: "总项目数",
              })}
              value={String(projects.length)}
            />

            <SummaryCard
              label={t("workspace.dashboard.avgReadiness", {
                defaultValue: "平均就绪分",
              })}
              value={
                stats.avgReadiness !== null ? String(stats.avgReadiness) : "—"
              }
              unit={
                stats.avgReadiness !== null
                  ? `/${AGENT_READINESS_MAX}`
                  : undefined
              }
              color={
                stats.avgReadiness !== null && isReadinessOk(stats.avgReadiness)
                  ? "text-emerald-500"
                  : "text-amber-500"
              }
            />

            <SummaryCard
              label={t("workspace.dashboard.activeProjects", {
                days: overviewWindowDays,

                defaultValue: `近 ${overviewWindowDays} 天活跃`,
              })}
              value={String(stats.activeProjects)}
              unit={`/${projects.length}`}
              // 活跃档只写在文字里，不靠颜色单独承载：颜色是强调，不是信息。
              color={averageActivityColor}
              // 插值变量刻意不叫 `count`：i18next 把 `count` 当复数选择器，
              // 会去找 `activeSub_one` / `activeSub_other`，为一句没有复数
              // 变化的中文凭空引入一套复数键。这里只是个数字，就叫 commits。
              sub={t("workspace.dashboard.activeSub", {
                commits: totalCommitsInWindow,

                tier: averageActivityLabel,

                defaultValue: "共 {{commits}} 次提交 · 活跃度 {{tier}}",
              })}
            />

            <SummaryCard
              label={t("board.summary.totalCodeLines", {
                defaultValue: "总代码行数",
              })}
              value={
                totalCodeLines > 0 ? formatCompactNumber(totalCodeLines) : "—"
              }
              unit={
                totalCodeLines > 0
                  ? t("board.summary.linesUnit", { defaultValue: "行" })
                  : undefined
              }
            />
          </div>

          {stats.mvpBehind > 0 && (
            <p className="text-[11px] text-amber-600/90 dark:text-amber-400/90 px-1">
              {t("workspace.dashboard.mvpBehindHint", {
                count: stats.mvpBehind,

                defaultValue: `${stats.mvpBehind} 个 MVP 项目进度低于 50%，建议优先推进。`,
              })}
            </p>
          )}
        </>
      )}
    </div>
  );
}
