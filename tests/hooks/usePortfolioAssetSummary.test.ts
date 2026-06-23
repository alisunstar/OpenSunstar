import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { usePortfolioAssetSummary } from "@/hooks/kanban/usePortfolioAssetSummary";
import { projectsApi } from "@/lib/api/projects";

vi.mock("@/lib/api/projects", () => ({
  projectsApi: {
    getMcpServers: vi.fn(),
    getSkills: vi.fn(),
    getPrompts: vi.fn(),
  },
}));

const project = {
  id: "p1",
  name: "Demo",
  path: "/demo",
  addedAt: new Date().toISOString(),
};

const projects = [project];

describe("usePortfolioAssetSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(projectsApi.getMcpServers).mockResolvedValue([
      { project_id: "p1", config_id: "m1", enabled: true, created_at: 1 },
    ]);
    vi.mocked(projectsApi.getSkills).mockResolvedValue([]);
    vi.mocked(projectsApi.getPrompts).mockResolvedValue([]);
  });

  it("reloads when refreshToken changes", async () => {
    const { rerender } = renderHook(
      ({ token }) => usePortfolioAssetSummary(projects, token),
      { initialProps: { token: 0 } },
    );

    await waitFor(() => {
      expect(projectsApi.getMcpServers).toHaveBeenCalledTimes(1);
    });

    rerender({ token: 1 });

    await waitFor(() => {
      expect(projectsApi.getMcpServers).toHaveBeenCalledTimes(2);
    });
  });
});
