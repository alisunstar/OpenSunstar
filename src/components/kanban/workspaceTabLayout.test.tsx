import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { KanbanPage } from "@/components/kanban/KanbanPage";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import type { WorkspaceTab } from "@/types/workspace";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 三个 Tab 的职责归属（审查报告 §3.3）。
 *
 * 重划之前，「今日工作台」堆了 11 个纵向块，什么都有；`AICostStrip` 和
 * `AINLQueryBar` 在它和「项目看板」下 **各挂一份**，同一份数据每个 Tab 拉两次
 * （§3.1）；`AIWeeklyReport` 常驻 sticky header，压在三个 Tab 头上，在
 * 「AI 资产总览」顶部尤其突兀。
 *
 * 这是本仓第一个 **非空态** 的整页测试。空态测试保不住这类缺陷 —— 空态下
 * 三个 Tab 长得一样。所以这里必须喂真项目进去。
 *
 * 断言刻意用「哪个组件出现在哪个 Tab」而不是像素/顺序：职责归属是这次重划的
 * 结论，块与块之间的排序不是。命名用正则兼容 §2.5 的后续改名（治理总览 →
 * 配置生效率），否则这个测试会在下一梯队变成噪音。
 */

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
    aiConfigured: true,
    refreshConfig: vi.fn(),
    getConfig: () => ({
      provider: "deepseek",
      apiKey: "k",
      model: "deepseek-chat",
    }),
  }),
}));

/**
 * 替身返回的 Map 必须是 **模块级常量**，不能每次调用现造。
 *
 * `KanbanPage.tsx` 有一个 `useEffect` 把 `agentReadinessMap` 放进依赖数组、
 * 在 `projects.length > 0` 时 `setReadinessLastUpdatedAt(Date.now())`。真
 * `useAgentReadinessBatch` 用 `useState` 持有这个 Map，两次渲染之间同一个引用；
 * 而 `() => ({ agentReadinessMap: new Map() })` 每渲染一次换一个引用 ——
 * 依赖变 → setState → 重渲染 → 依赖又变，直接把 vitest worker 撑爆
 * （表现是 `Error: Worker exited unexpectedly`，不是断言失败）。
 *
 * 空态测试碰不到：`projects.length > 0` 不成立，effect 不写 state。这是
 * 「第一个非空态测试」独有的坑，留给后来人。
 */
const EMPTY_MAP = new Map();
const SCAN_PROGRESS = { done: 0, total: 0 };

/**
 * 组合矩阵的 Y 轴现在画真实分数，缺分的项目直接不上图（§2.5）—— 此前它靠
 * `fallbackHealth` 给每个项目编一个 35~52 的假坐标，所以哪怕全空也能画出图来。
 * 假坐标删掉之后，这个测试必须真的喂一份就绪分进来，否则矩阵会因为「一个点都
 * 没有」整块不渲染，`portfolioMatrix` 锚点找不到 —— 那是数据没喂够，不是 Tab
 * 归属错了，两者混淆会让这个测试失去它本来的意义。
 */
function readinessEntry(score: number): AgentReadinessBatchEntry {
  return {
    score,
    driftCount: 0,
    scannedAt: 1_760_000_000,
    assessmentState: "managed",
    details: [
      {
        check_name: "mcp_enabled",
        label: "MCP 服务器",
        weight: 15,
        score: 15,
        detail: "",
        status: "ready",
      },
    ],
  };
}

const READINESS_MAP = new Map<string, AgentReadinessBatchEntry>([
  ["p1", readinessEntry(72)],
  ["p2", readinessEntry(38)],
]);

vi.mock("@/hooks/kanban/useProjectMetricsScan", () => ({
  useProjectMetricsScan: () => ({
    codeLinesMap: EMPTY_MAP,
    versionMap: EMPTY_MAP,
    gitInfoMap: EMPTY_MAP,
    commits7dMap: EMPTY_MAP,
    commits30dMap: EMPTY_MAP,
    contributorsMap: EMPTY_MAP,
    weeklyCommitsMap: EMPTY_MAP,
    scanning: false,
    scanProgress: SCAN_PROGRESS,
    scanEpoch: 0,
    refreshScan: vi.fn(),
  }),
}));

vi.mock("@/hooks/kanban/usePortfolioAIAnalysis", () => ({
  usePortfolioAIAnalysis: () => ({
    aiSummaryMap: EMPTY_MAP,
    aiHealthMap: EMPTY_MAP,
    aiLoadingMap: EMPTY_MAP,
    aiTrendInsightMap: EMPTY_MAP,
  }),
}));

vi.mock("@/hooks/kanban/useAgentReadinessBatch", () => ({
  useAgentReadinessBatch: () => ({
    agentReadinessMap: READINESS_MAP,
    loading: false,
    failedCount: 0,
  }),
}));

