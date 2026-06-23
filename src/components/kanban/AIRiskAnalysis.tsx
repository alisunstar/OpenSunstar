import { AlertTriangle, Shield, Activity, Users, Code2, Clock, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AIFeedbackButtons } from "./AIFeedbackButtons";
import type { AIRiskResult, RiskItem } from "@/api/aiInsight";

interface AIRiskAnalysisProps {
  data: AIRiskResult | null;
  isLoading: boolean;
  onRefresh: () => void;
  /** 是否已加载过（首次需手动触发） */
  hasLoaded: boolean;
  /** 项目 ID，用于反馈 */
  projectId?: string;
}

const riskIcons: Record<string, React.ReactNode> = {
  activity: <Activity className="h-3.5 w-3.5" />,
  concentration: <Users className="h-3.5 w-3.5" />,
  tech_debt: <Code2 className="h-3.5 w-3.5" />,
  schedule: <Clock className="h-3.5 w-3.5" />,
};

const riskLabels: Record<string, string> = {
  activity: "活跃度",
  concentration: "集中度",
  tech_debt: "技术债",
  schedule: "进度",
};

const levelColors: Record<string, string> = {
  high: "text-red-500 bg-red-500/10 border-red-500/20",
  medium: "text-amber-500 bg-amber-500/10 border-amber-500/20",
  low: "text-emerald-500 bg-emerald-500/10 border-emerald-500/20",
};

const levelLabels: Record<string, string> = {
  high: "高风险",
  medium: "中风险",
  low: "低风险",
};

/**
 * 风险分析面板 — 在项目详情抽屉中渲染。
 * 显示总体风险等级、摘要，以及各维度风险卡片。
 */
export function AIRiskAnalysis({
  data,
  isLoading,
  onRefresh,
  hasLoaded,
  projectId,
}: AIRiskAnalysisProps) {
  // ── 未触发首次分析 ──
  if (!hasLoaded && !isLoading) {
    return (
      <div className="mt-4 rounded-lg border border-dashed border-border/50 p-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs text-muted-foreground/60">
            <Shield className="h-3.5 w-3.5" />
            <span>AI 风险分析</span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-[11px] px-2.5"
            onClick={onRefresh}
          >
            <AlertTriangle className="h-3 w-3 mr-1" />
            分析风险
          </Button>
        </div>
      </div>
    );
  }

  // ── 加载中 ──
  if (isLoading) {
    return (
      <div className="mt-4 rounded-lg border border-border/50 p-3">
        <div className="flex items-center gap-2 text-xs text-muted-foreground/60 mb-2">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          <span>正在分析风险...</span>
        </div>
        <div className="space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-12 rounded-md bg-muted/30 animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  // ── 无数据 ──
  if (!data) return null;

  const overallColor = levelColors[data.overall_risk] ?? levelColors.low;

  return (
    <div className="mt-4 rounded-lg border border-border/50 p-3">
      {/* 标题栏 */}
      <div className="flex items-center justify-between mb-2.5">
        <div className="flex items-center gap-2">
          <Shield className="h-3.5 w-3.5 text-muted-foreground/50" />
          <span className="text-xs font-medium text-foreground/80">风险分析</span>
          <span
            className={`inline-flex items-center rounded-full border px-1.5 py-0.5 text-[10px] font-medium ${overallColor}`}
          >
            {levelLabels[data.overall_risk] ?? data.overall_risk}
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 text-[10px] px-2 text-muted-foreground/50"
          onClick={onRefresh}
        >
          刷新
        </Button>
        {projectId && (
          <AIFeedbackButtons
            projectId={projectId}
            insightType="risk_analysis"
            className="ml-1"
          />
        )}
      </div>

      {/* 总体摘要 */}
      <p className="text-[11px] text-muted-foreground/70 mb-2.5">{data.summary}</p>

      {/* 风险项列表 */}
      {data.risks.length > 0 ? (
        <div className="space-y-1.5">
          {data.risks.map((risk, i) => (
            <RiskCard key={i} risk={risk} />
          ))}
        </div>
      ) : (
        <div className="flex items-center gap-2 rounded-md bg-emerald-500/5 p-2.5">
          <Shield className="h-3.5 w-3.5 text-emerald-500/60" />
          <span className="text-[11px] text-emerald-600/80">项目状态健康，暂未发现风险</span>
        </div>
      )}
    </div>
  );
}

function RiskCard({ risk }: { risk: RiskItem }) {
  const color = levelColors[risk.level] ?? levelColors.low;
  const icon = riskIcons[risk.risk_type] ?? riskIcons.activity;
  const label = riskLabels[risk.risk_type] ?? risk.risk_type;

  return (
    <div className="rounded-md border border-border/40 bg-card/50 p-2.5">
      <div className="flex items-center gap-1.5 mb-1">
        <span className="text-muted-foreground/50">{icon}</span>
        <span className="text-[11px] font-medium text-foreground/70">{label}</span>
        <span
          className={`ml-auto inline-flex items-center rounded-full border px-1.5 py-0.5 text-[9px] font-medium ${color}`}
        >
          {levelLabels[risk.level] ?? risk.level}
        </span>
      </div>
      <p className="text-[11px] text-muted-foreground/60 leading-relaxed">{risk.evidence}</p>
      <p className="text-[11px] text-primary/70 mt-1 leading-relaxed">{risk.suggestion}</p>
    </div>
  );
}
