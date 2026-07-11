import { useMemo, useState } from "react";

import { useTranslation } from "react-i18next";

import { ArrowRight, ChevronDown, ChevronRight, Loader2, Radar, RefreshCw, Shield, Wrench } from "lucide-react";

import { Button } from "@/components/ui/button";

import type { AgentReadinessResult } from "@/api/aiInsight";

import type { PageView } from "@/App";

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
import {
  effectiveBadgeTone,
  hasEffectiveScan,
  resolveConfiguredState,
} from "@/lib/readinessEffective";
import {
  RepairDriftConfirmDialog,
  type RepairDriftAssetConfirm,
} from "./RepairDriftConfirmDialog";



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
  const [pendingRepair, setPendingRepair] = useState<RepairDriftAssetConfirm | null>(
    null,
  );



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
              defaultValue: "已生效",
            })}
          </span>
        )}
        {effTone === "warning" && (
          <span className="inline-flex items-center gap-0.5 text-[10px] text-amber-600/90 dark:text-amber-400/90">
            <span aria-hidden>⚠</span>
            {t("kanban.readiness.effective.drifted", {
              defaultValue: "未生效",
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
          <span className="text-[10px] text-muted-foreground/50 block w-full truncate" title={item.live_path}>
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

              item.score > 0

                ? "text-emerald-500"

                : "text-muted-foreground/40",

            )}

          >

            {item.score > 0 ? "✓" : "✗"}

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

              {item.status &&
                item.status !== "ready" &&
                item.status !== "missing" && (
                  <span className="block mt-0.5 text-[10px] text-blue-600/80 dark:text-blue-400/80">
                    {t(`kanban.readiness.status.${item.status}`, {
                      defaultValue:
                        item.status === "global_only"
                          ? "来源：全局基线"
                          : item.status === "detected_only"
                            ? "来源：仓库探测"
                            : item.status === "partial"
                              ? "部分 CLI 支持"
                              : item.status,
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

            {t("kanban.readiness.title", { defaultValue: "Agent 配置就绪" })}

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

              <Radar className={`h-3 w-3 mr-1 ${isLoading ? "animate-pulse" : ""}`} />

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



      {!compact && (

        <p className="text-[11px] text-muted-foreground/80 -mt-1 mb-1">

          {t("kanban.readiness.hint", {

            defaultValue:

              "点击下方条目可直达对应配置；项目级资产在本页「AI 资产配置」中关联。",

          })}

        </p>

      )}



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



      {compact && onOpenProjectAssets && (

        <Button

          variant="outline"

          size="sm"

          className="w-full mt-2"

          onClick={() => onOpenProjectAssets()}

        >

          {t("kanban.readiness.openAssetsTab", {

            defaultValue: "打开 AI 资产配置",

          })}

        </Button>

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
