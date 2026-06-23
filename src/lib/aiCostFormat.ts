/** insight_type → 中文标签 */
export const INSIGHT_TYPE_LABELS: Record<string, string> = {
  summary: "项目摘要",
  health: "健康评分",
  risk_analysis: "风险分析",
  trend_analysis: "趋势分析",
  nl_query: "自然语言查询",
  portfolio_summary: "组合周报",
  progress: "进度估算",
  agent_readiness: "Agent 就绪度",
};

export function insightTypeLabel(type: string): string {
  return INSIGHT_TYPE_LABELS[type] ?? type;
}

export function formatAiCostYuan(cost: number): string {
  if (cost <= 0) return "¥0.00";
  if (cost < 0.01) return `¥${cost.toFixed(4)}`;
  return `¥${cost.toFixed(2)}`;
}

export function formatAiTokens(tokens: number): string {
  if (tokens > 1000) return `${(tokens / 1000).toFixed(1)}K tokens`;
  return `${tokens} tokens`;
}
