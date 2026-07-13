import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { MethodologyPage } from "./MethodologyPage";

const sddApiMock = vi.hoisted(() => ({
  listDescriptors: vi.fn(),
  getAllSavedDetections: vi.fn(),
  detectAllProjects: vi.fn(),
  recommendPreset: vi.fn(),
}));

vi.mock("@/lib/api/sdd", () => ({ sddApi: sddApiMock }));
vi.mock("@/components/projects/ProjectFlowOrchestratorPanel", () => ({
  ProjectFlowOrchestratorPanel: ({ projectId }: { projectId: string }) => (
    <div>工作流面板：{projectId}</div>
  ),
}));
vi.mock("@/components/projects/ProjectRecipeComposer", () => ({
  ProjectRecipeComposer: ({ projectId }: { projectId: string }) => (
    <div>变更执行方案面板：{projectId}</div>
  ),
}));
vi.mock("@/components/projects/ProjectDesignContractPanel", () => ({
  default: ({ projectId }: { projectId: string }) => <div>设计面板：{projectId}</div>,
}));

const projects = [
  { id: "alpha", name: "Alpha", path: "C:/alpha", addedAt: "2026-01-01" },
  { id: "beta", name: "Beta", path: "C:/beta", addedAt: "2026-01-01" },
];

describe("MethodologyPage", () => {
  beforeEach(() => {
    sddApiMock.listDescriptors.mockResolvedValue([]);
    sddApiMock.getAllSavedDetections.mockResolvedValue({});
    sddApiMock.detectAllProjects.mockResolvedValue({});
    sddApiMock.recommendPreset.mockResolvedValue(null);
  });

  it("starts with workflow configuration and exposes one shared project context", async () => {
    render(<MethodologyPage projects={projects} />);

    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "工作流配置" })).toBeInTheDocument(),
    );
    expect(screen.getAllByLabelText("当前配置项目")).toHaveLength(1);
    expect((screen.getByLabelText("当前配置项目") as HTMLSelectElement).value).toBe("");
    expect(screen.queryByText("工作流面板：alpha")).not.toBeInTheDocument();
  });

  it("keeps the selected project when switching to change execution plans", async () => {
    const user = userEvent.setup();
    render(<MethodologyPage projects={projects} />);

    const projectSelect = await screen.findByLabelText("当前配置项目");
    await user.selectOptions(projectSelect, "beta");
    await user.click(screen.getByRole("tab", { name: /变更执行方案/i }));

    expect(screen.getByText("变更执行方案面板：beta")).toBeInTheDocument();
  });
});