vi.mock("@/hooks/kanban/usePortfolioAssetSummary", () => ({
  usePortfolioAssetSummary: () => ({
    assetMap: EMPTY_MAP,
    loading: false,
    error: null,
    lastUpdatedAt: null,
  }),
}));

const getAICostSummary = vi.fn().mockResolvedValue({
  total_cost: 1.5,
  total_tokens: 12000,
  insight_count: 3,
  nl_query_count: 1,
  period_days: 30,
  by_type: {},
});

vi.mock("@/api/aiInsight", async () => {
  const actual =
    await vi.importActual<typeof import("@/api/aiInsight")>("@/api/aiInsight");
  return {
    ...actual,
    getAICostSummary: (...args: unknown[]) => getAICostSummary(...args),
  };
});

const PROJECTS: Project[] = [
  {
    id: "p1",
    name: "OpenSunstar",
    path: "/tmp/opensunstar",
    addedAt: "2026-01-01T00:00:00.000Z",
  },
  {
    id: "p2",
    name: "Nebula",
    path: "/tmp/nebula",
    addedAt: "2026-01-02T00:00:00.000Z",
  },
];

/** 各组件在页面上的稳定锚点。 */
const ANCHOR = {
  costStrip: /AI 成本透明/,
  nlQueryBar: /AI 正在守护/,
  /** §2.5 会把「治理总览」改名成「配置生效率」，这里两边都认。 */
  governance: /治理总览|配置生效率/,
  portfolioMatrix: /项目组合矩阵/,
  weeklyReport: /生成周报/,
  /**
   * 「今日工作台」里唯一一份项目清单（PortfolioHealthSummary）的标题。
   *
   * 必须锚定首尾：同一个组件里还有「尚未完成 AI 配置状态扫描」和「所有项目
   * AI 配置状态正常。」两句正文，松散的 `/配置状态/` 会一次命中三个元素，
   * `getByText` 直接抛 "Found multiple elements"。
   */
  healthSummary: /^配置状态$/,
};

function renderTab(tab: WorkspaceTab, onWorkspaceTabChange = vi.fn()) {
  return renderWithProviders(
    <KanbanPage
      projects={PROJECTS}
      workspaceTab={tab}
      onWorkspaceTabChange={onWorkspaceTabChange}
      onProjectClick={vi.fn()}
      onOpenProjectAiConfig={vi.fn()}
      onProjectRemove={vi.fn()}
      onAddProject={vi.fn()}
    />,
  );
}

describe("今日工作台只回答「今天该动哪个项目」", () => {
  it("留下问答入口、成本条与周报，交出生效率与矩阵", async () => {
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.nlQueryBar)).toBeInTheDocument(),
    );
    // 周报从 sticky header 下沉到这里：它是组合级摘要，本就属于这个 Tab。
    expect(screen.getByText(ANCHOR.weeklyReport)).toBeInTheDocument();
    // 成本条答的是「本期烧了多少」，和「今天该动哪个项目」同一个决策。
    expect(screen.getByText(ANCHOR.costStrip)).toBeInTheDocument();

    expect(screen.queryByText(ANCHOR.governance)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.portfolioMatrix)).not.toBeInTheDocument();
  });
});

describe("今日工作台只画一份项目清单、只报一次自己的名字", () => {
  /**
   * 这两条是整页级的，`TodayWorkspace.test.tsx` 守不住。
   *
   * 那个文件只渲染 `TodayWorkspace` 一个组件，所以它能断言的是「这个组件里没有
   * 第二份列表」；而「同一个 Tab 里有没有两份列表」「这个词在一屏里出现几次」
   * 是相邻组件之间的事 —— 只有把整页渲染出来才看得见。§3.1 那类缺陷全长在
   * 这个缝里：每个组件单独看都对，拼起来才开始互相打架。
   */
  it("P0 回归：不得再出现第二份「建议优先处理」", async () => {
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.healthSummary)).toBeInTheDocument(),
    );

    // TodayWorkspace 曾在 PortfolioHealthSummary 旁边另挂一份，两份读同一个
    // agentReadinessMap 却各算各的理由、各排各的序（§3.1）。
    expect(screen.queryByText("建议优先处理")).not.toBeInTheDocument();
  });

  it("P0 回归：「今日工作台」在这一屏里只出现一次 —— Tab 按钮上那次", async () => {
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.healthSummary)).toBeInTheDocument(),
    );

    // 面板自己曾经再写一个 <h3>今日工作台</h3>，紧贴在 Tab 按钮下方，
    // 第二次不带任何新信息。锚点是 WorkspaceTabBar 的 defaultLabel。
    expect(screen.getAllByText("今日工作台")).toHaveLength(1);
  });
});

