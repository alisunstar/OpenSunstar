import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { AgentReadinessItem } from "@/api/aiInsight";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import { TodayWorkspace } from "@/components/kanban/TodayWorkspace";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * TodayWorkspace 现在只出数、不出列表。
 *
 * 它以前还挂着一份「建议优先处理」，和同一屏的 `PortfolioHealthSummary` 是两份
 * 读同一个 `agentReadinessMap`、却各算各理由的项目清单（§3.1）。那份被删了，
 * 因此本文件里关于「原因串」「就绪分徽章」「缺 MCP 项目卡」的断言也一并移除 ——
 * 它们守的行为整体搬到了 `PortfolioHealthSummary.test.tsx`（那里已有
 * 「后端判定 unmanaged 时显示『未纳管』且不展示分数」等用例）。
 *
 * 留在这里的是**只有这个组件才回答的两件事**：活跃窗口口径（§4.1）和平均就绪分
 * 的纳管口径。
 */

const PROJECTS: Project[] = [
  {
    id: "p1",
    name: "alpha",
    path: "E:/projects/alpha",
    addedAt: new Date("2026-07-01").toISOString(),
  },
  {
    id: "p2",
    name: "beta",
    path: "E:/projects/beta",
    addedAt: new Date("2026-07-02").toISOString(),
  },
  {
    id: "p3",
    name: "gamma",
    path: "E:/projects/gamma",
    addedAt: new Date("2026-07-03").toISOString(),
  },
];

function item(partial: Partial<AgentReadinessItem> = {}): AgentReadinessItem {
  return {
    check_name: "mcp_enabled",
    label: "MCP 服务器",
    weight: 15,
    score: 0,
    detail: "",
    status: "missing",
    ...partial,
  };
}

interface RenderOptions {
  projects?: Project[];
  commitsInWindowMap?: Map<string, number>;
  overviewWindowDays?: number;
  agentReadinessMap?: Map<string, AgentReadinessBatchEntry>;
  totalCommitsInWindow?: number;
  averageActivityLabel?: string;
  onOverviewWindowDaysChange?: (days: 7 | 30) => void;
}

function renderWorkspace(options: RenderOptions = {}) {
  const commitsInWindowMap = options.commitsInWindowMap ?? new Map();
  // 默认让提交总数与逐项目提交对得上，避免测试自己造出不自洽的输入。
  const totalCommitsInWindow =
    options.totalCommitsInWindow ??
    [...commitsInWindowMap.values()].reduce((a, b) => a + b, 0);

  return renderWithProviders(
    <TodayWorkspace
      projects={options.projects ?? PROJECTS}
      getStage={() => "mvp"}
      progressMap={new Map()}
      agentReadinessMap={options.agentReadinessMap ?? new Map()}
      commitsInWindowMap={commitsInWindowMap}
      overviewWindowDays={options.overviewWindowDays ?? 7}
      onOverviewWindowDaysChange={options.onOverviewWindowDaysChange ?? vi.fn()}
      totalCodeLines={0}
      totalCommitsInWindow={totalCommitsInWindow}
      averageActivityLabel={options.averageActivityLabel ?? "低"}
      averageActivityColor=""
    />,
  );
}

/** 读某张 SummaryCard 的整块文本：label 与 value 在同一个卡片容器里。 */
function cardText(label: string): string {
  const node = screen.getByText(label);
  return node.closest("div")?.parentElement?.textContent ?? "";
}

