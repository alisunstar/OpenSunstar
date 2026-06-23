import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useProjectMetricsScan } from "@/hooks/kanban/useProjectMetricsScan";
import type { Project } from "@/types/project";

const mockProject: Project = {
  id: "p1",
  name: "Demo",
  path: "/tmp/demo",
  addedAt: new Date().toISOString(),
};

vi.mock("@/api/codeMetrics", () => ({
  countProjectCodeLines: vi.fn(async () => ({
    total_lines: 100,
    code_lines: 80,
    comment_lines: 10,
    blank_lines: 10,
    files: 5,
    languages: [],
  })),
  readPackageVersion: vi.fn(async () => "1.0.0"),
  gitCommitCountLastNDays: vi.fn(async (_path: string, days: number) =>
    days === 7 ? 3 : 12,
  ),
  gitWeeklyCommitCounts: vi.fn(async () => [0, 0, 1, 2, 3, 4, 5]),
  gitContributors: vi.fn(async () => [{ name: "Alice", email: "a@x.com", commits: 5 }]),
}));

vi.mock("@/api/projectGit", () => ({
  detectProjectGitInfo: vi.fn(async () => ({
    is_repo: true,
    branch: "main",
  })),
}));

describe("useProjectMetricsScan", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("scans projects and stores 7-day commit counts", async () => {
    const { result, rerender } = renderHook(
      ({ projects }: { projects: Project[] }) => useProjectMetricsScan(projects),
      { initialProps: { projects: [mockProject] } },
    );

    await waitFor(() => {
      expect(result.current.scanning).toBe(false);
    });

    expect(result.current.commits7dMap.get("p1")).toBe(3);
    expect(result.current.commits30dMap.get("p1")).toBe(12);
    expect(result.current.codeLinesMap.get("p1")?.code_lines).toBe(80);

    rerender({ projects: [] });
    expect(result.current.scanning).toBe(false);
  });
});
