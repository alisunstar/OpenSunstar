import { useMemo, useState } from "react";

import { useTranslation } from "react-i18next";

import {
  ArrowRight,
  ChevronDown,
  ChevronRight,
  Loader2,
  Radar,
  RefreshCw,
  Shield,
  Wrench,
} from "lucide-react";

import { Button } from "@/components/ui/button";

import type {
  AgentReadinessResult,
  ReadinessItemStatus,
} from "@/api/aiInsight";

import type { PageView } from "@/app/navigation";

import {
  getReadinessAction,
  readinessActionLabelKey,
  type ProjectAssetSection,
} from "@/lib/readinessActions";

import { cn } from "@/lib/utils";
import {
  readinessMaxScore,
  readinessScoreTone,
} from "@/lib/readinessConstants";
import { projectScoreLabel } from "@/lib/kanban/projectScores";
import {
  effectiveBadgeTone,
  hasEffectiveScan,
  resolveConfiguredState,
} from "@/lib/readinessEffective";
import { isIndeterminateStatus } from "@/lib/portfolioHealth";
import {
  RepairDriftConfirmDialog,
  type RepairDriftAssetConfirm,
} from "./RepairDriftConfirmDialog";

/**
 * 条目状态的补充说明，覆盖 `agent_readiness.rs:11-19` 的全部 9 个取值。
 * `ready` / `missing` 由 detail 本身表达，不需要额外一行。
 *
 * 缺键就会把 `not_required`、`unmanaged` 这类 wire 值原样渲染给用户
 * （旧实现的三元兜底 `: item.status` 正是如此）。
 */
const STATUS_HINT: Partial<Record<ReadinessItemStatus, string>> = {
  global_only: "来源：全局基线",
  detected_only: "来源：仓库探测",
  partial: "部分 CLI 支持",
  not_required: "当前目标 CLI 不支持此项，已从评分中排除",
  unmanaged: "项目尚未纳入 OpenSunstar，不能判定为缺失",
  unknown: "暂不能判定",
  unhealthy: "配置与预期不一致",
};

export interface AgentReadinessPanelProps {
  data: AgentReadinessResult | null;

  isLoading?: boolean;

  onRefresh?: () => void;

  /** 触发生效态扫描（库 vs 磁盘） */
  onScanEffective?: () => void;

  /** 配置不一致修复（P0-B） */
  onRepairDrift?: (checkName: string) => Promise<void>;

  repairingCheckName?: string | null;

  onOpenProjectAssets?: (section?: ProjectAssetSection) => void;

  onNavigate?: (view: PageView) => void;

  compact?: boolean;
}

