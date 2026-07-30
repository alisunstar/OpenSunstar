import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { PageView } from "@/app/navigation";
import type { Project } from "@/types/project";
import { ProjectAiConfigPage } from "@/components/projects/ProjectAiConfigPage";
import { ProjectDetailSheet } from "@/components/kanban/ProjectDetailSheet";
import { makeProjectView } from "../../../tests/projectViewFactory";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 「AI 资产配置」从项目详情抽屉的第二个 Tab 升格成侧栏一级页之后，这一组测试
 * 守的是**唯一挂载点**这条不变式。
 *
 * 背景：审查报告 §3.1 反复清理的是同一类缺陷 —— 同一块 UI 挂在两处，改口径时
 * 永远漏掉一处，于是两个入口开始说不一样的话。抽屉那份被删掉是这次改动的**目的**，
 * 不是副产品；没有测试盯着，下次「顺手在抽屉里也放一个方便」就会把它加回来。
 *
 * 三块面板各自的行为由它们自己的测试覆盖，这里只问「渲染了几份、在哪儿」，
 * 所以全部替换成最小替身。
 */
vi.mock("framer-motion", async () => {
  const React = await import("react");
  const MotionDiv = React.forwardRef<HTMLDivElement, Record<string, unknown>>(
    (
      {
        children,
        initial: _initial,
        animate: _animate,
        exit: _exit,
        transition: _transition,
        ...props
      },
      ref,
    ) => (
      <div ref={ref} {...props}>
        {children as React.ReactNode}
      </div>
    ),
  );
  MotionDiv.displayName = "MotionDiv";
  return {
    motion: { div: MotionDiv },
    AnimatePresence: ({ children }: { children?: React.ReactNode }) => (
      <>{children}</>
    ),
  };
});

vi.mock("@/components/projects/ProjectAssetPanel", () => ({
  ProjectAssetPanel: () => <div data-testid="asset-panel" />,
}));
vi.mock("@/components/projects/ProjectBlueprintPanel", () => ({
  ProjectBlueprintPanel: () => <div data-testid="blueprint-panel" />,
}));
vi.mock("@/components/projects/ProjectAssetHealthSummary", () => ({
  ProjectAssetHealthSummary: () => <div data-testid="asset-health" />,
}));
vi.mock("@/components/projects/ProjectWikiPanel", () => ({
  ProjectWikiPanel: () => <div data-testid="project-wiki" />,
}));
vi.mock("@/components/projects/ProjectEnvironmentSnapshotPanel", () => ({
  ProjectEnvironmentSnapshotPanel: () => (
    <div data-testid="project-environment" />
  ),
}));
vi.mock("@/components/projects/ProjectFlowOrchestratorPanel", () => ({
  ProjectFlowOrchestratorPanel: () => <div data-testid="flow-panel" />,
}));
vi.mock("@/components/kanban/AgentReadinessPanel", () => ({
  AgentReadinessPanel: ({ compact }: { compact?: boolean }) => (
    <div data-testid={compact ? "readiness-compact" : "readiness-full"} />
  ),
}));
vi.mock("@/components/kanban/AIRiskAnalysis", () => ({
  AIRiskAnalysis: () => <div data-testid="risk" />,
}));
vi.mock("@/components/kanban/CommitTrendChart", () => ({
  CommitTrendChart: () => <div data-testid="commit-trend" />,
}));

vi.mock("@/hooks/useAIInsight", () => ({
  useAIRisk: () => ({ data: null, isLoading: false, refresh: vi.fn() }),
  useAgentReadiness: () => ({
    data: null,
    isLoading: false,
    refresh: vi.fn(),
    scanEffective: vi.fn(),
  }),
}));
vi.mock("@/hooks/useAIRoiReport", () => ({
  useAIRoiReport: () => ({ report: null }),
}));
vi.mock("@/contexts/AICostContext", () => ({
  useAICost: () => ({ refreshToken: 0 }),
}));

const ALPHA: Project = {
  id: "p1",
  name: "alpha",
  path: "E:/projects/alpha",
  addedAt: new Date("2026-07-01").toISOString(),
};
const BETA: Project = {
  id: "p2",
  name: "beta",
  path: "E:/projects/beta",
  addedAt: new Date("2026-07-02").toISOString(),
};

