import { AlertTriangle, Coins, ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useAICost } from "@/contexts/AICostContext";
import {
  aiCostDisclaimerFull,
  formatAiCostWithPricing,
  formatAiCostYuan,
  formatAiTokens,
} from "@/lib/aiCostFormat";

interface AICostStripProps {
  aiConfigured: boolean;
  projectCount: number;
  onOpenRoiPanel?: () => void;
}

/**
 * 看板常驻成本条：本月累计 + 最近一次 AI 调用消耗。
 */
export function AICostStrip({
  aiConfigured,
  projectCount,
  onOpenRoiPanel,
}: AICostStripProps) {
  /**
   * 数据来自 Provider，不再自己拉。
   *
   * 这个组件和 `AINLQueryBar` 原来各写了一份 `getAICostSummary(30)`，
   * 而两者在多个 Tab 下都挂着，同一份数据要往后端跑两趟（审查报告 §3.1）。
   *
   * `summaryState` 是三态而不是「summary 是否为 null」：API 层统一
   * `catch → return null`，原来的 `summary?.total_cost ?? 0` 把「没查到」和
   * 「加载中」一起渲染成「本月累计 ¥0.00」（审查报告 §5.6）。
   */
  const { t } = useTranslation();
  const { lastCall, summary, summaryState: loadState } = useAICost();

  // 未配置 AI 时整条不渲染：紧邻的 AINLQueryBar 已经承担了「去设置里配置」
  // 的引导，两个空盒子叠在一起只会更吵。
  //
  // 「紧邻」是这条规则成立的前提，别再把它们拆开：这两个组件一度隔着五块面板
  // （成本条在页顶、问答栏在阶段分布之后），于是没配 AI 的用户在页顶看到的是
  // 一片空白，唯一的解释在一屏半之外。调用点见 KanbanPage 的 dashboard 分支。
  if (!aiConfigured) return null;

  const monthCost = summary?.total_cost ?? 0;
  const monthTokens = summary?.total_tokens ?? 0;
  const apiCalls = summary?.insight_count ?? 0;
  const nlCount = summary?.nl_query_count ?? 0;

  return (
    <div className="mb-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border border-border/50 bg-card/40 px-3 py-2 text-[11px] text-muted-foreground/70">
      <div className="flex items-center gap-1.5 shrink-0">
        <Coins className="h-3.5 w-3.5 text-primary/60" />
        <span className="font-medium text-foreground/80">
          {t("ai.cost.stripTitle", { defaultValue: "AI 成本透明" })}
        </span>
        <span className="text-muted-foreground/40">·</span>
        <span>
          {t("ai.cost.projectScope", {
            projects: projectCount,
            defaultValue: "{{projects}} 个项目",
          })}
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        {loadState === "loaded" ? (
          <span>
            {t("ai.cost.monthTotal", { defaultValue: "本月累计" })}{" "}
            <span className="tabular-nums font-medium text-foreground/80">
              {formatAiCostYuan(monthCost)}
            </span>
            {monthTokens > 0 && (
              <span className="tabular-nums text-muted-foreground/50 ml-1">
                ({formatAiTokens(monthTokens)})
              </span>
            )}
          </span>
        ) : loadState === "failed" ? (
          <span className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
            <AlertTriangle className="h-3 w-3" />
            {/* 「未知，不是 ¥0」必须整句一起翻译（§5.6）：拆成「本月累计」+
                「查询失败」两条键，译者很容易只译前半句，而这条文案存在的
                全部理由就是后半句那个括号。 */}
            <span>
              {t("ai.cost.monthTotalFailed", {
                defaultValue: "本月累计查询失败（未知，不是 ¥0）",
              })}
            </span>
          </span>
        ) : (
          <span className="text-muted-foreground/50">
            {t("ai.cost.monthTotalLoading", {
              defaultValue: "本月累计 统计中…",
            })}
          </span>
        )}
        {loadState === "loaded" && (apiCalls > 0 || nlCount > 0) && (
          <>
            <span className="text-muted-foreground/30">·</span>
            <span className="tabular-nums">
              {t("ai.cost.analysisCount", {
                calls: apiCalls,
                defaultValue: "{{calls}} 次分析",
              })}
              {nlCount > 0
                ? ` · ${t("ai.cost.nlQueryCount", {
                    queries: nlCount,
                    defaultValue: "{{queries}} 次问答",
                  })}`
                : ""}
            </span>
          </>
        )}
        {lastCall && (
          <>
            <span className="text-muted-foreground/30">·</span>
            <span>
              {t("ai.cost.lastCall", { defaultValue: "本次" })}{" "}
              <span className="tabular-nums font-medium text-primary/90">
                {lastCall.isCached
                  ? t("ai.cost.cachedFree", { defaultValue: "¥0（缓存）" })
                  : formatAiCostWithPricing(
                      lastCall.cost,
                      lastCall.pricingKnown,
                      t,
                    )}
              </span>
            </span>
          </>
        )}
      </div>

      {/* 免责声明。原来只有 AICostPanel 底部有这句话，而这条常驻在看板顶部
          的成本条才是绝大多数人唯一会看到的金额（审查报告 §5.5）。
          11px 的行放不下整句，因此正文取要点、完整口径挂 title。 */}
      <span
        className="text-[10px] text-muted-foreground/40 shrink-0"
        title={aiCostDisclaimerFull(t)}
      >
        {t("ai.cost.disclaimerShort", {
          defaultValue: "估算 · 以供应商账单为准",
        })}
      </span>

      {onOpenRoiPanel && (
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto h-7 text-[11px] text-primary/80 hover:text-primary px-2"
          onClick={onOpenRoiPanel}
        >
          {t("ai.cost.roiReport", { defaultValue: "AI 投入报告" })}
          <ChevronRight className="h-3 w-3 ml-0.5" />
        </Button>
      )}
    </div>
  );
}
