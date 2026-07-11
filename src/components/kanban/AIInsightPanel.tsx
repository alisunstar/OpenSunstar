import { useState, useEffect } from "react";
import { Sparkles, ChevronDown } from "lucide-react";
import {
  getAICostSummary,
  type AICostSummary,
} from "@/api/aiInsight";

interface AIInsightPanelProps {
  projectCount: number;
  aiConfigured: boolean;
}

/** insight_type → 中文标签 */
const TYPE_LABELS: Record<string, string> = {
  project_summary: "项目摘要",
  health_score: "健康评分",
  risk_analysis: "风险分析",
  trend_analysis: "趋势分析",
  nl_query: "自然语言查询",
  portfolio_summary: "组合周报",
};

function typeLabel(t: string): string {
  return TYPE_LABELS[t] ?? t;
}

/**
 * 看板顶部 AI 面板：
 * - AI 已配置: 显示本月成本统计（可展开查看按类型分组的详情）
 * - AI 未配置: 显示引导卡片
 */
export function AIInsightPanel({
  projectCount,
  aiConfigured,
}: AIInsightPanelProps) {
  const [costSummary, setCostSummary] = useState<AICostSummary | null>(null);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    if (!aiConfigured) return;
    getAICostSummary(30).then((s) => {
      if (s) setCostSummary(s);
    });
  }, [aiConfigured]);

  if (!aiConfigured) return null;

  // ── 已配置 AI: 成本统计条 + 可展开详情 ─────────────────────
  const cost = costSummary?.total_cost ?? 0;
  const tokens = costSummary?.total_tokens ?? 0;
  const count = costSummary?.insight_count ?? 0;
  const byType = costSummary?.by_type ?? {};
  const typeEntries = Object.entries(byType).filter(([, v]) => v > 0);

  return (
    <div className="mb-3">
      {/* 主统计条 */}
      <button
        type="button"
        className={`flex w-full items-center gap-2 text-[11px] text-muted-foreground/60 text-left ${
          typeEntries.length > 0 ? "cursor-pointer select-none" : "cursor-default"
        }`}
        onClick={() => typeEntries.length > 0 && setExpanded((v) => !v)}
        aria-expanded={expanded}
      >
        <Sparkles className="h-3.5 w-3.5 text-primary/50" />
        <span>AI 已启用 · {projectCount} 个项目</span>
        {count > 0 && (
          <>
            <span className="text-muted-foreground/30">·</span>
            <span>本月 {count} 次分析</span>
            <span className="text-muted-foreground/30">·</span>
            <span className="tabular-nums">
              {tokens > 1000
                ? `${(tokens / 1000).toFixed(1)}K tokens`
                : `${tokens} tokens`}
            </span>
            <span className="text-muted-foreground/30">·</span>
            <span className="tabular-nums">
              ¥{cost < 0.01 ? cost.toFixed(4) : cost.toFixed(2)}
            </span>
          </>
        )}
        {typeEntries.length > 0 && (
          <ChevronDown
            className={`ml-auto h-3 w-3 transition-transform duration-200 ${
              expanded ? "rotate-180" : ""
            }`}
          />
        )}
      </button>

      {/* 展开：按类型分组详情 */}
      <div
        className={`overflow-hidden transition-all duration-200 ${
          expanded ? "max-h-40 opacity-100 mt-2" : "max-h-0 opacity-0"
        }`}
      >
        <div className="rounded-lg border border-border/40 bg-card/50 px-3 py-2">
          <p className="text-[10px] font-medium text-muted-foreground/50 mb-1.5">
            近 {costSummary?.period_days ?? 30} 天分析明细
          </p>
          <div className="flex flex-wrap gap-x-4 gap-y-1">
            {typeEntries.map(([type, cnt]) => (
              <div
                key={type}
                className="flex items-center gap-1.5 text-[11px] text-muted-foreground/70"
              >
                <span className="inline-block h-1.5 w-1.5 rounded-full bg-primary/40" />
                <span>{typeLabel(type)}</span>
                <span className="tabular-nums font-medium text-foreground/60">
                  {cnt} 次
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
