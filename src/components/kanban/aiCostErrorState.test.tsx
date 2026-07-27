import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AICostProvider } from "@/contexts/AICostContext";
import { AICostStrip } from "@/components/kanban/AICostStrip";
import { AINLQueryBar } from "@/components/kanban/AINLQueryBar";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 「失败 ≠ 零值」回归 · 成本展示（审查报告 §5.6）。
 *
 * `getAICostSummary` 在 API 层统一 `catch → return null`。两个成本组件都写
 * `summary?.total_cost ?? 0`，于是「查询失败」被渲染成
 * 「本月累计 ¥0.00 / 0 次分析」。
 *
 * 这是最坏的一种谎：用户据此认为「AI 没花钱」，而真实账单可能不是 0。
 * 未知必须长得像未知。
 *
 * 拉取搬进 `AICostProvider` 之后（§3.1 重复挂载），这里刻意 **不** mock
 * context —— 三态判定现在发生在 Provider 里，只测组件等于把断言挂在半空。
 * 用真 Provider 跑，等价的用户可见结果才守得住。
 */

const getAICostSummary = vi.fn();

vi.mock("@/api/aiInsight", async () => {
  const actual =
    await vi.importActual<typeof import("@/api/aiInsight")>("@/api/aiInsight");
  return {
    ...actual,
    getAICostSummary: (...args: unknown[]) => getAICostSummary(...args),
  };
});

vi.mock("@/hooks/useAIInsight", () => ({
  useNLQuery: () => ({
    answer: null,
    isLoading: false,
    error: null,
    costEstimate: 0,
    pricingKnown: true,
    queryLogId: null,
    ask: vi.fn(),
  }),
}));

const REAL_ZERO = {
  total_cost: 0,
  total_tokens: 0,
  insight_count: 0,
  nl_query_count: 0,
  by_type: {},
};

/**
 * `AINLQueryBar.tsx:95` 只在「有调用记录」时才渲染成本徽章，所以那个组件的
 * 「真零」场景必须带上调用次数：花了 0 元 ≠ 一次都没调用过。
 */
const REAL_ZERO_WITH_CALLS = {
  ...REAL_ZERO,
  insight_count: 2,
};

beforeEach(() => {
  getAICostSummary.mockReset();
});

describe("AICostStrip 成本查询失败", () => {
  it("查询失败不得显示「¥0.00」冒充真实零花费", async () => {
    getAICostSummary.mockResolvedValue(null);

    renderWithProviders(
      <AICostProvider aiConfigured>
        <AICostStrip aiConfigured projectCount={3} onOpenRoiPanel={() => {}} />
      </AICostProvider>,
    );

    await waitFor(() => expect(getAICostSummary).toHaveBeenCalled());

    await waitFor(() =>
      expect(screen.getByText(/查询失败|无法获取/)).toBeInTheDocument(),
    );
    expect(screen.queryByText("¥0.00")).not.toBeInTheDocument();
  });

  it("真实的 0 花费仍然要显示 ¥0.00", async () => {
    getAICostSummary.mockResolvedValue(REAL_ZERO);

    renderWithProviders(
      <AICostProvider aiConfigured>
        <AICostStrip aiConfigured projectCount={3} onOpenRoiPanel={() => {}} />
      </AICostProvider>,
    );

    await waitFor(() => expect(screen.getByText("¥0.00")).toBeInTheDocument());
    expect(screen.queryByText(/查询失败|无法获取/)).not.toBeInTheDocument();
  });
});

describe("AINLQueryBar 成本查询失败", () => {
  it("查询失败不得把成本渲染成 ¥0.00", async () => {
    getAICostSummary.mockResolvedValue(null);

    renderWithProviders(
      <AICostProvider aiConfigured>
        <AINLQueryBar projectContexts={[]} aiConfigured projectCount={3} />
      </AICostProvider>,
    );

    await waitFor(() => expect(getAICostSummary).toHaveBeenCalled());

    await waitFor(() =>
      expect(screen.getByText(/查询失败|无法获取/)).toBeInTheDocument(),
    );
    expect(screen.queryByText("¥0.00")).not.toBeInTheDocument();
  });

  it("真实的 0 花费仍然要显示 ¥0.00", async () => {
    getAICostSummary.mockResolvedValue(REAL_ZERO_WITH_CALLS);

    renderWithProviders(
      <AICostProvider aiConfigured>
        <AINLQueryBar projectContexts={[]} aiConfigured projectCount={3} />
      </AICostProvider>,
    );

    await waitFor(() => expect(screen.getByText("¥0.00")).toBeInTheDocument());
    expect(screen.queryByText(/查询失败|无法获取/)).not.toBeInTheDocument();
  });
});
