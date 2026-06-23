import { useState } from "react";
import { FileText, Loader2, Copy, Check, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AIFeedbackButtons } from "./AIFeedbackButtons";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { copyText } from "@/lib/clipboard";
import { toast } from "sonner";
import { buildProviderConfig, generateWeeklyReport, type ProjectContextInput, type WeeklyReportResult } from "@/api/aiInsight";
import { PORTFOLIO_COMMIT_WINDOW_DAYS } from "@/lib/portfolioMetrics";
import { useAICostOptional } from "@/contexts/AICostContext";

interface AIWeeklyReportProps {
  projectContexts: ProjectContextInput[];
  aiConfigured: boolean;
}

/**
 * 智能周报 Dialog — 一键生成所有项目的结构化周报。
 */
export function AIWeeklyReport({
  projectContexts,
  aiConfigured,
}: AIWeeklyReportProps) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [report, setReport] = useState<WeeklyReportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const costCtx = useAICostOptional();

  if (!aiConfigured) return null;

  const handleGenerate = async () => {
    setOpen(true);
    setLoading(true);
    setError(null);
    setReport(null);
    setCopied(false);

    const config = buildProviderConfig();
    if (!config) {
      setError("未配置 AI 提供方，请先在设置中配置 AI Key。");
      setLoading(false);
      return;
    }

    const result = await generateWeeklyReport(config, projectContexts);
    if (result) {
      setReport(result);
      costCtx?.recordCall({
        cost: result.cost_estimate,
        tokens: result.tokens_used,
        insightType: "portfolio_summary",
        isCached: result.is_cached,
      });
    } else {
      setError("周报生成失败，请稍后重试。");
    }
    setLoading(false);
  };

  const handleCopy = async () => {
    if (!report?.content) return;
    try {
      await copyText(report.content);
      setCopied(true);
      toast.success("周报已复制到剪贴板");
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error("复制失败");
    }
  };

  return (
    <>
      {/* 触发按钮 */}
      <Button
        variant="outline"
        size="sm"
        onClick={handleGenerate}
        disabled={projectContexts.length === 0}
      >
        <FileText className="w-4 h-4 mr-1" />
        生成周报
        <span className="text-[10px] text-muted-foreground/60 ml-1 tabular-nums">
          ({PORTFOLIO_COMMIT_WINDOW_DAYS}天)
        </span>
      </Button>

      {/* 周报 Dialog */}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-hidden flex flex-col p-0">
          <DialogHeader className="relative pr-12">
            <DialogTitle className="flex items-center gap-2 text-base">
              <FileText className="w-4.5 h-4.5 text-primary" />
              AI 智能周报
              <span className="text-[10px] text-muted-foreground/50 font-normal ml-1">
                · 近 {PORTFOLIO_COMMIT_WINDOW_DAYS} 天数据
              </span>
              {report?.is_cached && (
                <span className="text-[10px] text-muted-foreground/50 font-normal ml-2">
                  (缓存)
                </span>
              )}
            </DialogTitle>
            <DialogClose
              className="absolute right-4 top-1/2 -translate-y-1/2 rounded-sm p-1.5 text-muted-foreground opacity-70 ring-offset-background transition-opacity hover:opacity-100 hover:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
              aria-label="关闭"
            >
              <X className="h-4 w-4" />
            </DialogClose>
          </DialogHeader>

          <div className="flex-1 overflow-y-auto py-3 px-1">
            {loading && (
              <div className="flex flex-col items-center justify-center py-12 gap-3">
                <Loader2 className="w-6 h-6 animate-spin text-primary/60" />
                <p className="text-sm text-muted-foreground/60">
                  正在分析 {projectContexts.length} 个项目数据...
                </p>
              </div>
            )}

            {error && (
              <div className="rounded-lg border border-red-500/20 bg-red-500/5 p-4 text-sm text-red-500/80">
                {error}
              </div>
            )}

            {report && !loading && (
              <div className="prose prose-sm max-w-none text-foreground/80">
                <pre className="whitespace-pre-wrap font-sans text-sm leading-relaxed">
                  {report.content}
                </pre>
              </div>
            )}
          </div>

          {/* 底部操作栏 */}
          {report && !loading && (
            <div className="flex items-center justify-between border-t border-border/50 pt-3 pb-1">
              <div className="flex items-center gap-3 text-[11px] text-muted-foreground/50">
                <span className="tabular-nums">
                  {report.tokens_used > 0
                    ? `${report.tokens_used.toLocaleString()} tokens`
                    : "—"}
                </span>
                {report.cost_estimate > 0 && (
                  <span className="tabular-nums">
                    ¥{report.cost_estimate < 0.01
                      ? report.cost_estimate.toFixed(4)
                      : report.cost_estimate.toFixed(2)}
                  </span>
                )}
                <AIFeedbackButtons
                  projectId="__portfolio__"
                  insightType="portfolio_summary"
                />
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCopy}
                  className="h-8 text-xs"
                >
                  {copied ? (
                    <>
                      <Check className="w-3.5 h-3.5 mr-1 text-emerald-500" />
                      已复制
                    </>
                  ) : (
                    <>
                      <Copy className="w-3.5 h-3.5 mr-1" />
                      复制
                    </>
                  )}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setOpen(false)}
                  className="h-8 text-xs"
                >
                  <X className="w-3.5 h-3.5 mr-1" />
                  关闭
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
