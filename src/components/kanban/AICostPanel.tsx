import { useState } from "react";
import type { ReactNode } from "react";
import {
  BarChart3,
  Coins,
  Loader2,
  Sparkles,
  ThumbsUp,
  AlertTriangle,
  X,
} from "lucide-react";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { useTranslation } from "react-i18next";
import { useAICost } from "@/contexts/AICostContext";
import { useAIRoiReport } from "@/hooks/useAIRoiReport";
import {
  aiCostDisclaimerFull,
  formatAiCostYuan,
  formatAiTokens,
  insightTypeLabel,
} from "@/lib/aiCostFormat";

interface AICostPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const RANGE_OPTIONS = [7, 30, 90] as const;

export function AICostPanel({ open, onOpenChange }: AICostPanelProps) {
  const { t, i18n } = useTranslation();
  const [rangeDays, setRangeDays] =
    useState<(typeof RANGE_OPTIONS)[number]>(30);
  const { refreshToken, bumpRefresh } = useAICost();
  const { report, loading, error } = useAIRoiReport(rangeDays, refreshToken);

  const chartData =
    report?.trends.map((point) => ({
      // 第一个参数原来是 `undefined`，即「跟浏览器语言走」。于是切到 en 之后
      // 整页文案是英文、横轴刻度还是「7月27日」—— 这类不一致比全篇没翻译更
      // 显眼。这里改成跟应用语言走。
      label: new Date(point.bucket_start * 1000).toLocaleDateString(
        i18n.language,
        { month: "short", day: "numeric" },
      ),
      cost: point.cost,
      tokens: point.tokens,
      calls: point.api_calls,
      nl: point.nl_answers,
    })) ?? [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[85vh] overflow-hidden flex flex-col p-0">
        <DialogHeader className="relative pr-12">
          <DialogTitle className="flex items-center gap-2 text-base">
            <BarChart3 className="h-4 w-4 text-primary" />
            {t("ai.cost.panelTitle", { defaultValue: "AI 成本-价值追踪" })}
          </DialogTitle>
          <DialogClose
            className="absolute right-4 top-1/2 -translate-y-1/2 rounded-sm p-1.5 text-muted-foreground opacity-70 ring-offset-background transition-opacity hover:opacity-100 hover:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            aria-label={t("common.close", { defaultValue: "关闭" })}
          >
            <X className="h-4 w-4" />
          </DialogClose>
        </DialogHeader>

        <div className="flex items-center gap-2 px-6 pb-2">
          {RANGE_OPTIONS.map((d) => (
            <Button
              key={d}
              variant={rangeDays === d ? "default" : "outline"}
              size="sm"
              className="h-7 text-xs"
              onClick={() => setRangeDays(d)}
            >
              {/* 变量名不能叫 `count` —— i18next 会把它当复数选择器，转而去找
                  `_one` / `_other` 后缀的键，结果是整条文案回退成裸 key。 */}
              {t("ai.cost.rangeDays", { days: d, defaultValue: "{{days}} 天" })}
            </Button>
          ))}
          <Button
            variant="ghost"
            size="sm"
            className="ml-auto h-7 text-xs"
            onClick={() => bumpRefresh()}
          >
            {t("common.refresh", { defaultValue: "刷新" })}
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto space-y-4 px-6 pb-6 pr-7">
          {loading && (
            <div className="flex items-center justify-center py-16 gap-2 text-muted-foreground">
              <Loader2 className="h-5 w-5 animate-spin" />
              <span className="text-sm">
                {t("ai.cost.loadingRoi", { defaultValue: "加载 ROI 数据…" })}
              </span>
            </div>
          )}

          {error && !loading && (
            <p className="text-sm text-red-500/80 rounded-lg border border-red-500/20 bg-red-500/5 p-3">
              {error}
            </p>
          )}

          {report && !loading && (
            <>
              {/* 叙事 Hero */}
              <div className="rounded-xl border border-primary/15 bg-primary/[0.04] p-4">
                <p className="text-sm text-foreground/85 leading-relaxed">
                  {report.narrative}
                </p>
                <div className="mt-3 flex flex-wrap gap-3 text-[11px]">
                  <StatChip
                    icon={<Coins className="h-3 w-3" />}
                    label={t("ai.cost.statTotalSpend", {
                      defaultValue: "总投入",
                    })}
                    value={formatAiCostYuan(report.totals.cost)}
                  />
                  <StatChip
                    icon={<Sparkles className="h-3 w-3" />}
                    label={t("ai.cost.statTokens", { defaultValue: "Token" })}
                    value={formatAiTokens(report.totals.tokens)}
                  />
                  <StatChip
                    icon={<AlertTriangle className="h-3 w-3" />}
                    label={t("ai.cost.statRisks", { defaultValue: "发现风险" })}
                    value={String(report.totals.risks_found)}
                  />
                  <StatChip
                    icon={<ThumbsUp className="h-3 w-3" />}
                    label={t("ai.cost.statUseful", {
                      defaultValue: "标记有用",
                    })}
                    value={String(report.totals.useful_feedback)}
                  />
                </div>
              </div>

              {/* 趋势图 */}
              {chartData.length > 0 && (
                <div className="rounded-xl border border-border/50 bg-card/30 p-3">
                  <p className="text-xs font-medium text-muted-foreground mb-2">
                    {t("ai.cost.trendTitle", {
                      defaultValue: "每日 AI 投入趋势",
                    })}
                  </p>
                  <ResponsiveContainer width="100%" height={200}>
                    <AreaChart data={chartData}>
                      <CartesianGrid strokeDasharray="3 3" opacity={0.15} />
                      <XAxis dataKey="label" tick={{ fontSize: 10 }} />
                      <YAxis tick={{ fontSize: 10 }} width={40} />
                      <Tooltip
                        contentStyle={{ fontSize: 12 }}
                        formatter={(value, name) => {
                          const n = Number(value ?? 0);
                          // `name` 是 recharts 的 dataKey，不是文案 —— 这几个
                          // 字符串必须保持英文原样，翻译只发生在返回值里。
                          if (name === "cost")
                            return [
                              formatAiCostYuan(n),
                              t("ai.cost.chartCost", { defaultValue: "费用" }),
                            ];
                          if (name === "calls")
                            return [
                              n,
                              t("ai.cost.chartApiCalls", {
                                defaultValue: "API 调用",
                              }),
                            ];
                          return [
                            n,
                            t("ai.cost.chartNlQuery", {
                              defaultValue: "NL 问答",
                            }),
                          ];
                        }}
                      />
                      <Area
                        type="monotone"
                        dataKey="cost"
                        stroke="hsl(var(--primary))"
                        fill="hsl(var(--primary))"
                        fillOpacity={0.15}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              )}

              {/* 按类型 */}
              {report.by_type.length > 0 && (
                <Section
                  title={t("ai.cost.sectionByType", {
                    defaultValue: "按分析类型",
                  })}
                >
                  <div className="overflow-x-auto">
                    <table className="w-full text-[11px]">
                      <thead>
                        <tr className="text-muted-foreground/60 border-b border-border/40">
                          <th className="text-left py-1.5 font-medium">
                            {t("ai.cost.colType", { defaultValue: "类型" })}
                          </th>
                          <th className="text-right py-1.5 font-medium">
                            {t("ai.cost.colCount", { defaultValue: "次数" })}
                          </th>
                          <th className="text-right py-1.5 font-medium">
                            {t("ai.cost.colTokens", { defaultValue: "Token" })}
                          </th>
                          <th className="text-right py-1.5 font-medium">
                            {t("ai.cost.colCost", { defaultValue: "费用" })}
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {report.by_type.map((row) => (
                          <tr
                            key={row.insight_type}
                            className="border-b border-border/20"
                          >
                            <td className="py-1.5">
                              {insightTypeLabel(row.insight_type, t)}
                            </td>
                            <td className="py-1.5 text-right tabular-nums">
                              {row.count}
                            </td>
                            <td className="py-1.5 text-right tabular-nums">
                              {row.total_tokens.toLocaleString()}
                            </td>
                            <td className="py-1.5 text-right tabular-nums">
                              {formatAiCostYuan(row.total_cost)}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </Section>
              )}

              {/* 按项目 — CTO 视角 */}
              {report.by_project.length > 0 && (
                <Section
                  title={t("ai.cost.sectionByProject", {
                    defaultValue: "按项目（消耗 ↔ 价值）",
                  })}
                >
                  <div className="space-y-2">
                    {report.by_project.map((p) => (
                      <div
                        key={p.project_id}
                        className="rounded-lg border border-border/40 bg-muted/20 px-3 py-2"
                      >
                        <div className="flex flex-wrap items-baseline justify-between gap-2">
                          <span className="text-xs font-semibold text-foreground truncate">
                            {p.project_name}
                          </span>
                          <span className="text-[11px] tabular-nums text-muted-foreground">
                            {formatAiCostYuan(p.cost)} ·{" "}
                            {formatAiTokens(p.tokens)}
                          </span>
                        </div>
                        <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] text-muted-foreground/70">
                          {/* 复用成本条那条键：同一个口径（一次 AI 调用算一次
                              分析），译文变了必须两处一起变，这正是复用要的。 */}
                          <span>
                            {t("ai.cost.analysisCount", {
                              calls: p.insight_count,
                              defaultValue: "{{calls}} 次分析",
                            })}
                          </span>
                          {p.risk_count > 0 && (
                            <span className="text-amber-500/90">
                              {t("ai.cost.projectRisks", {
                                risks: p.risk_count,
                                defaultValue: "{{risks}} 项风险",
                              })}
                            </span>
                          )}
                          {p.useful_count > 0 && (
                            <span className="text-emerald-500/90">
                              {t("ai.cost.projectUseful", {
                                useful: p.useful_count,
                                defaultValue: "{{useful}} 次有用",
                              })}
                            </span>
                          )}
                        </div>
                        {p.top_risks.length > 0 && (
                          <ul className="mt-1.5 space-y-0.5 text-[10px] text-muted-foreground/60 list-disc pl-3">
                            {p.top_risks.map((r, i) => (
                              <li key={i} className="truncate">
                                {r}
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    ))}
                  </div>
                </Section>
              )}

              {report.by_type.length === 0 &&
                report.by_project.length === 0 && (
                  <p className="text-sm text-muted-foreground text-center py-8">
                    {t("ai.cost.emptyRoi", {
                      days: rangeDays,
                      defaultValue:
                        "近 {{days}} 天尚无 AI 调用记录。触发摘要、风险分析或 NL 问答后将在此展示 ROI。",
                    })}
                  </p>
                )}

              {/* 这句话曾经只在这里有，现在它是全站共用的函数：
                  成本条、NL 问答栏、周报都指向同一份口径、同一批译文。 */}
              <p className="text-[10px] text-muted-foreground/40 text-center pb-2">
                {aiCostDisclaimerFull(t)}
              </p>
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div>
      <p className="text-xs font-medium text-muted-foreground mb-2">{title}</p>
      {children}
    </div>
  );
}

function StatChip({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <span className="inline-flex items-center gap-1 rounded-md bg-background/60 px-2 py-1 border border-border/30">
      {icon}
      <span className="text-muted-foreground/60">{label}</span>
      <span className="font-medium tabular-nums text-foreground/80">
        {value}
      </span>
    </span>
  );
}