export function AgentReadinessPanel({
  data,

  isLoading,

  onRefresh,

  onScanEffective,

  onRepairDrift,

  repairingCheckName = null,

  onOpenProjectAssets,

  onNavigate,

  compact = false,
}: AgentReadinessPanelProps) {
  const { t } = useTranslation();

  const [showCompleted, setShowCompleted] = useState(false);
  const [pendingRepair, setPendingRepair] =
    useState<RepairDriftAssetConfirm | null>(null);

  const { incomplete, complete } = useMemo(() => {
    if (!data) return { incomplete: [], complete: [] };

    const inc: typeof data.details = [];

    const done: typeof data.details = [];

    for (const item of data.details) {
      if (item.score >= item.weight) done.push(item);
      else inc.push(item);
    }

    return { incomplete: inc, complete: done };
  }, [data]);

  if (isLoading && !data) {
    return (
      <div className="flex items-center justify-center py-6 text-muted-foreground text-sm">
        <Loader2 className="w-4 h-4 animate-spin mr-2" />

        {t("kanban.readiness.loading", { defaultValue: "正在评估配置就绪度…" })}
      </div>
    );
  }

  if (!data) return null;

  const maxScore = readinessMaxScore(data.max_score);

  if (compact) {
    const drifted = data.details.filter(
      (item) =>
        item.effective_state === "drifted" || item.status === "unhealthy",
    );
    const normal = data.details.filter(
      (item) =>
        !isIndeterminateStatus(item.status) &&
        item.effective_state !== "drifted" &&
        item.status !== "unhealthy" &&
        item.score >= item.weight,
    );
    const missing = data.details.filter(
      (item) =>
        !isIndeterminateStatus(item.status) &&
        item.effective_state !== "drifted" &&
        item.status !== "unhealthy" &&
        item.score < item.weight,
    );
    const priority = drifted[0] ?? missing[0] ?? null;

    return (
      <div className="rounded-xl border border-border/60 bg-card/40 p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 shrink-0 text-primary" />
              <h3 className="text-sm font-semibold text-foreground">
                {t("kanban.readiness.compactTitle", {
                  defaultValue: "项目配置就绪度",
                })}
              </h3>
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              <span className="font-medium text-emerald-600 dark:text-emerald-400">
                {t("kanban.readiness.normalCount", {
                  count: normal.length,
                  defaultValue: "{{count}} 项正常",
                })}
              </span>
              <span aria-hidden> · </span>
              <span className="font-medium text-amber-600 dark:text-amber-400">
                {t("kanban.readiness.driftCount", {
                  count: drifted.length,
                  defaultValue: "{{count}} 项不一致",
                })}
              </span>
              <span aria-hidden> · </span>
              <span>
                {t("kanban.readiness.missingCount", {
                  count: missing.length,
                  defaultValue: "{{count}} 项缺失",
                })}
              </span>
            </p>
          </div>
          <span
            className={cn(
              "shrink-0 text-lg font-bold tabular-nums",
              readinessScoreTone(data.score, maxScore),
            )}
          >
            {data.score}
            <span className="text-xs font-normal text-muted-foreground">
              /{maxScore}
            </span>
          </span>
        </div>

        {priority && (
          <p className="mt-4 rounded-lg bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
            {drifted.includes(priority)
              ? t("kanban.readiness.priorityDrift", {
                  label: priority.label,
                  defaultValue: "优先处理：{{label}} 配置不一致",
                })
              : t("kanban.readiness.priorityMissing", {
                  label: priority.label,
                  defaultValue: "优先处理：{{label}} 尚未配置",
                })}
          </p>
        )}

        {onOpenProjectAssets && (
          <Button
            variant="outline"
            size="sm"
            className="mt-4 w-full justify-between"
            onClick={() => onOpenProjectAssets()}
          >
            {t("kanban.readiness.openAssetsTab", {
              defaultValue: "打开项目资产配置",
            })}
            <ArrowRight className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>
    );
  }

  const handleAction = (checkName: string, score: number) => {
    const action = getReadinessAction(checkName, score);

    if (!action) return;

    if (action.type === "projectTab") {
      onOpenProjectAssets?.(action.section);

      return;
    }

    onNavigate?.(action.view);
  };

  const renderEffectiveBadges = (item: (typeof data.details)[number]) => {
    const configured = resolveConfiguredState(item);
    const scanned = hasEffectiveScan(item);

    if (!scanned && configured === "unconfigured") {
      return (
        <span className="text-[10px] text-amber-600/90 dark:text-amber-400/90">
          {t("kanban.readiness.effective.unconfigured", {
            defaultValue: "未配置",
          })}
        </span>
      );
    }

    if (!scanned) return null;

    const effTone = effectiveBadgeTone(item.effective_state);

    return (
      <div className="flex flex-wrap items-center gap-1.5 mt-1">
        <span className="inline-flex items-center gap-0.5 text-[10px] text-emerald-600/90 dark:text-emerald-400/90">
          <span aria-hidden>✓</span>
          {t("kanban.readiness.effective.configured", {
            defaultValue: "已配置",
          })}
        </span>
        {effTone === "success" && (
          <span className="inline-flex items-center gap-0.5 text-[10px] text-emerald-600/90 dark:text-emerald-400/90">
            <span aria-hidden>✓</span>
            {t("kanban.readiness.effective.effective", {
              defaultValue: "配置已验证",
            })}
          </span>
        )}
        {effTone === "success" && (
          <span className="text-[10px] text-muted-foreground/70 block w-full">
            {t("kanban.readiness.effective.runtimePending", {
              defaultValue:
                "已验证项目配置一致；目标 CLI 的运行时读取仍待验证。",
            })}
          </span>
        )}
        {effTone === "warning" && (
          <span className="inline-flex items-center gap-0.5 text-[10px] text-amber-600/90 dark:text-amber-400/90">
            <span aria-hidden>⚠</span>
            {t("kanban.readiness.effective.drifted", {
              defaultValue: "配置不一致",
            })}
          </span>
        )}
        {effTone === "muted" && item.effective_state === "unchecked" && (
          <span className="text-[10px] text-muted-foreground/70">
            {t("kanban.readiness.effective.unchecked", {
              defaultValue: "暂未比对",
            })}
          </span>
        )}
        {item.effective_detail && effTone === "warning" && (
          <span className="text-[10px] text-muted-foreground/70 block w-full">
            {item.effective_detail}
          </span>
        )}
        {item.live_path && effTone === "warning" && (
          <span
            className="text-[10px] text-muted-foreground/50 block w-full truncate"
            title={item.live_path}
          >
            {item.live_path}
          </span>
        )}
        {effTone === "warning" && onRepairDrift && (
          <Button
            variant="outline"
            size="sm"
            className="h-6 text-[10px] px-2 mt-1"
            disabled={isLoading || repairingCheckName === item.check_name}
            onClick={() =>
              setPendingRepair({
                kind: "asset",
                checkName: item.check_name,
                label: item.label,
                effectiveDetail: item.effective_detail,
                livePath: item.live_path,
                targetApp: data?.target_app ?? null,
              })
            }
          >
            {repairingCheckName === item.check_name ? (
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
            ) : (
              <Wrench className="h-3 w-3 mr-1" />
            )}
            {t("kanban.readiness.repairDrift", { defaultValue: "修复配置" })}
          </Button>
        )}
      </div>
    );
  };

  const renderItem = (item: (typeof data.details)[number]) => {
    const action = getReadinessAction(item.check_name, item.score);

    const incomplete = item.score < item.weight;

    const done = !incomplete;

    // 不可判定态既不是「已完成」也不是「缺失」，不能用 ✓/✗ 表态。
    // not_required 拿满分（agent_readiness.rs:85-87）打上 ✓ 会谎报已生效；
    // unmanaged/unknown 计零打上 ✗ 会谎报缺失。
    const indeterminate = isIndeterminateStatus(item.status);

    const statusHint = item.status ? STATUS_HINT[item.status] : undefined;

    return (
      <div
        key={item.check_name}
        className={cn(
          "rounded-lg border px-3 py-2",

          done
            ? "border-border/25 bg-background/20 opacity-80"
            : "border-border/40 bg-background/40",
        )}
      >
        <div className="flex items-start gap-2">
          <span
            className={cn(
              "mt-0.5",

              !indeterminate && item.score > 0
                ? "text-emerald-500"
                : "text-muted-foreground/40",
            )}
          >
            {indeterminate ? "—" : item.score > 0 ? "✓" : "✗"}
          </span>

          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium text-foreground/90">
                {item.label}
              </span>

              <span className="text-[10px] text-muted-foreground/60 tabular-nums shrink-0">
                {item.score}/{item.weight}
              </span>
            </div>

            <p className="text-[11px] text-muted-foreground/70 mt-0.5 leading-relaxed">
              {item.detail}

              {statusHint && (
                <span className="block mt-0.5 text-[10px] text-blue-600/80 dark:text-blue-400/80">
                  {t(`kanban.readiness.status.${item.status}`, {
                    defaultValue: statusHint,
                  })}
                </span>
              )}
            </p>

            {renderEffectiveBadges(item)}

            {action && (incomplete || !compact) && (
              <Button
                variant="link"
                size="sm"
                className="h-auto p-0 mt-1 text-[11px] text-primary"
                onClick={() => handleAction(item.check_name, item.score)}
              >
                {t(readinessActionLabelKey(item.check_name, item.score), {
                  defaultValue: item.score > 0 ? "管理" : "去配置",
                })}

                <ArrowRight className="h-3 w-3 ml-0.5" />
              </Button>
            )}
          </div>
        </div>
      </div>
    );
  };

  return (
    <div
      className={
        compact
          ? "rounded-xl border border-border/60 bg-card/40 p-4"
          : "space-y-3"
      }
    >
      <div className="flex items-center justify-between gap-2 mb-3">
        <div className="flex items-center gap-2 min-w-0">
          <Shield className="h-4 w-4 text-primary shrink-0" />

          <h3 className="text-sm font-semibold text-foreground">
            {/* 名字与卡片/清单/矩阵四处同源（审查报告 §5.2） */}
            {projectScoreLabel("agentReadiness", t)}
          </h3>

          {data.target_app && (
            <span className="text-[10px] text-muted-foreground/80 truncate">
              {t("kanban.readiness.forApp", {
                app: data.target_app,
                defaultValue: `按 ${data.target_app} 计分`,
              })}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {data.evaluated_at && (
            <span className="text-[10px] text-muted-foreground/70 hidden sm:inline">
              {new Date(data.evaluated_at * 1000).toLocaleString()}
            </span>
          )}

          {onRefresh && (
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={onRefresh}
              disabled={isLoading}
              aria-label={t("common.refresh", { defaultValue: "刷新" })}
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${isLoading ? "animate-spin" : ""}`}
              />
            </Button>
          )}

          {onScanEffective && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-[10px] px-2"
              onClick={onScanEffective}
              disabled={isLoading}
            >
              <Radar
                className={`h-3 w-3 mr-1 ${isLoading ? "animate-pulse" : ""}`}
              />

              {t("kanban.readiness.scanEffective", {
                defaultValue: "生效态扫描",
              })}
            </Button>
          )}

          <span
            className={cn(
              "text-lg font-bold tabular-nums",

              readinessScoreTone(data.score, maxScore),
            )}
          >
            {data.score}

            <span className="text-xs font-normal text-muted-foreground">
              /{maxScore}
            </span>
          </span>
        </div>
      </div>

      <p className="text-[11px] text-muted-foreground/80 -mt-1 mb-1">
        {t("kanban.readiness.hint", {
          defaultValue:
            "点击下方条目可直达对应配置；项目级资产在「项目资产配置」中关联。",
        })}
      </p>

      <div className="space-y-2">
        {incomplete.map(renderItem)}

        {complete.length > 0 && (
          <div className="pt-1">
            <button
              type="button"
              className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors w-full"
              onClick={() => setShowCompleted((v) => !v)}
            >
              {showCompleted ? (
                <ChevronDown className="h-3.5 w-3.5" />
              ) : (
                <ChevronRight className="h-3.5 w-3.5" />
              )}

              {t("kanban.readiness.completedSection", {
                count: complete.length,

                defaultValue: `已完成 ${complete.length} 项`,
              })}
            </button>

            {showCompleted && (
              <div className="space-y-2 mt-2">{complete.map(renderItem)}</div>
            )}
          </div>
        )}
      </div>

      {data.llm_suggestion && (
        <p className="text-[11px] text-primary/70 leading-relaxed rounded-lg bg-primary/5 px-3 py-2">
          {data.llm_suggestion}
        </p>
      )}

      {onRepairDrift && (
        <RepairDriftConfirmDialog
          pending={pendingRepair}
          onConfirm={() => {
            if (pendingRepair) {
              void onRepairDrift(pendingRepair.checkName);
            }
            setPendingRepair(null);
          }}
          onCancel={() => setPendingRepair(null)}
        />
      )}
    </div>
  );
}
