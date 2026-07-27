import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import {
  aiCostDisclaimer,
  aiCostDisclaimerFull,
  aiCostUnknownLabel,
  formatAiCostWithPricing,
  formatAiCostYuan,
  insightTypeLabel,
} from "./aiCostFormat";

/**
 * 直接回吐 `defaultValue` 的替身 —— 和 `portfolioMatrixData.test.ts` 同一套。
 *
 * 用替身而不是真 i18next：这个文件测的是**格式化规则**（几位小数、什么时候
 * 不出数字、拼接结构），不是翻译内容。挂上真实例只会让「金额被抹成 ¥0.00」
 * 这类断言的失败原因变得含糊。
 */
const t = ((key: string, opts?: { defaultValue?: string }) =>
  opts?.defaultValue ?? key) as unknown as TFunction;

/**
 * 金额格式化曾经有四套规则（审查报告 §5.5）：
 * `aiCostFormat.ts` 2/4 位、`AINLQueryBar` 3/4 位、`AIWeeklyReport` 2/4 位内联。
 * 同一笔钱在成本条、问答栏、周报里长得不一样，用户没法比对。
 */
describe("formatAiCostYuan 统一 2 位小数", () => {
  it("正常金额固定 2 位小数", () => {
    expect(formatAiCostYuan(1)).toBe("¥1.00");
    expect(formatAiCostYuan(0.129)).toBe("¥0.13");
    expect(formatAiCostYuan(12.3456)).toBe("¥12.35");
  });

  it("P0 回归：不足 1 分的开销不得被抹成「¥0.00」", () => {
    // 2 位小数会把 0.003 渲染成「¥0.00」——「AI 没花钱」是假的。
    // 这与第一梯队刚修掉的「查询失败显示 ¥0.00」是同一类谎。
    expect(formatAiCostYuan(0.003)).not.toBe("¥0.00");
    expect(formatAiCostYuan(0.003)).toBe("<¥0.01");
    expect(formatAiCostYuan(0.0001)).toBe("<¥0.01");
  });

  it("真正的零仍然是 ¥0.00", () => {
    expect(formatAiCostYuan(0)).toBe("¥0.00");
    expect(formatAiCostYuan(-1)).toBe("¥0.00");
  });

  it("NaN / Infinity 不得渲染成「¥NaN」", () => {
    expect(formatAiCostYuan(Number.NaN)).toBe("¥0.00");
    expect(formatAiCostYuan(Number.POSITIVE_INFINITY)).toBe("¥0.00");
  });
});

/**
 * 后端 `lookup_model_pricing` 未命中定价表时会给一个兜底价并置
 * `pricing_known: false`。界面此时展示一个精确到分的数字，比不展示更糟。
 */
describe("formatAiCostWithPricing 单价可信度", () => {
  it("单价未知时不出数字", () => {
    expect(formatAiCostWithPricing(1.23, false, t)).toBe(aiCostUnknownLabel(t));
    expect(formatAiCostWithPricing(1.23, false, t)).not.toContain("1.23");
  });

  it("单价已知时照常出数", () => {
    expect(formatAiCostWithPricing(1.23, true, t)).toBe("¥1.23");
  });

  it("字段缺省按已知处理：老后端不返回它，不该被一律打成不可信", () => {
    expect(formatAiCostWithPricing(1.23, undefined, t)).toBe("¥1.23");
  });
});

describe("免责声明是全站唯一来源", () => {
  it("完整版包含要点版，避免两处文案漂移", () => {
    // 这条在**每种语言**下都成立，因为完整版是「要点版 + 后缀」拼出来的，
    // 而不是另一条独立翻译。译者改不歪它。
    expect(aiCostDisclaimerFull(t)).toContain(aiCostDisclaimer(t));
  });

  it("说清了「估算」和「以账单为准」两件事", () => {
    expect(aiCostDisclaimer(t)).toContain("估算");
    expect(aiCostDisclaimer(t)).toContain("账单");
  });
});

/**
 * `insight_type` 是后端枚举，界面上到处在画它（成本面板的类型表、问答栏的
 * 分析明细）。这里守的是「认不出的类型别编标签」。
 */
describe("insightTypeLabel 只翻译认识的类型", () => {
  it("已知类型走翻译", () => {
    expect(insightTypeLabel("risk_analysis", t)).toBe("风险分析");
    expect(insightTypeLabel("agent_readiness", t)).toBe("Agent 就绪度");
  });

  it("P0：后端加了新类型时原样回显，不得伪装成已知类型", () => {
    // 编一个假标签（比如统一回退成「其他」）会把「前端还没跟上」藏起来。
    expect(insightTypeLabel("brand_new_type", t)).toBe("brand_new_type");
  });
});
