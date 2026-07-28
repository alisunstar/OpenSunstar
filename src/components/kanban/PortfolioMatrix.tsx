import { useMemo } from "react";
import { useTranslation } from "react-i18next";
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
import {
  projectScoreLabel,
  type ProjectScoreKind,
} from "@/lib/kanban/projectScores";

export interface ProjectPoint {
  projectId: string;
  name: string;
  stage: StageKey;
  /** 近 7 天提交数（与看板卡片、周报统一） */
  activity: number;
  /**
   * Y 轴分值 (0-100)。**具体是哪个分数由 `scoreKind` 说了算。**
   *
   * 不再叫 `health`：这个字段现在也可能装的是就绪分，沿用旧名就是审查报告
   * §5.2「一个分数，三个名字」在图表里复发一次。
   */
  score: number;
  /** 代码行数 */
  codeLines: number;
}

interface PortfolioMatrixProps {
  points: ProjectPoint[];
  /**
   * Y 轴画哪个分数。整张图**只有一种**，绝不混画 —— 两个 0-100 分数来源
   * 毫不相干（配置扫描 vs AI 分析），混在一条轴上等于把 §5.2 的错误升级成
   * 一张会误导决策的图。选择逻辑在 `usePortfolioDerivedMetrics`。
   */
  scoreKind: ProjectScoreKind;
}

const stageColors: Record<string, string> = {
  mvp: "hsl(var(--chart-4))", // purple
  rapid: "hsl(var(--chart-2))", // emerald
  stable: "hsl(var(--chart-1))", // blue
};

/** 四象限的横切线。两种分数都是 0-100，60 分作「过半偏上」的分界。 */
const SCORE_THRESHOLD = 60;

type QuadrantKey = "star" | "attention" | "stable" | "dormant";

/**
 * 象限措辞必须跟着 Y 轴的语义走。
 *
 * 「明星项目 / 可能废弃」是对**项目本身**的价值判断，只有当 Y 轴是 AI 健康分
 * （一个确实在评价工程质量的分数）时才站得住。切到就绪分之后，Y 轴衡量的是
 * 「这个项目的 AI 配置落地了多少」—— 一个刚加进来、还没接 Claude 的项目就绪分
 * 接近 0，但它不是「可能废弃」，只是还没配（与 `portfolioHealth.ts:7-9`
 * 「score 是采纳度指标，不是告警阈值」同一条口径）。
 *
 * 沿用同一套字眼，就是把审查报告 §5.6「图上说的事，数据并不支持」原样搬进来。
 */
const QUADRANT_COPY: Record<
  ProjectScoreKind,
  Record<QuadrantKey, { label: string; hint: string }>
> = {
  aiHealth: {
    star: { label: "明星项目", hint: "高活跃 + 高健康" },
    attention: { label: "需关注", hint: "高活跃 + 低健康" },
    stable: { label: "稳定维护", hint: "低活跃 + 高健康" },
    dormant: { label: "可能废弃", hint: "低活跃 + 低健康" },
  },
  agentReadiness: {
    star: { label: "活跃 · 配置齐全", hint: "高活跃 + 高就绪分" },
    attention: { label: "活跃 · 配置待补", hint: "高活跃 + 低就绪分" },
    stable: { label: "低频 · 配置齐全", hint: "低活跃 + 高就绪分" },
    dormant: { label: "低频 · 配置待补", hint: "低活跃 + 低就绪分" },
  },
};

const QUADRANT_CORNERS: Array<{ key: QuadrantKey; corner: string }> = [
  { key: "star", corner: "右上" },
  { key: "attention", corner: "右下" },
  { key: "stable", corner: "左上" },
  { key: "dormant", corner: "左下" },
];

/** 同坐标多点时按黄金角微偏移，避免完全重叠只看到一个点 */
function spreadOverlappingPoints(points: ProjectPoint[]) {
  const slotAt = new Map<string, number>();
  return points.map((p) => {
    const key = `${p.activity}|${p.score}`;
    const slot = slotAt.get(key) ?? 0;
    slotAt.set(key, slot + 1);
    const overlapCount = points.filter(
      (o) => o.activity === p.activity && o.score === p.score,
    ).length;
    let x = p.activity;
    let y = p.score;
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
      rawY: p.score,
      overlapCount,
      z: p.codeLines > 0 ? Math.max(Math.log10(p.codeLines + 1) * 8, 20) : 20,
      name: p.name,
      stage: p.stage,
      fill: stageColors[p.stage] ?? "hsl(var(--muted-foreground))",
    };
  });
}