function renderPage(
  overrides: Partial<{
    projects: Project[];
    selectedProjectId: string | null;
    onSelectProject: (id: string) => void;
    onNavigate: (view: PageView) => void;
    onOpenProjectWorkflow: (projectId: string) => void;
  }> = {},
) {
  const onSelectProject = overrides.onSelectProject ?? vi.fn();
  const onNavigate = overrides.onNavigate ?? vi.fn();
  const onOpenProjectWorkflow = overrides.onOpenProjectWorkflow ?? vi.fn();
  return {
    onSelectProject,
    onNavigate,
    onOpenProjectWorkflow,
    ...renderWithProviders(
      <ProjectAiConfigPage
        projects={overrides.projects ?? [ALPHA, BETA]}
        selectedProjectId={
          overrides.selectedProjectId === undefined
            ? ALPHA.id
            : overrides.selectedProjectId
        }
        onSelectProject={onSelectProject}
        onNavigate={onNavigate}
        onOpenProjectWorkflow={onOpenProjectWorkflow}
      />,
    ),
  };
}

describe("项目资产配置：唯一挂载点", () => {
  it("默认首屏直接呈现蓝图和项目资产关联，不让长篇就绪列表把它们压到折叠线下", () => {
    renderPage();

    expect(
      screen.getByRole("heading", { name: "项目资产配置" }),
    ).toBeInTheDocument();
    expect(screen.getByText("项目级 · alpha")).toBeInTheDocument();
    expect(screen.getByTestId("blueprint-panel")).toBeInTheDocument();
    expect(screen.getByTestId("asset-panel")).toBeInTheDocument();
    expect(screen.queryByTestId("readiness-full")).not.toBeInTheDocument();
    expect(screen.queryByTestId("asset-health")).not.toBeInTheDocument();
  });

  it("把详细检查和项目环境拆成独立分区", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(screen.getByRole("tab", { name: "资产关联" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("tab", { name: "就绪与生效" }));
    expect(screen.getByTestId("readiness-full")).toBeInTheDocument();
    expect(screen.getByTestId("asset-health")).toBeInTheDocument();
    expect(screen.queryByTestId("asset-panel")).not.toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "项目环境 & Wiki" }));
    expect(screen.getByTestId("project-wiki")).toBeInTheDocument();
    expect(screen.getByTestId("project-environment")).toBeInTheDocument();
    expect(screen.queryByTestId("readiness-full")).not.toBeInTheDocument();
  });

  it("P0 回归：抽屉不得再挂第二份配置面板", () => {
    renderWithProviders(
      <ProjectDetailSheet
        view={makeProjectView(ALPHA, { progress: 50 })}
        aiConfigured={false}
        onStageChange={vi.fn()}
        onProgressChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByTestId("blueprint-panel")).toBeNull();
    expect(screen.queryByTestId("asset-panel")).toBeNull();
    expect(screen.queryByTestId("flow-panel")).toBeNull();
    // 抽屉只留 compact 就绪度：报缺口，配置去配置页
    expect(screen.getByTestId("readiness-compact")).toBeInTheDocument();
    expect(screen.queryByTestId("readiness-full")).toBeNull();
    // Tab 栏随第二个 Tab 一起消失 —— 只剩一种形态就不该还有切换器
    expect(screen.queryByRole("tablist")).toBeNull();
  });

  /**
   * 工作流编排在配置页上是**一个链接**，不是第四块面板。
   * `ProjectFlowOrchestratorPanel` 已经是「流程与方法论 → 工作流配置」的主体，
   * 在这里再挂一份就是第三个挂载点 —— 正是上面那条回归要防的东西。
   */
  it("工作流入口归到页头，带当前项目打开唯一的编排主体", async () => {
    const user = userEvent.setup();
    const { onOpenProjectWorkflow } = renderPage();

    expect(screen.queryByTestId("flow-panel")).toBeNull();
    await user.click(screen.getByRole("button", { name: "配置项目工作流" }));
    expect(onOpenProjectWorkflow).toHaveBeenCalledWith(ALPHA.id);

    await user.click(screen.getByRole("tab", { name: "项目环境 & Wiki" }));
    expect(screen.queryByText("工作流编排")).toBeNull();
  });
});

describe("项目资产配置：当前项目是共享状态", () => {
  it("页内切换器改的是全应用的当前项目，不是页内私有副本", async () => {
    const { onSelectProject } = renderPage();

    await userEvent.selectOptions(
      screen.getByLabelText("当前配置项目"),
      BETA.id,
    );

    expect(onSelectProject).toHaveBeenCalledWith(BETA.id);
  });

  it("从侧栏直接进来没选过项目时，替用户认领第一个", () => {
    const { onSelectProject } = renderPage({ selectedProjectId: null });

    expect(onSelectProject).toHaveBeenCalledWith(ALPHA.id);
  });

  it("一个项目都没有时给空态，而不是渲染一堆没有主语的面板", () => {
    renderPage({ projects: [], selectedProjectId: null });

    expect(screen.queryByTestId("blueprint-panel")).toBeNull();
    expect(screen.queryByTestId("asset-panel")).toBeNull();
    expect(
      screen.getByText("还没有添加项目，先添加一个再来配置 AI 资产。"),
    ).toBeInTheDocument();
  });
});
