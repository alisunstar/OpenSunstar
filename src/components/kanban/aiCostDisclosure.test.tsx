import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { LastAICall } from "@/contexts/AICostContext";
import { AICostStrip } from "@/components/kanban/AICostStrip";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 成本口径回归（审查报告 §5.5）。
 *
 * 两件事在这里守住：
 *
 * 1. **免责声明不能只挂在 `AICostPanel` 底部。** 那个面板要点开才看得到，
 *    而这条常驻在看板顶部的成本条才是绝大多数人唯一会看到的金额。
 * 2. **单价未知时不许出精确数字。** 后端 `lookup_model_pricing` 认不出模型时
 *    会给一个兜底价并置 `pricing_known: false` —— 把猜出来的数字印成
 *    「¥1.23」，比什么都不显示更糟。
 */

const SUMMARY = {
  total_cost: 1.5,
  total_tokens: 12000,
  insight_count: 3,
  nl_query_count: 1,
  period_days: 30,
  by_type: {},
};

let lastCall: LastAICall | null = null;

/**
 * 这里仍然 mock context —— 需要逐例注入 `lastCall`，而真 Provider 只能靠
 * `recordCall` 间接写入。汇总数据从 `getAICostSummary` 搬进 Provider 之后
 * （§3.1），mock 也得跟着补上 `summary` / `summaryState`。
 */
vi.mock("@/contexts/AICostContext", () => ({
  useAICost: () => ({
    lastCall,
    refreshToken: 0,
    recordCall: vi.fn(),
    bumpRefresh: vi.fn(),
    summary: SUMMARY,
    summaryState: "loaded" as const,
    summaryRangeDays: 30,
  }),
  useAICostOptional: () => null,
}));

function call(partial: Partial<LastAICall> = {}): LastAICall {
  return {
    cost: 1.23,
    tokens: 500,
    insightType: "summary",
    isCached: false,
    at: Date.now(),
    ...partial,
  };
}

beforeEach(() => {
  lastCall = null;
});

describe("AICostStrip 免责声明", () => {
  it("常驻成本条必须自带「估算 / 以账单为准」的口径", async () => {
    renderWithProviders(<AICostStrip aiConfigured projectCount={3} />);

    await waitFor(() =>
      expect(screen.getByText(/账单为准/)).toBeInTheDocument(),
    );
    expect(screen.getByText(/估算/)).toBeInTheDocument();
  });
});

describe("AICostStrip 单价可信度", () => {
  it("P0 回归：单价未知时显示「单价未知」，不得印出兜底猜的金额", async () => {
    lastCall = call({ cost: 1.23, pricingKnown: false });

    renderWithProviders(<AICostStrip aiConfigured projectCount={3} />);

    await waitFor(() =>
      expect(screen.getByText("单价未知")).toBeInTheDocument(),
    );
    expect(screen.queryByText("¥1.23")).not.toBeInTheDocument();
  });

  it("单价已知时照常出数", async () => {
    lastCall = call({ cost: 1.23, pricingKnown: true });

    renderWithProviders(<AICostStrip aiConfigured projectCount={3} />);

    await waitFor(() => expect(screen.getByText("¥1.23")).toBeInTheDocument());
    expect(screen.queryByText("单价未知")).not.toBeInTheDocument();
  });

  it("字段缺省按已知处理：老数据不该被一律打成不可信", async () => {
    lastCall = call({ cost: 1.23 });

    renderWithProviders(<AICostStrip aiConfigured projectCount={3} />);

    await waitFor(() => expect(screen.getByText("¥1.23")).toBeInTheDocument());
    expect(screen.queryByText("单价未知")).not.toBeInTheDocument();
  });
});
