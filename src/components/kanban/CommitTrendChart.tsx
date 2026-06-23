import { useId } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { TrendingUp } from "lucide-react";
import { AIFeedbackButtons } from "./AIFeedbackButtons";

interface CommitTrendChartProps {
  /** 最近 12 周每周提交数（从旧到新） */
  weeklyCommits: number[];
  projectName: string;
  /** AI 生成的趋势解读（可选） */
  aiInsight?: string | null;
  /** 项目 ID，用于反馈 */
  projectId?: string;
}

/**
 * 提交趋势图表 — 12 周提交数据的紧凑面积图。
 * 参考 UsageTrendChart.tsx 的 recharts 用法。
 */
export function CommitTrendChart({
  weeklyCommits,
  projectName: _projectName,
  aiInsight,
  projectId,
}: CommitTrendChartProps) {
  if (!weeklyCommits || weeklyCommits.length === 0) {
    return (
      <div className="mt-3 rounded-lg border border-border/50 p-3">
        <div className="flex items-center gap-2 text-xs text-muted-foreground/50">
          <TrendingUp className="h-3.5 w-3.5" />
          <span>暂无提交趋势数据</span>
        </div>
      </div>
    );
  }

  const gradientId = useId();

  // 构建图表数据（从旧到新: W12 → W1）
  const data = weeklyCommits.map((count, i) => ({
    week: `W${weeklyCommits.length - i}`,
    commits: count,
  }));

  const total = weeklyCommits.reduce((s, c) => s + c, 0);

  return (
    <div className="mt-3 rounded-lg border border-border/50 p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <TrendingUp className="h-3.5 w-3.5 text-muted-foreground/50" />
          <span className="text-xs font-medium text-foreground/80">提交趋势</span>
        </div>
        <span className="text-[10px] text-muted-foreground/50 tabular-nums">
          12 周共 {total} 次
        </span>
      </div>

      <div className="h-[120px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart
            data={data}
            margin={{ top: 4, right: 4, left: -20, bottom: 0 }}
          >
            <defs>
              <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3} />
                <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
              </linearGradient>
            </defs>
            <XAxis
              dataKey="week"
              tick={{ fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" }}
              axisLine={false}
              tickLine={false}
            />
            <YAxis
              tick={{ fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" }}
              axisLine={false}
              tickLine={false}
              allowDecimals={false}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "hsl(var(--popover))",
                border: "1px solid hsl(var(--border))",
                borderRadius: "8px",
                fontSize: "11px",
                padding: "6px 10px",
              }}
              labelStyle={{ color: "hsl(var(--foreground))", fontSize: "10px" }}
              formatter={(value: unknown) => [`${value} 次`, "提交"]}
            />
            <Area
              type="monotone"
              dataKey="commits"
              stroke="hsl(var(--primary))"
              strokeWidth={1.5}
              fill={`url(#${gradientId})`}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* AI 趋势解读 */}
      {aiInsight && (
        <div className="mt-2 flex items-start gap-1.5">
          <p className="flex-1 text-[11px] text-muted-foreground/60 leading-relaxed">
            {aiInsight}
          </p>
          {projectId && (
            <AIFeedbackButtons
              projectId={projectId}
              insightType="trend_analysis"
            />
          )}
        </div>
      )}
    </div>
  );
}
