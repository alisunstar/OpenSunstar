import { AlertTriangle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export interface PortfolioDataNoticeProps {
  /** 资产扫描失败的原始原因；null 表示这一轮数据可信 */
  assetError: string | null;
  /** 就绪度读不到的项目数（rejected + API 返回 null 都算） */
  readinessFailedCount: number;
  /** 参与本轮扫描的项目总数，用于给失败数一个分母 */
  totalProjects: number;
  /** 正在刷新时禁用重试，避免叠加请求 */
  refreshing?: boolean;
  /**
   * 外层间距由调用方给，但必须传进来而不是包一层 div：数据正常时本组件返回
   * null，包在外面的带 padding 的 div 会留下一条空白缝。
   */
  className?: string;
  onRetry: () => void;
}

/**
 * 组合层数据不完整告警（审查报告 §5.6）。
 *
 * 三个 Tab 共用一个入口，理由是：`assetMap` 与 `agentReadinessMap` 同时喂
 * 「今日告警 / 项目看板 / AI 资产总览」，读不到时三处都会退化成
 * 「未扫 / 未配置 / 缺 MCP」—— 也就是把网络故障讲成治理结论。
 *
 * 刻意保留原始 error message：全站原本把 429 限流、401 密钥失效、IPC 超时
 * 都折叠成同一句「请稍后重试」，用户无法判断该改配置还是等一等。
 *
 * 数据完整时返回 null —— 它必须是异常态才出现的东西，否则会退化成常驻噪音，
 * 被用户训练成「那条黄的可以无视」。
 */
export function PortfolioDataNotice({
  assetError,
  readinessFailedCount,
  totalProjects,
  refreshing = false,
  className,
  onRetry,
}: PortfolioDataNoticeProps) {
  const { t } = useTranslation();

  // 用 `> 0` 正向判断而不是 `<= 0`：调用方（或过期的测试替身）漏传时
  // `undefined <= 0` 是 false，会让这条告警在一切正常时空着壳子渲染出来。
  const hasReadinessFailure = readinessFailedCount > 0;
  if (!assetError && !hasReadinessFailure) return null;

  return (
    <div
      role="alert"
      className={cn(
        "flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/5 px-3 py-2 text-[11px] text-amber-700 dark:text-amber-400",
        className,
      )}
    >
      <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="font-medium">
          {t("portfolioNotice.title", {
            defaultValue:
              "部分数据没读到，下面的「未扫 / 未配置 / 缺失」不代表磁盘上真的缺失。",
          })}
        </p>
        {hasReadinessFailure && (
          <p className="tabular-nums text-amber-700/80 dark:text-amber-400/80">
            {t("portfolioNotice.readinessFailed", {
              defaultValue: "{{failed}}/{{total}} 个项目的就绪度读取失败",
              failed: readinessFailedCount,
              total: totalProjects,
            })}
          </p>
        )}
        {assetError && (
          <p className="break-words text-amber-700/80 dark:text-amber-400/80">
            {t("portfolioNotice.assetFailed", {
              defaultValue: "资产统计失败：{{reason}}",
              reason: assetError,
            })}
          </p>
        )}
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="h-6 shrink-0 px-2 text-[11px] text-amber-700 hover:text-amber-800 dark:text-amber-400"
        disabled={refreshing}
        onClick={onRetry}
      >
        <RefreshCw
          className={`mr-1 h-3 w-3 ${refreshing ? "animate-spin" : ""}`}
        />
        {t("portfolioNotice.retry", { defaultValue: "重试" })}
      </Button>
    </div>
  );
}
