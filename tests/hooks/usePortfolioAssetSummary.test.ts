import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { usePortfolioAssetSummary } from "@/hooks/kanban/usePortfolioAssetSummary";
import { projectsApi } from "@/lib/api/projects";

vi.mock("@/lib/api/projects", () => ({
  projectsApi: {
    getAllAssetCounts: vi.fn(),
  },
}));

const project = {
  id: "p1",
  name: "Demo",
  path: "/demo",
  addedAt: new Date().toISOString(),
};

const projects = [project];

const emptyCounts = {
  mcp: 1,
  skills: 0,
  prompts: 0,
  commands: 0,
  hooks: 0,
  ignore: 0,
  permissions: 0,
  subagents: 0,
};

describe("usePortfolioAssetSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(projectsApi.getAllAssetCounts).mockResolvedValue(emptyCounts);
  });

  it("reloads when refreshToken changes", async () => {
    const { rerender } = renderHook(
      ({ token }) => usePortfolioAssetSummary(projects, token),
      { initialProps: { token: 0 } },
    );

    await waitFor(() => {
      expect(projectsApi.getAllAssetCounts).toHaveBeenCalledTimes(1);
    });

    rerender({ token: 1 });

    await waitFor(() => {
      expect(projectsApi.getAllAssetCounts).toHaveBeenCalledTimes(2);
    });
  });
});
