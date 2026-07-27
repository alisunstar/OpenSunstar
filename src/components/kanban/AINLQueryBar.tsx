import { useState, useRef, type FormEvent } from "react";
import type { TFunction } from "i18next";
import {
  Sparkles,
  Send,
  Loader2,
  ChevronDown,
  ChevronUp,
  Activity,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { AIFeedbackButtons } from "./AIFeedbackButtons";
import type { ProjectContextInput } from "@/api/aiInsight";
import { useNLQuery } from "@/hooks/useAIInsight";
import { useAICost } from "@/contexts/AICostContext";
import {
  aiCostDisclaimer,
  formatAiCostWithPricing,
  formatAiCostYuan,
  insightTypeLabel,
} from "@/lib/aiCostFormat";

interface AINLQueryBarProps {
  /** 所有项目的上下文数据 */
  projectContexts: ProjectContextInput[];
  /** AI 是否已配置 */
  aiConfigured: boolean;
  /** 项目总数 */
  projectCount: number;
}

/**
 * `insight_type` → **短标签**，专供这一排挤在一行里的胶囊用。
 *
 * 和 `aiCostFormat.insightTypeLabel` 是同一个枚举的两套叫法（「摘要」vs
 * 「项目摘要」）—— 这不是遗漏，是版面：成本面板里那是一列表格，这里是一排
 * 并列胶囊，长名字会换行。所以这个函数返回 `string | undefined`，调用点写成
 * `shortTypeLabel(type, t) ?? insightTypeLabel(type, t)` —— 这张表只负责
 * 「需要缩写的那几个」，其余一律走那边的长名，新增枚举不会在这里漏掉。
 *
 * `project_summary` / `health_score` 是历史别名：现役后端只写
 * `agent_readiness` / `nl_query` / `portfolio_summary`，前端另写 `summary` /
 * `trend_analysis`。这两个键谁都不再产生，但 `ai_usage_log` 是持久表，
 * 老版本写下的行还在库里 —— 删掉它们等于让升级用户的历史统计露出裸枚举名。
 */
function shortTypeLabel(type: string, t: TFunction): string | undefined {
  const labels: Record<string, string> = {
    summary: t("ai.query.typeSummary", { defaultValue: "摘要" }),
    project_summary: t("ai.query.typeSummary", { defaultValue: "摘要" }),
    health: t("ai.query.typeHealth", { defaultValue: "健康" }),
    health_score: t("ai.query.typeHealth", { defaultValue: "健康" }),
    risk_analysis: t("ai.query.typeRisk", { defaultValue: "风险" }),
    trend_analysis: t("ai.query.typeTrend", { defaultValue: "趋势" }),
    nl_query: t("ai.query.typeNlQuery", { defaultValue: "问答" }),
    portfolio_summary: t("ai.query.typeWeekly", { defaultValue: "周报" }),
  };
  return labels[type];
}

/**
 * AI 智能助手 — 一体化模块。
 * 融合成本统计 + 自然语言提问，让用户感知 AI 深度嵌入看板。
 */
export function AINLQueryBar({
  projectContexts,
  aiConfigured,
  projectCount,
}: AINLQueryBarProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState(true);
  const inputRef = useRef<HTMLInputElement>(null);
  /**
   * 与 `AICostStrip` 共用 Provider 里那一份数据。
   *
   * 原来这里自己写了一份 `getAICostSummary(30)`，和成本条各拉各的
   * （审查报告 §3.1）。`costLoadState` 仍是三态：失败不得渲染成 ¥0（§5.6）。
   */
  const {
    summary: costSummary,
    summaryState: costLoadState,
    summaryRangeDays,
  } = useAICost();
  const {
    answer,
    isLoading,
    error,
    costEstimate,
    pricingKnown,
    queryLogId,
    ask,
  } = useNLQuery();

  // 原来是裸 `return null`：整个模块凭空消失，用户不知道自己错过了什么
  // （审查报告 §5.6）。这里给出可行动的一句话，而不是留白。
  if (!aiConfigured) {
    return (
      <div className="mb-3 flex items-center gap-2 rounded-xl border border-dashed border-border/60 bg-card/20 px-4 py-2.5 text-[11px] text-muted-foreground/70">
        <Sparkles className="h-3.5 w-3.5 shrink-0 text-muted-foreground/40" />
        <span>
          {t("ai.query.notConfigured", {
            defaultValue:
              "AI 问答与成本统计尚未启用 —— 在「设置 → AI 提供方」填入 API Key 后，这里会出现自然语言提问入口。",
          })}
        </span>
      </div>
    );
  }

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!query.trim() || isLoading) return;
    ask(query.trim(), projectContexts);
  };

  const hasResult = answer !== null || error !== null;
  const cost = costSummary?.total_cost ?? 0;
  const analysisCount = costSummary?.insight_count ?? 0;
  const byType = costSummary?.by_type ?? {};
  const activeTypes = Object.entries(byType).filter(([, v]) => v > 0);

  return (
    <div className="mb-3 rounded-xl border border-primary/15 bg-card/40 glass-card overflow-hidden">
      {/* ── 头部：AI 守护状态 + 成本统计 ───────────── */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border/30">
        <div className="flex items-center gap-2 shrink-0">
          <div className="relative">
            <Sparkles className="h-4 w-4 text-primary" />
            <span className="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full bg-emerald-400 ring-2 ring-background animate-pulse" />
          </div>
          <span className="text-xs font-semibold text-foreground">
            {t("ai.query.guarding", { defaultValue: "AI 正在守护" })}
          </span>
          <span className="text-xs font-bold text-primary tabular-nums">
            {projectCount}
          </span>
          {/* 数字单独一个 <span> 是为了给它 tabular-nums，所以量词只能拆出来。
              这在中文里天然成立，英文得是「projects」这样的独立词 —— 翻译时
              别把它并回上一句。 */}
          <span className="text-xs text-foreground/60">
            {t("ai.query.projectsUnit", { defaultValue: "个项目" })}
          </span>
        </div>

        {costLoadState === "failed" && (
          <div className="ml-auto flex items-center gap-1 text-[10px] text-amber-600 dark:text-amber-400">
            <Activity className="h-2.5 w-2.5" />
            <span>
              {t("ai.query.costFailed", {
                defaultValue: "成本统计查询失败（未知，不是 ¥0）",
              })}
            </span>
          </div>
        )}

        {costLoadState === "loaded" &&
          (analysisCount > 0 || (costSummary?.nl_query_count ?? 0) > 0) && (
            <div className="flex items-center gap-1.5 ml-auto text-[10px] text-muted-foreground/60">
              <span className="inline-flex items-center gap-1 rounded-md bg-muted/40 px-1.5 py-0.5">
                <Activity className="h-2.5 w-2.5" />
                <span className="tabular-nums">
                  {t("ai.cost.analysisCount", {
                    calls: analysisCount,
                    defaultValue: "{{calls}} 次分析",
                  })}
                </span>
              </span>
              {(costSummary?.nl_query_count ?? 0) > 0 && (
                <span className="inline-flex items-center rounded-md bg-muted/40 px-1.5 py-0.5 tabular-nums">
                  {t("ai.cost.nlQueryCount", {
                    queries: costSummary?.nl_query_count ?? 0,
                    defaultValue: "{{queries}} 次问答",
                  })}
                </span>
              )}
              <span className="inline-flex items-center rounded-md bg-muted/40 px-1.5 py-0.5 tabular-nums">
                {formatAiCostYuan(cost)}
              </span>
            </div>
          )}
      </div>

      {/* ── 输入栏 ────────────────────────────── */}
      <form
        onSubmit={handleSubmit}
        className="flex items-center gap-2.5 px-4 py-3"
      >
        <Sparkles className="h-4 w-4 text-primary/60 shrink-0" />
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("ai.query.placeholder", {
            defaultValue:
              "向 AI 提问，如：哪个项目最近不活跃？帮我生成本周总结",
          })}
          aria-label={t("ai.query.inputLabel", { defaultValue: "向 AI 提问" })}
          className="flex-1 min-w-0 bg-transparent text-sm text-foreground placeholder:text-muted-foreground/40 outline-none"
          disabled={isLoading}
        />
        <Button
          type="submit"
          variant="default"
          size="sm"
          className="h-8 px-3 shrink-0"
          disabled={!query.trim() || isLoading}
        >
          {isLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Send className="h-4 w-4" />
          )}
        </Button>
      </form>

      {/* ── AI 分析明细（可折叠）─────────────────── */}
      {activeTypes.length > 0 && (
        <div className="px-4 pb-2">
          <button
            type="button"
            className="flex items-center gap-1.5 text-[10px] text-muted-foreground/50 hover:text-muted-foreground/80 transition-colors"
            onClick={() => setExpanded(!expanded)}
            aria-expanded={expanded}
            /*
             * 这里原来写的是 `nl-query-result` —— 全 DOM 里没有这个 id（审查
             * 报告 §7）。被它控制的那块是下面的 `nl-query-detail`。
             *
             * 断了的 `aria-controls` 比没有更糟：读屏器会宣告「此按钮控制某个
             * 区域」，然后跳过去找不到，用户以为自己漏了什么。而这类错误
             * 在视觉上完全没有症状，所以它活到了现在。
             */
            aria-controls="nl-query-detail"
          >
            {expanded ? (
              <ChevronUp className="h-3 w-3" />
            ) : (
              <ChevronDown className="h-3 w-3" />
            )}
            <span>
              {t("ai.query.detailToggle", {
                days: costSummary?.period_days ?? summaryRangeDays,
                defaultValue: "近 {{days}} 天分析明细",
              })}
            </span>
          </button>
          {expanded && (
            <div
              id="nl-query-detail"
              role="region"
              className="flex flex-wrap gap-x-3 gap-y-1 mt-1.5"
            >
              {activeTypes.map(([type, cnt]) => (
                <span
                  key={type}
                  className="inline-flex items-center gap-1 text-[10px] text-muted-foreground/60"
                >
                  <span className="h-1.5 w-1.5 rounded-full bg-primary/40" />
                  {shortTypeLabel(type, t) ?? insightTypeLabel(type, t)}
                  <span className="font-medium tabular-nums text-foreground/50">
                    {cnt}
                  </span>
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── 回答结果 ──────────────────────────── */}
      {hasResult && (
        <div className="border-t border-border/30 px-4 py-3 bg-primary/[0.02]">
          <div className="flex items-center gap-1.5 mb-2">
            <Sparkles className="h-3 w-3 text-primary/60" />
            <span className="text-[11px] font-medium text-foreground/70">
              {t("ai.query.answerTitle", { defaultValue: "AI 回答" })}
            </span>
            {costEstimate > 0 && (
              // 曾经这里是全站第三套小数规则（3 位），和成本条的 2 位、
              // 周报的 4 位各说各话。统一走 formatAiCostWithPricing，
              // 并挂上口径 —— 一个不带说明的精确数字最容易被当成账单。
              <span
                className="ml-auto text-[10px] text-muted-foreground/40 tabular-nums"
                title={aiCostDisclaimer(t)}
              >
                {formatAiCostWithPricing(costEstimate, pricingKnown, t)}{" "}
                {t("ai.cost.estimatedSuffix", { defaultValue: "估算" })}
              </span>
            )}
          </div>
          {error ? (
            <p className="text-[12px] text-red-500/80 leading-relaxed">
              {error}
            </p>
          ) : answer ? (
            <div className="flex items-start gap-2">
              <p className="flex-1 text-[12px] text-foreground/80 leading-relaxed whitespace-pre-wrap">
                {answer}
              </p>
              <AIFeedbackButtons queryLogId={queryLogId ?? undefined} />
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}
