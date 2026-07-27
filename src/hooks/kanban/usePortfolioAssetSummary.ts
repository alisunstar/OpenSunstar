import { useCallback, useEffect, useMemo, useState } from "react";

import type { Project } from "@/types/project";

import { projectsApi } from "@/lib/api/projects";

import type { ProjectAllAssetCounts } from "@/types/projectAsset";

export type ProjectAssetCounts = ProjectAllAssetCounts;

export type PortfolioAssetSummaryMap = Map<string, ProjectAssetCounts>;

/**
 * 把异常压成一行可展示的原因。
 *
 * 刻意保留原始 message 而不是统一替换成「加载失败，请重试」：审查报告 §5.6
 * 指出全站错误都被折叠成同一句话，429 限流、401 密钥失效、IPC 超时在界面上
 * 无法区分。原因串由这里产出，人话由调用方（UI）补。
 */
function describeError(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err) return err;
  return String(err);
}

async function loadAssetMap(
  projects: Project[],
): Promise<PortfolioAssetSummaryMap> {
  const results = await Promise.all(
    projects.map(async (project) => {
      const counts = await projectsApi.getAllAssetCounts(project.id);

      return { id: project.id, counts };
    }),
  );

  const next = new Map<string, ProjectAssetCounts>();

  for (const row of results) {
    next.set(row.id, row.counts);
  }

  return next;
}

export function usePortfolioAssetSummary(
  projects: Project[],

  refreshToken = 0,
) {
  const [assetMap, setAssetMap] = useState<PortfolioAssetSummaryMap>(new Map());

  const [loading, setLoading] = useState(false);

  /**
   * 加载失败的原因；null 表示「这一轮数据是可信的」。
   *
   * 之前异常只进 `console.warn`，assetMap 保持为空 —— 矩阵满屏「未扫」，
   * 与「确实没配置」在界面上完全一样（审查报告 §5.6）。
   */
  const [error, setError] = useState<string | null>(null);

  const [lastUpdatedAt, setLastUpdatedAt] = useState<number | null>(null);

  const projectIdsKey = useMemo(
    () => projects.map((p) => p.id).join("\0"),

    [projects],
  );

  const refresh = useCallback(async () => {
    if (projects.length === 0) {
      setAssetMap(new Map());

      setError(null);

      return;
    }

    setLoading(true);

    try {
      setAssetMap(await loadAssetMap(projects));

      setLastUpdatedAt(Date.now());

      setError(null);
    } catch (err) {
      console.warn("[usePortfolioAssetSummary] load failed", err);

      setError(describeError(err));
    } finally {
      setLoading(false);
    }
  }, [projects]);

  useEffect(() => {
    let cancelled = false;

    const run = async () => {
      if (projects.length === 0) {
        setAssetMap(new Map());

        setError(null);

        return;
      }

      setLoading(true);

      try {
        const next = await loadAssetMap(projects);

        if (!cancelled) {
          setAssetMap(next);

          setLastUpdatedAt(Date.now());

          setError(null);
        }
      } catch (err) {
        console.warn("[usePortfolioAssetSummary] load failed", err);

        if (!cancelled) setError(describeError(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void run();

    return () => {
      cancelled = true;
    };
  }, [projectIdsKey, refreshToken, projects]);

  return { assetMap, loading, error, lastUpdatedAt, refresh };
}
