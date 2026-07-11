import { useEffect, useState } from "react";
import { Coins, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { getAICostSummary, type AICostSummary } from "@/api/aiInsight";
import { useAICost } from "@/contexts/AICostContext";
import { formatAiCostYuan, formatAiTokens } from "@/lib/aiCostFormat";

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
  const { lastCall, refreshToken } = useAICost();
  const [summary, setSummary] = useState<AICostSummary | null>(null);

  useEffect(() => {
    if (!aiConfigured) return;
    getAICostSummary(30).then((s) => {
      if (s) setSummary(s);
    });
  }, [aiConfigured, refreshToken]);

  if (!aiConfigured) return null;

  const monthCost = summary?.total_cost ?? 0;
  const monthTokens = summary?.total_tokens ?? 0;
  const apiCalls = summary?.insight_count ?? 0;
  const nlCount = summary?.nl_query_count ?? 0;

  return (
    <div className="mb-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border border-border/50 bg-card/40 px-3 py-2 text-[11px] text-muted-foreground/70">
      <div className="flex items-center gap-1.5 shrink-0">
        <Coins className="h-3.5 w-3.5 text-primary/60" />
        <span className="font-medium text-foreground/80">AI 成本透明</span>
        <span className="text-muted-foreground/40">·</span>
        <span>{projectCount} 个项目</span>
      </div>

      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <span>
          本月累计{" "}
          <span className="tabular-nums font-medium text-foreground/80">
            {formatAiCostYuan(monthCost)}
          </span>
          {monthTokens > 0 && (
            <span className="tabular-nums text-muted-foreground/50 ml-1">
              ({formatAiTokens(monthTokens)})
            </span>
          )}
        </span>
        {(apiCalls > 0 || nlCount > 0) && (
          <>
            <span className="text-muted-foreground/30">·</span>
            <span className="tabular-nums">
              {apiCalls} 次分析
              {nlCount > 0 ? ` · ${nlCount} 次问答` : ""}
            </span>
          </>
        )}
        {lastCall && (
          <>
            <span className="text-muted-foreground/30">·</span>
            <span>
              本次{" "}
              <span className="tabular-nums font-medium text-primary/90">
                {lastCall.isCached
                  ? "¥0（缓存）"
                  : formatAiCostYuan(lastCall.cost)}
              </span>
            </span>
          </>
        )}
      </div>

      {onOpenRoiPanel && (
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto h-7 text-[11px] text-primary/80 hover:text-primary px-2"
          onClick={onOpenRoiPanel}
        >
          AI 投入报告
          <ChevronRight className="h-3 w-3 ml-0.5" />
        </Button>
      )}
    </div>
  );
}
