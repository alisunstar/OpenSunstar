import { useMemo } from "react";
import {
  ScatterChart,
  Scatter,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";
import { Grid3x3 } from "lucide-react";
import type { StageKey } from "@/hooks/useProjectStages";

interface ProjectPoint {
  projectId: string;
  name: string;
  stage: StageKey;
  /** 近 7 天提交数（与看板卡片、周报统一） */
  activity: number;
  /** 健康评分 (0-100) */
  health: number;
  /** 代码行数 */
  codeLines: number;
}

interface AIPortfolioMatrixProps {
  points: ProjectPoint[];
}

const stageColors: Record<string, string> = {
  mvp: "#a855f7",    // purple
  rapid: "#10b981",  // emerald
  stable: "#3b82f6", // blue
};

const quadrantLabels = {
  star: "明星项目",
  attention: "需关注",
  stable: "稳定维护",
  dormant: "可能废弃",
};

/** 同坐标多点时按黄金角微偏移，避免完全重叠只看到一个点 */
function spreadOverlappingPoints(points: ProjectPoint[]) {
  const slotAt = new Map<string, number>();
  return points.map((p) => {
    const key = `${p.activity}|${p.health}`;
    const slot = slotAt.get(key) ?? 0;
    slotAt.set(key, slot + 1);
    const overlapCount = points.filter(
      (o) => o.activity === p.activity && o.health === p.health,
    ).length;
    let x = p.activity;
    let y = p.health;
    if (slot > 0) {
      const angle = slot * 2.399963;
      const radius = 0.6 + slot * 0.55;
      x += Math.cos(angle) * radius;
      y = Math.min(100, Math.max(0, y + Math.sin(angle) * radius * 2.5));
    }
    return {
      x,
      y,
      rawX: p.activity,
      rawY: p.health,
      overlapCount,
      z: p.codeLines > 0 ? Math.max(Math.log10(p.codeLines + 1) * 8, 20) : 20,
      name: p.name,
      stage: p.stage,
      fill: stageColors[p.stage] ?? "#6b7280",
    };
  });
}

/**
 * 项目组合矩阵图 — 四象限气泡图。
 * X=活跃度, Y=健康度, 气泡大小=代码规模。
 */
export function AIPortfolioMatrix({ points }: AIPortfolioMatrixProps) {
  const chartData = useMemo(() => {
    if (points.length === 0) return [];
    return spreadOverlappingPoints(points);
  }, [points]);

  const hasOverlap = useMemo(
    () => chartData.some((d) => d.overlapCount > 1),
    [chartData],
  );

  if (points.length === 0) return null;

  // 计算中位数活跃度作为分割线（用原始坐标，不受 jitter 影响）
  const sorted = [...points].sort((a, b) => a.activity - b.activity);
  const midIdx = Math.floor(sorted.length / 2);
  const medianActivity =
    sorted.length > 0
      ? (sorted[midIdx]!.activity + (sorted[midIdx - 1]?.activity ?? sorted[midIdx]!.activity)) / 2
      : 5;
  const healthThreshold = 60;

  // 统计四象限
  const quadrants = useMemo(() => {
    const q = { star: 0, attention: 0, stable: 0, dormant: 0 };
    for (const p of points) {
      if (p.activity >= medianActivity && p.health >= healthThreshold) q.star++;
      else if (p.activity >= medianActivity && p.health < healthThreshold)
        q.attention++;
      else if (p.activity < medianActivity && p.health >= healthThreshold) q.stable++;
      else q.dormant++;
    }
    return q;
  }, [points, medianActivity]);

  const maxActivity = Math.max(
    ...chartData.map((d) => d.x),
    ...points.map((p) => p.activity),
    4,
  );

  return (
    <div className="rounded-xl border border-border/60 bg-card/30 p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Grid3x3 className="w-4 h-4 text-primary/60" />
          <h3 className="text-sm font-semibold text-foreground">项目组合矩阵</h3>
        </div>
        <div className="flex items-center gap-3 text-[10px] text-muted-foreground/60">
          {Object.entries(quadrants).map(([key, count]) => (
            <span key={key}>
              {quadrantLabels[key as keyof typeof quadrantLabels]}: {count}
            </span>
          ))}
        </div>
      </div>

      {/* 阶段图例 */}
      <div className="flex items-center gap-4 mb-2 text-[10px] text-muted-foreground/70">
        {(["mvp", "rapid", "stable"] as const).map((s) => (
          <span key={s} className="flex items-center gap-1">
            <span
              className="w-2 h-2 rounded-full"
              style={{ backgroundColor: stageColors[s] }}
            />
            {s === "mvp" ? "MVP" : s === "rapid" ? "快速迭代" : "稳定维护"}
          </span>
        ))}
      </div>

      <div className="h-[200px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <ScatterChart margin={{ top: 8, right: 12, bottom: 8, left: -8 }}>
            <XAxis
              type="number"
              dataKey="x"
              name="活跃度"
              unit=" 次"
              domain={[0, maxActivity + 5]}
              tick={{ fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" }}
              axisLine={false}
              tickLine={false}
              label={{
                value: "近 7 天提交数",
                position: "insideBottom",
                offset: -2,
                style: { fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" },
              }}
            />
            <YAxis
              type="number"
              dataKey="y"
              name="健康度"
              domain={[0, 100]}
              tick={{ fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" }}
              axisLine={false}
              tickLine={false}
              label={{
                value: "健康评分",
                angle: -90,
                position: "insideLeft",
                offset: 16,
                style: { fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" },
              }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "hsl(var(--popover))",
                border: "1px solid hsl(var(--border))",
                borderRadius: "8px",
                fontSize: "11px",
                padding: "6px 10px",
              }}
              formatter={((value: unknown, name?: unknown) => {
                if (name === "活跃度") return [`${value} 次`, "活跃度"];
                if (name === "健康度") return [`${value} 分`, "健康度"];
                return [value, name];
              }) as any}
              labelFormatter={(_, payload) => {
                if (payload && payload.length > 0) {
                  const d = payload[0].payload as {
                    name: string;
                    stage: string;
                    rawX: number;
                    rawY: number;
                    overlapCount: number;
                  };
                  const stageLabel =
                    d.stage === "mvp"
                      ? "MVP"
                      : d.stage === "rapid"
                        ? "快速迭代"
                        : "稳定维护";
                  const overlapHint =
                    d.overlapCount > 1
                      ? ` · 同坐标 ${d.overlapCount} 项`
                      : "";
                  return `${d.name} (${stageLabel})${overlapHint}`;
                }
                return "";
              }}
            />
            {/* 四象限分割线 */}
            <ReferenceLine
              x={medianActivity}
              stroke="hsl(var(--border))"
              strokeDasharray="4 4"
              strokeWidth={0.5}
            />
            <ReferenceLine
              y={healthThreshold}
              stroke="hsl(var(--border))"
              strokeDasharray="4 4"
              strokeWidth={0.5}
            />
            <Scatter
              data={chartData}
              shape={(props: any) => {
                const { cx, cy, payload } = props;
                const r = Math.max((props.z ?? 20) / 8, 4);
                return (
                  <circle
                    cx={cx}
                    cy={cy}
                    r={r}
                    fill={payload.fill}
                    fillOpacity={0.75}
                    stroke={payload.fill}
                    strokeOpacity={0.3}
                    strokeWidth={1}
                  />
                );
              }}
            />
          </ScatterChart>
        </ResponsiveContainer>
      </div>

      {/* 象限说明 */}
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 mt-2 text-[10px] text-muted-foreground/50">
        <span>右上: 明星项目（高活跃 + 高健康）</span>
        <span>右下: 需关注（高活跃 + 低健康）</span>
        <span>左上: 稳定维护（低活跃 + 高健康）</span>
        <span>左下: 可能废弃（低活跃 + 低健康）</span>
      </div>
      {hasOverlap && (
        <p className="mt-1.5 text-[10px] text-muted-foreground/45">
          活跃度或健康分相同的项目已轻微错开，悬停可查看详情（共 {points.length}{" "}
          个项目）
        </p>
      )}
    </div>
  );
}
