import { describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen } from "@testing-library/react";
import { KanbanPage } from "@/components/kanban/KanbanPage";
import { renderWithProviders } from "../../../tests/renderWithProviders";

vi.mock("framer-motion", () => ({
  motion: {
    div: ({
      children,
      ...props
    }: React.PropsWithChildren<Record<string, unknown>>) => (
      <div {...props}>{children}</div>
    ),
  },
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
}));

vi.mock("@/hooks/useAIConfig", () => ({
  useAIConfig: () => ({
    aiConfigured: false,
    refreshConfig: vi.fn(),
    getConfig: () => null,
  }),
}));

vi.mock("@/hooks/kanban/useProjectMetricsScan", () => ({
  useProjectMetricsScan: () => ({
    codeLinesMap: new Map(),
    versionMap: new Map(),
    gitInfoMap: new Map(),
    commits7dMap: new Map(),
    commits30dMap: new Map(),
    contributorsMap: new Map(),
    weeklyCommitsMap: new Map(),
    scanning: false,
    scanProgress: { done: 0, total: 0 },
    scanEpoch: 0,
    refreshScan: vi.fn(),
  }),
}));

vi.mock("@/hooks/kanban/usePortfolioAIAnalysis", () => ({
  usePortfolioAIAnalysis: () => ({
    aiSummaryMap: new Map(),
    aiHealthMap: new Map(),
    aiLoadingMap: new Map(),
    aiTrendInsightMap: new Map(),
  }),
}));

vi.mock("@/hooks/kanban/useAgentReadinessBatch", () => ({
  useAgentReadinessBatch: () => ({ agentReadinessMap: new Map() }),
}));

describe("KanbanPage empty state", () => {
  it("shows empty placeholder and add buttons", () => {
    const onAddProject = vi.fn();

    renderWithProviders(
      <KanbanPage
        projects={[]}
        onProjectClick={vi.fn()}
        onProjectRemove={vi.fn()}
        onAddProject={onAddProject}
      />,
    );

    expect(screen.getByText("暂无项目")).toBeInTheDocument();
    expect(
      screen.getByText("点击下方按钮或在侧边栏添加你的第一个项目"),
    ).toBeInTheDocument();

    const addButtons = screen.getAllByRole("button", { name: /添加项目/ });
    expect(addButtons.length).toBeGreaterThanOrEqual(2);

    expect(screen.queryByPlaceholderText(/搜索项目/)).not.toBeInTheDocument();
    expect(screen.queryByText("项目总览")).not.toBeInTheDocument();
  });

  it("calls onAddProject from empty state CTA", async () => {
    const user = userEvent.setup();
    const onAddProject = vi.fn();

    renderWithProviders(
      <KanbanPage
        projects={[]}
        onProjectClick={vi.fn()}
        onProjectRemove={vi.fn()}
        onAddProject={onAddProject}
      />,
    );

    const addButtons = screen.getAllByRole("button", { name: /添加项目/ });
    await user.click(addButtons[0]);
    expect(onAddProject).toHaveBeenCalledTimes(1);
  });
});