describe("TodayWorkspace 活跃窗口", () => {
  it("7 天窗口下按 7 天数据统计活跃项目数", () => {
    // alpha 有提交、beta/gamma 没有 → 近 7 天活跃 = 1（共 3 个项目）
    renderWorkspace({
      commitsInWindowMap: new Map([
        ["p1", 3],
        ["p2", 0],
        ["p3", 0],
      ]),
      overviewWindowDays: 7,
    });

    expect(screen.getByText("近 7 天活跃")).toBeInTheDocument();
    // 断言 `1/3` 而不是 `1`：分母把「1 个活跃」放回「共 3 个」的语境里，
    // 也让断言不会被副标题里的其它数字碰巧满足。
    expect(cardText("近 7 天活跃")).toContain("1/3");
  });

  it("P0 回归：切到 30 天窗口后数字必须跟着换口径，不能只换标签", () => {
    // 同样三个项目，30 天窗口里 alpha/beta/gamma 都有提交 → 近 30 天活跃 = 3。
    // 旧实现固定读 commits7dMap，标签写「近 30 天」而数字还是 7 天的，
    // 用户切了窗口只看到标题变了（审查报告 §4.1）。
    renderWorkspace({
      commitsInWindowMap: new Map([
        ["p1", 12],
        ["p2", 4],
        ["p3", 1],
      ]),
      overviewWindowDays: 30,
    });

    expect(screen.getByText("近 30 天活跃")).toBeInTheDocument();
    expect(cardText("近 30 天活跃")).toContain("3/3");
  });

  it("窗口内零提交时活跃数为 0", () => {
    renderWorkspace({
      commitsInWindowMap: new Map([
        ["p1", 0],
        ["p2", 0],
        ["p3", 0],
      ]),
      overviewWindowDays: 30,
    });

    expect(cardText("近 30 天活跃")).toContain("0/3");
  });

  /**
   * 合并前这三个数字占着三张卡：「近 N 天活跃」（活跃项目数）、「近 N 天提交」
   * （提交总数）、「平均活跃度」（提交数分档）——而「近 N 天提交」的数值恰好就是
   * 「平均活跃度」那张卡的副标题，同一个数字在相邻两张卡上各画一次。
   */
  it("三张活跃度卡合并成一张：主数是活跃项目数，提交总数与活跃档在副标题", () => {
    renderWorkspace({
      commitsInWindowMap: new Map([
        ["p1", 12],
        ["p2", 4],
        ["p3", 1],
      ]),
      overviewWindowDays: 30,
      averageActivityLabel: "很高",
    });

    const text = cardText("近 30 天活跃");
    expect(text).toContain("3/3");
    expect(text).toContain("共 17 次提交");
    expect(text).toContain("活跃度 很高");

    // 合并掉的两张卡不该再各自存在
    expect(screen.queryByText("近 30 天提交")).not.toBeInTheDocument();
    expect(screen.queryByText(/平均活跃度/)).not.toBeInTheDocument();
  });

  it("窗口切换器改的是外部状态，组件自己不另存一份", async () => {
    const onOverviewWindowDaysChange = vi.fn();
    renderWorkspace({ overviewWindowDays: 7, onOverviewWindowDaysChange });

    await userEvent.click(screen.getByRole("button", { name: "30 天" }));

    expect(onOverviewWindowDaysChange).toHaveBeenCalledWith(30);
  });
});

describe("TodayWorkspace 纳管口径", () => {
  const ONLY_ALPHA = [PROJECTS[0]];

  it("P0 回归：未纳管项目不得拉低平均就绪分", () => {
    // 后端 classify_unmanaged_readiness（agent_readiness.rs:387-413）会把
    // 未纳管项目的分数归零并改判 status，此时 score 不具判定意义
    // （cli_api.rs:404 在 CLI 侧直接给 None）。把 0 算进平均就是拿「还没配」
    // 冒充「坏了」。一个项目都判定不了时该给「—」，不是「0」。
    renderWorkspace({
      projects: ONLY_ALPHA,
      agentReadinessMap: new Map([
        [
          "p1",
          {
            score: 0,
            driftCount: 0,
            scannedAt: 1,
            assessmentState: "unmanaged",
            details: [
              item({ check_name: "mcp_enabled", status: "unmanaged" }),
              item({ check_name: "skills", status: "unmanaged" }),
            ],
          },
        ],
      ]),
    });

    expect(cardText("平均就绪分")).toContain("—");
  });

  it("对照组：已纳管的项目照常计入平均就绪分", () => {
    renderWorkspace({
      projects: ONLY_ALPHA,
      agentReadinessMap: new Map([
        [
          "p1",
          {
            score: 40,
            driftCount: 0,
            scannedAt: 1,
            assessmentState: "managed",
            details: [
              item({ check_name: "mcp_enabled", status: "missing" }),
              item({ check_name: "skills", status: "ready", score: 12 }),
            ],
          },
        ],
      ]),
    });

    expect(cardText("平均就绪分")).toContain("40");
  });
});

describe("TodayWorkspace 不再是第二份项目清单", () => {
  it("P0 回归：不得再挂「建议优先处理」列表", () => {
    renderWorkspace({
      agentReadinessMap: new Map([
        [
          "p1",
          {
            score: 20,
            driftCount: 3,
            scannedAt: 1,
            assessmentState: "managed",
            details: [item({ status: "missing" })],
          },
        ],
      ]),
    });

    expect(screen.queryByText("建议优先处理")).not.toBeInTheDocument();
    // 项目名只出现在 PortfolioHealthSummary 那一份清单里
    expect(screen.queryByText("alpha")).not.toBeInTheDocument();
    // 「配置资产」按钮随列表一起走了
    expect(
      screen.queryByRole("button", { name: /配置资产/ }),
    ).not.toBeInTheDocument();
  });

  it("P0 回归：面板不再自报「今日工作台」—— Tab 栏已经写着了", () => {
    renderWorkspace();

    expect(screen.queryByText("今日工作台")).not.toBeInTheDocument();
    // 副标题留着：它说的是范围，Tab 名说的是「你在哪儿」
    expect(screen.getByText(/共 3 个项目/)).toBeInTheDocument();
  });
});
