import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { Project } from "@/types/project";
import { ProjectDetailSheet } from "@/components/kanban/ProjectDetailSheet";
import { makeProjectView } from "../../../tests/projectViewFactory";
import { renderWithProviders } from "../../../tests/renderWithProviders";

// 必须 forwardRef：抽屉的键盘监听挂在 `sheetRef.current` 上
// （ProjectDetailSheet.tsx:93-95 `if (!sheet) return;`）。
// 普通函数组件收不到 ref，整个 useEffect 会被静默跳过，测试就会假绿。
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

// 用一个最小替身模拟真实结构：修复确认框由 AgentReadinessPanel 在抽屉**内部**渲染
// （AgentReadinessPanel.tsx:455-466），走的是 Radix Dialog 的 portal。
vi.mock("@/components/kanban/AgentReadinessPanel", async () => {
  const React = await import("react");
  const { ConfirmDialog } = await import("@/components/ConfirmDialog");
  return {
    AgentReadinessPanel: () => {
      const [open, setOpen] = React.useState(false);
      return (
        <div>
          <button type="button" onClick={() => setOpen(true)}>
            打开修复确认
          </button>
          <ConfirmDialog
            isOpen={open}
            title="确认修复配置漂移"
            message="可能覆盖你在 IDE/终端里手动改过的内容"
            confirmText="确认修复"
            cancelText="取消"
            zIndex="top"
            onConfirm={() => setOpen(false)}
            onCancel={() => setOpen(false)}
          />
        </div>
      );
    },
  };
});

vi.mock("@/components/kanban/AIRiskAnalysis", () => ({
  AIRiskAnalysis: () => <div data-testid="risk" />,
}));
vi.mock("@/components/kanban/CommitTrendChart", () => ({
  CommitTrendChart: () => <div data-testid="commit-trend" />,
}));
vi.mock("@/components/projects/ProjectAssetPanel", () => ({
  ProjectAssetPanel: () => <div data-testid="asset-panel" />,
}));
vi.mock("@/components/projects/ProjectBlueprintPanel", () => ({
  ProjectBlueprintPanel: () => <div data-testid="blueprint-panel" />,
}));
vi.mock("@/components/projects/ProjectFlowOrchestratorPanel", () => ({
  ProjectFlowOrchestratorPanel: () => <div data-testid="flow-panel" />,
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

const PROJECT: Project = {
  id: "p1",
  name: "alpha",
  path: "E:/projects/alpha",
  addedAt: new Date("2026-07-01").toISOString(),
};

function renderSheet(onClose: () => void) {
  return renderWithProviders(
    <ProjectDetailSheet
      view={makeProjectView(PROJECT, { progress: 50 })}
      aiConfigured={false}
      onStageChange={vi.fn()}
      onProgressChange={vi.fn()}
      onClose={onClose}
    />,
  );
}

describe("ProjectDetailSheet Esc 键守卫", () => {
  it("P0 回归：子确认框打开时按 Esc 只关子框，不得连抽屉一起关掉", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderSheet(onClose);

    await user.click(screen.getByRole("button", { name: "打开修复确认" }));
    expect(screen.getByText("确认修复配置漂移")).toBeInTheDocument();

    await user.keyboard("{Escape}");

    // 「取消这次高危修复」和「关掉整个抽屉」是两件事，一次 Esc 只能做前者。
    expect(onClose).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.queryByText("确认修复配置漂移")).not.toBeInTheDocument(),
    );
  });

  it("没有子对话框时 Esc 仍然关闭抽屉", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderSheet(onClose);

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("子确认框关闭后，再按一次 Esc 才关闭抽屉", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderSheet(onClose);

    await user.click(screen.getByRole("button", { name: "打开修复确认" }));
    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByText("确认修复配置漂移")).not.toBeInTheDocument(),
    );

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
