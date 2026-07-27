import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AICostProvider } from "@/contexts/AICostContext";
import { AICostStrip } from "@/components/kanban/AICostStrip";
import { AINLQueryBar } from "@/components/kanban/AINLQueryBar";
import { renderWithProviders } from "../../tests/renderWithProviders";

/**
 * 成本汇总只能拉一次（审查报告 §3.1「重复挂载」）。
 *
 * `AICostStrip` 与 `AINLQueryBar` 各自写了一份 `getAICostSummary(30)` 的
 * `useEffect`，而这两个组件在「今日工作台」和「项目看板」两个 Tab 下都挂着 ——
 * 同一份数据每切一次 Tab 就要往后端跑两趟。
 *
 * `AICostContext` 早就存在，两处也都消费了它的 `refreshToken` 来决定
 * **什么时候重拉**，却谁都没把 **数据本身** 提上去。这是典型的「同步了时钟、
 * 没同步内容」。
 *
 * 这里守住的是数据层的唯一性：无论页面上挂了几个消费者，一个窗口期内
 * 后端只该被问一次。
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

const SUMMARY = {
  total_cost: 1.5,
  total_tokens: 12000,
  insight_count: 3,
  nl_query_count: 1,
  period_days: 30,
  by_type: {},
};

beforeEach(() => {
  getAICostSummary.mockReset();
  getAICostSummary.mockResolvedValue(SUMMARY);
});

describe("AICostProvider 是成本汇总的唯一拉取方", () => {
  it("P0 回归：两个消费者同时挂载，后端只被问一次", async () => {
    renderWithProviders(
      <AICostProvider aiConfigured>
        <AICostStrip aiConfigured projectCount={3} />
        <AINLQueryBar projectContexts={[]} aiConfigured projectCount={3} />
      </AICostProvider>,
    );

    await waitFor(() => expect(getAICostSummary).toHaveBeenCalled());
    // 等两个消费者都渲染出金额，确保没有「第二个组件晚一拍才发请求」的漏网。
    await waitFor(() =>
      expect(screen.getAllByText("¥1.50").length).toBeGreaterThanOrEqual(2),
    );

    expect(getAICostSummary).toHaveBeenCalledTimes(1);
  });

  it("未配置 AI 时不发请求：那一趟往返没有意义，还会把「未配置」混进「查询失败」", async () => {
    renderWithProviders(
      <AICostProvider aiConfigured={false}>
        <AICostStrip aiConfigured={false} projectCount={3} />
        <AINLQueryBar
          projectContexts={[]}
          aiConfigured={false}
          projectCount={3}
        />
      </AICostProvider>,
    );

    await waitFor(() =>
      expect(screen.getByText(/尚未启用/)).toBeInTheDocument(),
    );
    expect(getAICostSummary).not.toHaveBeenCalled();
  });

  it("窗口天数由 Provider 统一决定，消费者不再各写一份 30", async () => {
    renderWithProviders(
      <AICostProvider aiConfigured rangeDays={7}>
        <AICostStrip aiConfigured projectCount={3} />
        <AINLQueryBar projectContexts={[]} aiConfigured projectCount={3} />
      </AICostProvider>,
    );

    await waitFor(() => expect(getAICostSummary).toHaveBeenCalledTimes(1));
    expect(getAICostSummary).toHaveBeenCalledWith(7);
  });
});
