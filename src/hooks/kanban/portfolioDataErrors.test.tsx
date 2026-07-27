import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useAgentReadinessBatch } from "@/hooks/kanban/useAgentReadinessBatch";
import { usePortfolioAssetSummary } from "@/hooks/kanban/usePortfolioAssetSummary";
import type { Project } from "@/types/project";

/**
 * 「失败 ≠ 零值」回归（审查报告 §5.6）。
 *
 * 两个组合层 hook 都把异常吞进 `console.warn` 就算完：
 * - `usePortfolioAssetSummary.ts:64/91` —— 扫描失败后 assetMap 保持空，
 *   矩阵满屏「未扫」，和「确实没配置」视觉上完全一样。
 * - `useAgentReadinessBatch.ts:80-84` —— rejected 直接丢弃；更常见的是
 *   API 层 `catch → return null`（aiInsight.ts）让 `r` 为 null，
 *   在 `:63` 被静默跳过，那个项目就变成「未扫描」。
 *
 * 「不知道」和「已知为 0/未配置」必须是两种可区分的状态，否则用户会
 * 拿网络故障当治理结论。
 */

const getAllAssetCounts = vi.fn();
vi.mock("@/lib/api/projects", () => ({
  projectsApi: {
    getAllAssetCounts: (id: string) => getAllAssetCounts(id),
  },
}));

const getAgentReadinessScore = vi.fn();
vi.mock("@/api/aiInsight", () => ({
  getAgentReadinessScore: (...args: unknown[]) =>
    getAgentReadinessScore(...args),
}));

function project(id: string): Project {
  return { id, name: id, path: `E:/repos/${id}` } as Project;
}

const EMPTY_COUNTS = { mcp: 0, skills: 0, prompts: 0, subagents: 0 };

beforeEach(() => {
  getAllAssetCounts.mockReset();
  getAgentReadinessScore.mockReset();
});

describe("usePortfolioAssetSummary 错误分支", () => {
  it("加载失败必须暴露 error，而不是留下一个空 Map 冒充「都没配置」", async () => {
    getAllAssetCounts.mockRejectedValue(new Error("ipc timeout"));
    const projects = [project("alpha")];

    const { result } = renderHook(() => usePortfolioAssetSummary(projects));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBeTruthy();
    expect(result.current.assetMap.size).toBe(0);
  });

  it("加载成功时 error 必须为 null", async () => {
    getAllAssetCounts.mockResolvedValue(EMPTY_COUNTS);
    const projects = [project("alpha")];

    const { result } = renderHook(() => usePortfolioAssetSummary(projects));

    await waitFor(() => expect(result.current.assetMap.size).toBe(1));
    expect(result.current.error).toBeNull();
  });

  it("失败后重试成功必须清掉 error（否则告警会永久黏住）", async () => {
    getAllAssetCounts.mockRejectedValueOnce(new Error("ipc timeout"));
    const projects = [project("alpha")];

    const { result } = renderHook(() => usePortfolioAssetSummary(projects));
    await waitFor(() => expect(result.current.error).toBeTruthy());

    getAllAssetCounts.mockResolvedValue(EMPTY_COUNTS);
    await result.current.refresh();

    await waitFor(() => expect(result.current.error).toBeNull());
    expect(result.current.assetMap.size).toBe(1);
  });
});

describe("useAgentReadinessBatch 错误分支", () => {
  const READY = {
    score: 80,
    details: [],
    evaluated_at: 1_700_000_000,
    assessment_state: "managed",
  };

  // `getConfig` 是 useEffect 的依赖项。真实的 `useAIConfig.ts:22-24` 用
  // `useCallback([])` 保证了它跨渲染稳定；测试里若每次渲染新建一个箭头函数，
  // 就会造出真实环境不存在的 effect 死循环。
  const GET_CONFIG = () => null;
  const PROJECTS = [project("alpha"), project("beta")];

  function setup(projects: Project[] = PROJECTS) {
    return renderHook(() =>
      useAgentReadinessBatch({
        projects,
        scanning: false,
        scanEpoch: 1,
        getConfig: GET_CONFIG,
      }),
    );
  }

  it("API 返回 null（catch 后的静默失败）必须计入 failedCount", async () => {
    getAgentReadinessScore.mockImplementation((path: string) =>
      Promise.resolve(path.endsWith("beta") ? null : READY),
    );

    const { result } = setup();

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.agentReadinessMap.size).toBe(1);
    expect(result.current.failedCount).toBe(1);
  });

  it("rejected 的项目同样必须计入 failedCount", async () => {
    getAgentReadinessScore.mockImplementation((path: string) =>
      path.endsWith("beta")
        ? Promise.reject(new Error("scan crashed"))
        : Promise.resolve(READY),
    );

    const { result } = setup();

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.agentReadinessMap.size).toBe(1);
    expect(result.current.failedCount).toBe(1);
  });

  it("全部成功时 failedCount 必须归零", async () => {
    getAgentReadinessScore.mockResolvedValue(READY);

    const { result } = setup();

    await waitFor(() => expect(result.current.agentReadinessMap.size).toBe(2));
    expect(result.current.failedCount).toBe(0);
  });
});