describe("项目看板是项目集合的空间视图", () => {
  it("只收四象限图，不重复挂问答栏与成本条", async () => {
    renderTab("board");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.portfolioMatrix)).toBeInTheDocument(),
    );

    // 重复挂载的两份都已删（§3.1），它们现在只活在「今日工作台」。
    expect(screen.queryByText(ANCHOR.nlQueryBar)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.costStrip)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.governance)).not.toBeInTheDocument();
  });
});

describe("AI 资产总览是配置落地的唯一权威视图", () => {
  it("收下配置生效率，且不再被周报压在顶部", async () => {
    renderTab("assetsMatrix");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.governance)).toBeInTheDocument(),
    );

    // 周报与「配置落地」语义无关，却曾常驻 header 压在这个 Tab 顶部（§3.3）。
    expect(screen.queryByText(ANCHOR.weeklyReport)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.costStrip)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.nlQueryBar)).not.toBeInTheDocument();
  });
});

/**
 * `role="tab"` 是一份承诺，之前一条都没兑现（审查报告 §7）。
 *
 * 它向读屏器承诺：(1) 存在一块我控制的内容区，(2) 方向键能在标签之间走。
 * 原来的标签栏两条都做不到 —— 没有任何元素带 `role="tabpanel"`，方向键
 * 什么也不做，键盘用户得一路 Tab 穿过三个按钮。
 *
 * 这几条只能在整页测：面板在 `KanbanPage` 里，标签在 `WorkspaceTabBar` 里，
 * `aria-controls` 指向的 id 是否真的存在，只有两边同时渲染才验得出来 ——
 * 而这正是 `aria-controls` 最容易坏的方式：指向一个不存在的 id，界面上
 * 什么都看不出来，读屏器只是默默找不到面板。
 */
describe("三个 Tab 的 ARIA 契约", () => {
  it("P0 回归：aria-controls 指向的面板真实存在", async () => {
    renderTab("dashboard");
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.healthSummary)).toBeInTheDocument(),
    );

    const panel = screen.getByRole("tabpanel");
    for (const tab of screen.getAllByRole("tab")) {
      expect(tab.getAttribute("aria-controls")).toBe(panel.id);
    }
    expect(panel.id).toBeTruthy();
  });

  it("面板由**当前选中**的那个 Tab 命名", async () => {
    renderTab("assetsMatrix");
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.governance)).toBeInTheDocument(),
    );

    expect(screen.getByRole("tabpanel")).toHaveAccessibleName("AI 资产总览");
  });

  it("roving tabindex：标签栏在 Tab 键序里只占一站", async () => {
    renderTab("board");
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.portfolioMatrix)).toBeInTheDocument(),
    );

    const tabs = screen.getAllByRole("tab");
    const tabbable = tabs.filter((el) => el.getAttribute("tabindex") === "0");
    expect(tabbable).toHaveLength(1);
    expect(tabbable[0]).toHaveAttribute("aria-selected", "true");
  });

  it("方向键能切换标签 —— tablist 承诺过的第二件事", async () => {
    const onChange = vi.fn();
    renderTab("dashboard", onChange);
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.healthSummary)).toBeInTheDocument(),
    );

    screen.getByRole("tab", { name: /今日工作台/ }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onChange).toHaveBeenCalledWith("board");

    onChange.mockClear();
    // 左键从第一个绕回最后一个：三个标签是环形的，不该在两端撞墙。
    await userEvent.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenCalledWith("assetsMatrix");

    onChange.mockClear();
    await userEvent.keyboard("{End}");
    expect(onChange).toHaveBeenCalledWith("assetsMatrix");
  });

  it("空态下不渲染孤立的 tabpanel", () => {
    // 空态不渲染标签栏。此时那块内容区若仍自称 tabpanel，读屏器会报出一个
    // 没有任何 tab 指向它的「标签面板」。
    renderWithProviders(
      <KanbanPage
        projects={[]}
        workspaceTab="dashboard"
        onWorkspaceTabChange={vi.fn()}
        onProjectClick={vi.fn()}
        onOpenProjectAiConfig={vi.fn()}
        onProjectRemove={vi.fn()}
        onAddProject={vi.fn()}
      />,
    );

    expect(screen.queryAllByRole("tab")).toHaveLength(0);
    expect(screen.queryByRole("tabpanel")).not.toBeInTheDocument();
  });
});

describe("成本汇总在整页范围内只拉一次", () => {
  it("P0 回归：切到哪个 Tab 都不该把同一份数据问两遍", async () => {
    getAICostSummary.mockClear();
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.costStrip)).toBeInTheDocument(),
    );
    await waitFor(() => expect(getAICostSummary).toHaveBeenCalled());

    expect(getAICostSummary).toHaveBeenCalledTimes(1);
  });
});
