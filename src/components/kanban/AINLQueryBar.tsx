import { useState, useRef, useEffect, type FormEvent } from "react";
import {
  Sparkles,
  Send,
  Loader2,
  ChevronDown,
  ChevronUp,
  Activity,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { AIFeedbackButtons } from "./AIFeedbackButtons";
import type { ProjectContextInput } from "@/api/aiInsight";
import { getAICostSummary, type AICostSummary } from "@/api/aiInsight";
import { useNLQuery } from "@/hooks/useAIInsight";
import { useAICost } from "@/contexts/AICostContext";
import { insightTypeLabel, formatAiCostYuan } from "@/lib/aiCostFormat";

interface AINLQueryBarProps {
  /** 所有项目的上下文数据 */
  projectContexts: ProjectContextInput[];
  /** AI 是否已配置 */
  aiConfigured: boolean;
  /** 项目总数 */
  projectCount: number;
}

/** insight_type → 中文标签（兼容旧键名） */
const TYPE_LABELS: Record<string, string> = {
  summary: "摘要",
  project_summary: "摘要",
  health: "健康",
  health_score: "健康",
  risk_analysis: "风险",
  trend_analysis: "趋势",
  nl_query: "问答",
  portfolio_summary: "周报",
};

/**
 * AI 智能助手 — 一体化模块。
 * 融合成本统计 + 自然语言提问，让用户感知 AI 深度嵌入看板。
 */
export function AINLQueryBar({
  projectContexts,
  aiConfigured,
  projectCount,
}: AINLQueryBarProps) {
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState(true);
  const [costSummary, setCostSummary] = useState<AICostSummary | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const { refreshToken } = useAICost();
  const { answer, isLoading, error, costEstimate, queryLogId, ask } = useNLQuery();

  useEffect(() => {
    if (!aiConfigured) return;
    getAICostSummary(30).then((s) => {
      if (s) setCostSummary(s);
    });
  }, [aiConfigured, refreshToken]);

  if (!aiConfigured) return null;

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
            AI 正在守护
          </span>
          <span className="text-xs font-bold text-primary tabular-nums">
            {projectCount}
          </span>
          <span className="text-xs text-foreground/60">个项目</span>
        </div>

        {(analysisCount > 0 || (costSummary?.nl_query_count ?? 0) > 0) && (
          <div className="flex items-center gap-1.5 ml-auto text-[10px] text-muted-foreground/60">
            <span className="inline-flex items-center gap-1 rounded-md bg-muted/40 px-1.5 py-0.5">
              <Activity className="h-2.5 w-2.5" />
              <span className="tabular-nums">{analysisCount} 次分析</span>
            </span>
            {(costSummary?.nl_query_count ?? 0) > 0 && (
              <span className="inline-flex items-center rounded-md bg-muted/40 px-1.5 py-0.5 tabular-nums">
                {costSummary?.nl_query_count} 次问答
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
          placeholder="向 AI 提问，如：哪个项目最近不活跃？帮我生成本周总结"
          aria-label="向 AI 提问"
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
            aria-controls="nl-query-result"
          >
            {expanded ? (
              <ChevronUp className="h-3 w-3" />
            ) : (
              <ChevronDown className="h-3 w-3" />
            )}
            <span>
              近 {costSummary?.period_days ?? 30} 天分析明细
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
                  {TYPE_LABELS[type] ?? insightTypeLabel(type)}
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
              AI 回答
            </span>
            {costEstimate > 0 && (
              <span className="ml-auto text-[10px] text-muted-foreground/40 tabular-nums">
                ¥{costEstimate < 0.001 ? costEstimate.toFixed(4) : costEstimate.toFixed(3)}
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
