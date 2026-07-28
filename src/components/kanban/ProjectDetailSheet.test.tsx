import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
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

let readinessProps: Record<string, unknown> = {};
vi.mock("@/components/kanban/AgentReadinessPanel", () => ({
  AgentReadinessPanel: (props: Record<string, unknown>) => {
    readinessProps = props;
    return (
      <button
        type="button"
        onClick={() =>
          (props.onOpenProjectAssets as (() => void) | undefined)?.()
        }
      >
        打开项目资产配置
      </button>
    );
  },
}));

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

function renderSheet(onClose: () => void, onOpenAiConfig = vi.fn()) {
  return renderWithProviders(
    <ProjectDetailSheet
      view={makeProjectView(PROJECT, { progress: 50 })}
      aiConfigured={false}
      onStageChange={vi.fn()}
      onProgressChange={vi.fn()}
      onClose={onClose}
      onOpenAiConfig={onOpenAiConfig}
    />,
  );
}

describe("ProjectDetailSheet 就绪度只保留摘要", () => {
  it("不再把扫描、修复和完整配置能力传进抽屉", () => {
    const onClose = vi.fn();
    renderSheet(onClose);

    expect(readinessProps.compact).toBe(true);
    expect(readinessProps.onScanEffective).toBeUndefined();
    expect(readinessProps.onRepairDrift).toBeUndefined();
    expect(readinessProps.onRefresh).toBeUndefined();
    expect(readinessProps.onNavigate).toBeUndefined();
  });

  it("唯一 CTA 打开项目资产配置并关闭抽屉", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onOpenAiConfig = vi.fn();
    renderSheet(onClose, onOpenAiConfig);

    await user.click(screen.getByRole("button", { name: "打开项目资产配置" }));

    expect(onOpenAiConfig).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("ProjectDetailSheet Esc 键守卫", () => {
  it("按 Esc 关闭抽屉", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderSheet(onClose);

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
