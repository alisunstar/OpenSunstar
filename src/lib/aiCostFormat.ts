import type { TFunction } from "i18next";

/**
 * 这个模块里的文案曾经是一组模块级中文常量（`INSIGHT_TYPE_LABELS`、
 * `AI_COST_DISCLAIMER*`、`AI_COST_UNKNOWN_LABEL`）。它们被 6 个 `AI*` 组件
 * 引用，于是切到 en/ja 之后界面上依然是中文（审查报告 §7）。
 *
 * 改成收 `TFunction` 而不是在模块顶层 `import i18n` 取全局单例 ——
 * 后者会让这个纯函数模块在测试里必须先初始化整个 i18next，也会让「文案在
 * 哪一刻被解析」变得不可见。`repairFeedback.ts` 已经是这个约定（`t` 放最后
 * 一个形参），这里跟着走。
 */

/** insight_type → 可读标签。后端给的是稳定枚举，翻译只发生在这一层。 */
export function insightTypeLabel(type: string, t: TFunction): string {
  const labels: Record<string, string> = {
    summary: t("ai.insightType.summary", { defaultValue: "项目摘要" }),
    health: t("ai.insightType.health", { defaultValue: "健康评分" }),
    risk_analysis: t("ai.insightType.riskAnalysis", {
      defaultValue: "风险分析",
    }),
    trend_analysis: t("ai.insightType.trendAnalysis", {
      defaultValue: "趋势分析",
    }),
    nl_query: t("ai.insightType.nlQuery", { defaultValue: "自然语言查询" }),
    portfolio_summary: t("ai.insightType.portfolioSummary", {
      defaultValue: "组合周报",
    }),
    progress: t("ai.insightType.progress", { defaultValue: "进度估算" }),
    agent_readiness: t("ai.insightType.agentReadiness", {
      defaultValue: "Agent 就绪度",
    }),
  };
  // 认不出的类型原样回显：编一个假标签会把「后端加了新类型」伪装成正常。
  return labels[type] ?? type;
}

/**
 * 全站统一的费用免责声明（审查报告 §5.5）。
 *
 * 原来只有 `AICostPanel` 底部有这句话，而金额在成本条、NL 问答、周报里
 * 到处都是 —— 三处不带口径的精确数字，比一处带口径的更容易被当成账单。
 */
export function aiCostDisclaimer(t: TFunction): string {
  return t("ai.cost.disclaimer", {
    defaultValue: "费用为基于公开定价的估算，以供应商账单为准。",
  });
}

/**
 * 面板级（版面允许时）附加 CLI 代理用量的去处。
 *
 * 刻意用「要点版 + 后缀」拼接，而不是给完整版单独一条翻译：
 * 「完整版必须包含要点版」这条约束因此是**结构成立**的，在每种语言里都成立，
 * 不依赖译者恰好把前半句一字不差地抄进去。`aiCostFormat.test.ts` 守着它。
 */
export function aiCostDisclaimerFull(t: TFunction): string {
  return (
    aiCostDisclaimer(t) +
    t("ai.cost.disclaimerCliSuffix", {
      defaultValue: "CLI 代理用量见「设置 → 用量」。",
    })
  );
}

/** 单价未知时的占位。不给数字 —— 那个数字是猜的。 */
export function aiCostUnknownLabel(t: TFunction): string {
  return t("ai.cost.unknownPricing", { defaultValue: "单价未知" });
}

/**
 * 金额的唯一格式化入口：统一 2 位小数。
 *
 * 全站曾有四套小数规则（2 位 / 3 位 / 4 位 / 混合），同一笔钱在成本条和
 * 问答栏里长得不一样。收敛到这里之后唯一的例外是不足 1 分的开销：直接
 * `toFixed(2)` 会把它抹成「¥0.00」，那和「查询失败显示 ¥0.00」是同一类谎，
 * 只是成因不同。所以低于 1 分显式渲染成 `<¥0.01`。
 */
export function formatAiCostYuan(cost: number): string {
  if (!Number.isFinite(cost) || cost <= 0) return "¥0.00";
  if (cost < 0.01) return "<¥0.01";
  return `¥${cost.toFixed(2)}`;
}

/**
 * 带单价可信度的金额。
 *
 * `pricingKnown === false` 表示后端没在定价表里认出这个模型，金额是兜底猜的
 * （`ai/client.rs::lookup_model_pricing`）—— 此时展示一个精确到分的数字比不
 * 展示更糟。缺省（undefined）按已知处理：老后端不返回这个字段，不该因此把
 * 历史数据一律打成不可信。
 */
export function formatAiCostWithPricing(
  cost: number,
  pricingKnown: boolean | undefined,
  t: TFunction,
): string {
  if (pricingKnown === false) return aiCostUnknownLabel(t);
  return formatAiCostYuan(cost);
}

export function formatAiTokens(tokens: number): string {
  if (tokens > 1000) return `${(tokens / 1000).toFixed(1)}K tokens`;
  return `${tokens} tokens`;
}
