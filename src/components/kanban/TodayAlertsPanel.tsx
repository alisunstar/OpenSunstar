import { useTranslation } from "react-i18next";
import {
  Bell,
  CheckCircle2,
  Coffee,
  Coins,
  HeartPulse,
  Wrench,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type {
  WorkspaceAlert,
  WorkspaceAlertKind,
} from "@/hooks/useWorkspaceAlerts";

/**
 * 「今日告警」的告警区（工作区重构 2026-07-30）。
 *
 * 它替换掉原来的聚合指标卡与整屏健康清单。原则只有一条：
 * **这一屏只回答「今天有没有事」，没事就一杯咖啡。**
 *
 * - 命（life） 红色：故障转移/熔断 —— 会丢进度的事
 * - 钱（money） 黄色：预算告警     —— 正在烧钱的事
 * - 事（task）  蓝色：配置缺口     —— 可以排期的事
 *
 * 巡视类内容（就绪分、代码行数、阶段分布）全部搬去了「项目看板」。
 */

const KIND_META: Record<
  WorkspaceAlertKind,
  { icon: typeof HeartPulse; stripe: string; iconColor: string }
> = {
  life: {
    icon: HeartPulse,
    stripe: "border-l-red-500",
    iconColor: "text-red-500",
  },
  money: {
    icon: Coins,
    stripe: "border-l-amber-500",
    iconColor: "text-amber-500",
  },
  task: {
    icon: Wrench,
    stripe: "border-l-blue-500",
    iconColor: "text-blue-500",
  },
};

interface TodayAlertsPanelProps {
  alerts: WorkspaceAlert[];
  /** 事件类告警（命/钱）可关闭；task 是派生态，修完自然消失。 */
  onDismiss?: (id: string) => void;
  /** 「查看全部待办」→ 切到项目看板。 */
  onOpenBoard?: () => void;
  className?: string;
}

export function TodayAlertsPanel({
  alerts,
  onDismiss,
  onOpenBoard,
  className,
}: TodayAlertsPanelProps) {
  const { t } = useTranslation();

  if (alerts.length === 0) {
    return (
      <div
        className={cn(
          "rounded-xl border border-border/60 bg-card/30 px-6 py-10 flex flex-col items-center text-center gap-2",
          className,
        )}
      >
        <Coffee className="h-8 w-8 text-muted-foreground/50" />
        <p className="text-base font-semibold text-foreground">
          {t("alerts.empty.title", { defaultValue: "今天没事" })}
        </p>
        <p className="text-xs text-muted-foreground max-w-sm">
          {t("alerts.empty.desc", {
            defaultValue: "没有故障、没有超支、没有待修复。去写代码吧。",
          })}
        </p>
      </div>
    );
  }

  return (
    <div className={cn("space-y-3", className)}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Bell className="h-4 w-4 text-primary" />
          <h3 className="text-sm font-semibold text-foreground">
            {t("alerts.header", {
              count: alerts.length,
              defaultValue: `今天有 ${alerts.length} 件事`,
            })}
          </h3>
        </div>
        {onOpenBoard && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs"
            onClick={onOpenBoard}
          >
            {t("alerts.viewAll", { defaultValue: "查看项目全景" })}
          </Button>
        )}
      </div>

      {alerts.map((alert) => {
        const meta = KIND_META[alert.kind];
        const Icon = meta.icon;
        const dismissible =
          (alert.kind === "life" || alert.kind === "money") && onDismiss;
        return (
          <div
            key={alert.id}
            className={cn(
              "flex items-start gap-3 rounded-xl border border-border/60 bg-card/40 border-l-4 px-4 py-3",
              meta.stripe,
            )}
          >
            <Icon
              className={cn("h-4.5 w-4.5 mt-0.5 shrink-0", meta.iconColor)}
            />
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-foreground">
                {alert.title}
              </p>
              <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                {alert.description}
              </p>
              {alert.action?.hint && (
                <p className="text-[11px] text-muted-foreground/70 mt-1">
                  {alert.action.hint}
                </p>
              )}
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              {alert.action && (
                <Button
                  size="sm"
                  variant={alert.kind === "task" ? "default" : "outline"}
                  className="h-7 text-xs rounded-lg"
                  onClick={alert.action.onClick}
                >
                  {alert.action.label}
                </Button>
              )}
              {dismissible && (
                <button
                  type="button"
                  onClick={() => onDismiss(alert.id)}
                  aria-label={t("alerts.dismiss", { defaultValue: "知道了" })}
                  className="p-1 rounded-md text-muted-foreground/60 hover:text-foreground hover:bg-muted/60 transition-colors"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          </div>
        );
      })}

      {alerts.every((a) => a.kind === "task") && (
        <div className="flex items-center gap-2 px-1 text-[11px] text-muted-foreground/80">
          <CheckCircle2 className="h-3 w-3 text-emerald-500" />
          {t("alerts.onlyTasks", {
            defaultValue: "命和钱都没事——剩下的都是可以排期的事。",
          })}
        </div>
      )}
    </div>
  );
}
