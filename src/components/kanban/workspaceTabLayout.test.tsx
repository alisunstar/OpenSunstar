import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { KanbanPage } from "@/components/kanban/KanbanPage";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import type { WorkspaceTab } from "@/types/workspace";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 两个 Tab 的职责归属（工作区重构 2026-07-30，三砍二版）。
 *
 * 第一版职责划分（审查报告 §3.3）把「今日工作台 / 项目看板 / AI 资产总览」
 * 按「今天 / 项目关系 / 配置落地」切开；重构再进一步：
 *
 * - 「今日工作台」改告警制首屏 —— 只回答「今天有没有事」：问答栏、成本条、
 *   周报留下，聚合指标卡、健康清单、阶段分布全部搬走；
 * - 「项目看板」吸收「AI 资产总览」—— 四象限、指标卡、待办清单、治理面板、
 *   资产矩阵同在这一屏，项目维度的巡视内容不再分两个 Tab。
 *
 * 断言刻意用「哪个组件出现在哪个 Tab」而不是像素/顺序：职责归属是这次重划的
 * 结论，块与块之间的排序不是。
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

// Feature Flag 默认关闭，但本测试文件验证的是「告警制首屏」开启后的职责归属。
vi.mock("@/hooks/useWorkspaceAlertFirst", () => ({
  useWorkspaceAlertFirst: () => ({
    enabled: true,
    setEnabled: vi.fn(),
    toggle: vi.fn(),
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
  /** §2.5 会把「治理总览」改名成「配置生效率」，这里两边都认。 */
  governance: /治理总览|配置生效率/,
  portfolioMatrix: /项目组合矩阵/,
  weeklyReport: /生成周报/,
  /** 「今天没事」是告警制首屏的健康态文案（TodayAlertsPanel）。 */
  allClear: /^今天没事$/,
  /**
   * 「需要动手的项目」清单（PortfolioHealthSummary）的标题。
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

describe("今日工作台只回答「今天有没有事」", () => {
  it("留下成本条、周报与告警区，交出指标、清单与矩阵", async () => {
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.allClear)).toBeInTheDocument(),
    );
    // 周报从 sticky header 下沉到这里：它是组合级摘要，本就属于这个 Tab。
    expect(screen.getByText(ANCHOR.weeklyReport)).toBeInTheDocument();
    // 成本条答的是「本期烧了多少」，和告警卡同一个决策场景。
    expect(screen.getByText(ANCHOR.costStrip)).toBeInTheDocument();

    // mock 的项目全部健康（无漂移无缺口）→ 告警区是健康态「今天没事」。
    expect(screen.getByText(ANCHOR.allClear)).toBeInTheDocument();

    // 巡视类内容全部搬走：指标、清单、矩阵、治理面板不再占开机第一眼。
    expect(screen.queryByText(ANCHOR.governance)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.portfolioMatrix)).not.toBeInTheDocument();
    expect(screen.queryByText(ANCHOR.healthSummary)).not.toBeInTheDocument();
  });
});

describe("今日工作台只报一次自己的名字", () => {
  it("P0 回归：「今日工作台」在这一屏里只出现一次 —— Tab 按钮上那次", async () => {
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.allClear)).toBeInTheDocument(),
    );

    // 面板自己曾经再写一个 <h3>今日工作台</h3>，紧贴在 Tab 按钮下方，
    // 第二次不带任何新信息。锚点是 WorkspaceTabBar 的 defaultLabel。
    expect(screen.getAllByText("今日工作台")).toHaveLength(1);
  });

  it("P0 回归：不得再出现「建议优先处理」清单", async () => {
    renderTab("dashboard");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.allClear)).toBeInTheDocument(),
    );

    // TodayWorkspace 曾在 PortfolioHealthSummary 旁边另挂一份，两份读同一个
    // agentReadinessMap 却各算各的理由、各排各的序（§3.1）。
    expect(screen.queryByText("建议优先处理")).not.toBeInTheDocument();
  });
});

describe("项目看板吸收 AI 资产总览，是项目维度的家", () => {
  it("收下四象限、待办清单与配置生效率，不重复挂问答栏与成本条", async () => {
    renderTab("board");

    await waitFor(() =>
      expect(screen.getByText(ANCHOR.portfolioMatrix)).toBeInTheDocument(),
    );

    // 原「AI 资产总览」Tab 的内容（治理面板/生效率）并入这一屏。
    expect(screen.getByText(ANCHOR.governance)).toBeInTheDocument();
    // 完整待办清单也从「今日」迁入：告警卡只出 top 5，全景在这里。
    expect(screen.getByText(ANCHOR.healthSummary)).toBeInTheDocument();

    // 重复挂载的两份都已删（§3.1），成本条现在只活在「今日工作台」。
    expect(screen.queryByText(ANCHOR.costStrip)).not.toBeInTheDocument();
    // 周报与「配置落地」语义无关，不许再压到这一屏顶部。
    expect(screen.queryByText(ANCHOR.weeklyReport)).not.toBeInTheDocument();
  });
});

/**
 * `role="tab"` 是一份承诺（审查报告 §7）。
 *
 * 它向读屏器承诺：(1) 存在一块我控制的内容区，(2) 方向键能在标签之间走。
 * 这几条只能在整页测：面板在 `KanbanPage` 里，标签在 `WorkspaceTabBar` 里，
 * `aria-controls` 指向的 id 是否真的存在，只有两边同时渲染才验得出来 ——
 * 而这正是 `aria-controls` 最容易坏的方式：指向一个不存在的 id，界面上
 * 什么都看不出来，读屏器只是默默找不到面板。
 */
describe("两个 Tab 的 ARIA 契约", () => {
  it("P0 回归：aria-controls 指向的面板真实存在", async () => {
    renderTab("dashboard");
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.allClear)).toBeInTheDocument(),
    );

    const panel = screen.getByRole("tabpanel");
    for (const tab of screen.getAllByRole("tab")) {
      expect(tab.getAttribute("aria-controls")).toBe(panel.id);
    }
    expect(panel.id).toBeTruthy();
  });

  it("面板由**当前选中**的那个 Tab 命名", async () => {
    renderTab("board");
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.portfolioMatrix)).toBeInTheDocument(),
    );

    expect(screen.getByRole("tabpanel")).toHaveAccessibleName("项目看板");
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

  it("方向键能切换标签 —— 两个标签也是环形的，不在两端撞墙", async () => {
    const onChange = vi.fn();
    renderTab("dashboard", onChange);
    await waitFor(() =>
      expect(screen.getByText(ANCHOR.allClear)).toBeInTheDocument(),
    );

    screen.getByRole("tab", { name: /今日工作台/ }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onChange).toHaveBeenCalledWith("board");

    onChange.mockClear();
    // 左键从第一个绕回最后一个。
    await userEvent.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenCalledWith("board");

    onChange.mockClear();
    await userEvent.keyboard("{End}");
    expect(onChange).toHaveBeenCalledWith("board");
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
  });
});