/**
 * 项目组合矩阵图 — 四象限气泡图。
 * X=活跃度, Y=`scoreKind` 指定的分数, 气泡大小=代码规模。
 *
 * 组件名去掉了 `AI` 前缀（审查报告 §2.5）：这里不调用任何模型，纯 recharts
 * 画本地已有的数字。前缀带来的唯一后果是让人以为它需要 API Key —— 而它此前
 * 确实被 `aiConfigured` 门在外面，没配 Key 的用户连一张纯本地图表都看不到。
 */
export function PortfolioMatrix({ points, scoreKind }: PortfolioMatrixProps) {
  const { t } = useTranslation();
  const scoreLabel = projectScoreLabel(scoreKind, t);
  const activityLabel = "活跃度";

  const chartData = useMemo(() => {
    if (points.length === 0) return [];
    return spreadOverlappingPoints(points);
  }, [points]);

  const hasOverlap = useMemo(
    () => chartData.some((d) => d.overlapCount > 1),
    [chartData],
  );

  // 计算中位数活跃度作为分割线（用原始坐标，不受 jitter 影响）
  const sorted = [...points].sort((a, b) => a.activity - b.activity);
  const midIdx = Math.floor(sorted.length / 2);
  const medianActivity =
    sorted.length > 0
      ? (sorted[midIdx]!.activity +
          (sorted[midIdx - 1]?.activity ?? sorted[midIdx]!.activity)) /
        2
      : 5;

  // 统计四象限
  const quadrants = useMemo(() => {
    const q: Record<QuadrantKey, number> = {
      star: 0,
      attention: 0,
      stable: 0,
      dormant: 0,
    };
    for (const p of points) {
      if (p.activity >= medianActivity && p.score >= SCORE_THRESHOLD) q.star++;
      else if (p.activity >= medianActivity && p.score < SCORE_THRESHOLD)
        q.attention++;
      else if (p.activity < medianActivity && p.score >= SCORE_THRESHOLD)
        q.stable++;
      else q.dormant++;
    }
    return q;
  }, [points, medianActivity]);

  // 提前 return 必须排在所有 Hook 之后（Rules of Hooks）。扫描完成前 points 是
  // 空的，完成后才填充；若在 useMemo 之前 return，空→非空那一帧 Hook 数量从 2
  // 跳到 3，React 直接抛「Rendered more hooks than during the previous render」
  // 把整棵子树打掉。上面的 medianActivity 已用 `sorted.length > 0 ? ... : 5`
  // 兜住空数组，maxActivity 也有兜底常量 4，所以空数据算一遍是安全的。
  if (points.length === 0) return null;

  const copy = QUADRANT_COPY[scoreKind];

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
          <h3 className="text-sm font-semibold text-foreground">
            项目组合矩阵
          </h3>
        </div>
        <div className="flex items-center gap-3 text-[10px] text-muted-foreground/60">
          {QUADRANT_CORNERS.map(({ key }) => (
            <span key={key}>
              {copy[key].label}: {quadrants[key]}
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
              name={activityLabel}
              unit=" 次"
              domain={[0, maxActivity + 5]}
              tick={{ fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" }}
              axisLine={false}
              tickLine={false}
              label={{
                value: "近 7 天提交数",
                position: "insideBottom",
                offset: -2,
                style: {
                  fontSize: 9,
                  fill: "hsl(var(--muted-foreground) / 0.4)",
                },
              }}
            />
            <YAxis
              type="number"
              dataKey="y"
              name={scoreLabel}
              domain={[0, 100]}
              tick={{ fontSize: 9, fill: "hsl(var(--muted-foreground) / 0.4)" }}
              axisLine={false}
              tickLine={false}
              label={{
                value: scoreLabel,
                angle: -90,
                position: "insideLeft",
                offset: 16,
                style: {
                  fontSize: 9,
                  fill: "hsl(var(--muted-foreground) / 0.4)",
                },
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
              formatter={
                ((value: unknown, name?: unknown) => {
                  if (name === activityLabel)
                    return [`${value} 次`, activityLabel];
                  if (name === scoreLabel) return [`${value} 分`, scoreLabel];
                  return [value, name];
                }) as any
              }
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
                    d.overlapCount > 1 ? ` · 同坐标 ${d.overlapCount} 项` : "";
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
              y={SCORE_THRESHOLD}
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
        {QUADRANT_CORNERS.map(({ key, corner }) => (
          <span key={key}>
            {corner}: {copy[key].label}（{copy[key].hint}）
          </span>
        ))}
      </div>
      {hasOverlap && (
        <p className="mt-1.5 text-[10px] text-muted-foreground/45">
          活跃度或{scoreLabel}相同的项目已轻微错开，悬停可查看详情（共{" "}
          {points.length} 个项目）
        </p>
      )}
    </div>
  );
}
