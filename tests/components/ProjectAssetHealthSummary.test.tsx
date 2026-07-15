import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProjectAssetHealthSummary } from "@/components/projects/ProjectAssetHealthSummary";

const getHealthMock = vi.fn();
const planHealthMock = vi.fn();
const applyHealthMock = vi.fn();
const rollbackHealthMock = vi.fn();

vi.mock("@/lib/api/projects", () => ({
  projectsApi: {
    getAssetHealth: (...args: unknown[]) => getHealthMock(...args),
    planAssetHealth: (...args: unknown[]) => planHealthMock(...args),
    applyAssetHealthPlan: (...args: unknown[]) => applyHealthMock(...args),
    rollbackAssetHealthReceipt: (...args: unknown[]) =>
      rollbackHealthMock(...args),
  },
}));

describe("ProjectAssetHealthSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows a conservative breakdown and does not call unknown assets healthy", async () => {
    getHealthMock.mockResolvedValue([
      {
        expectation: {
          expectationId: "e1",
          assetType: "mcp",
          assetId: "m1",
          targetApp: "claude",
          source: "manual",
        },
        status: "healthy",
        evidenceLevel: "runtime_verified",
        reasonCode: "runtime_verified",
        lastReceiptFiles: [],
      },
      {
        expectation: {
          expectationId: "e2",
          assetType: "skill",
          assetId: "s1",
          targetApp: "codex",
          source: "manual",
        },
        status: "attention",
        evidenceLevel: "written",
        reasonCode: "deployment_unverified",
        lastReceiptFiles: [],
      },
      {
        expectation: {
          expectationId: "e3",
          assetType: "hook",
          assetId: "h1",
          targetApp: "opencode",
          source: "manual",
        },
        status: "unsupported",
        evidenceLevel: "none",
        reasonCode: "unsupported_combination",
        lastReceiptFiles: [],
      },
    ]);

    render(<ProjectAssetHealthSummary projectId="project-1" />);

    await waitFor(() => {
      expect(screen.getByText("符合验证策略 1")).toBeInTheDocument();
      expect(screen.getByText("待确认 1")).toBeInTheDocument();
      expect(screen.getByText("不支持 1")).toBeInTheDocument();
    });
    expect(getHealthMock).toHaveBeenCalledWith("project-1");
  });

  it("requires a visible plan preview before applying", async () => {
    getHealthMock.mockResolvedValue([
      {
        expectation: {
          expectationId: "e1",
          assetType: "mcp",
          assetId: "m1",
          targetApp: "claude",
          source: "manual",
        },
        status: "unknown",
        evidenceLevel: "none",
        reasonCode: "not_scanned",
        lastReceiptFiles: [],
      },
    ]);
    planHealthMock.mockResolvedValue({
      operationId: "operation",
      projectId: "project-1",
      planSha256: "digest",
      steps: [
        {
          expectationId: "e1",
          assetType: "mcp",
          assetId: "m1",
          targetApp: "claude",
          action: "legacy_project_sync",
          reasonCode: "not_scanned",
          adapterId: "adapter",
          writeMode: "project_file",
          verifyModes: ["config_parse"],
          limitations: [],
          managedPaths: [".mcp.json"],
          protectedPaths: [],
        },
      ],
    });

    render(<ProjectAssetHealthSummary projectId="project-1" />);
    const previewButton = await screen.findByRole("button", {
      name: "预览同步计划",
    });
    fireEvent.click(previewButton);

    expect(
      await screen.findByText("同步计划（预览阶段，尚未写入）"),
    ).toBeInTheDocument();
    expect(planHealthMock).toHaveBeenCalledWith("project-1");
    expect(applyHealthMock).not.toHaveBeenCalled();
  });
});
