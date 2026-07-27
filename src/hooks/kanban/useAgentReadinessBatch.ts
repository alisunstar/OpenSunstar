import { useRef, useState, useEffect } from "react";

import type { Project } from "@/types/project";

import type { AIProviderConfig } from "@/api/aiInsight";
import {
  countDriftItems,
  pickScannedAt,
  type AgentReadinessBatchEntry,
} from "@/lib/readinessBatch";

interface AgentReadinessBatchInput {
  projects: Project[];
  scanning: boolean;
  scanEpoch: number;
  portfolioRefreshToken?: number;
  getConfig: () => AIProviderConfig | null;
  targetApp?: string | null;
  /** 组合层默认开启生效态扫描（S2-01） */
  scanEffective?: boolean;
}

export function useAgentReadinessBatch({
  projects,
  scanning,
  scanEpoch,
  portfolioRefreshToken = 0,
  getConfig,
  targetApp,
  scanEffective = true,
}: AgentReadinessBatchInput) {
  const [agentReadinessMap, setAgentReadinessMap] = useState<
    Map<string, AgentReadinessBatchEntry>
  >(new Map());
  const [loading, setLoading] = useState(false);
  /**
   * 本轮扫描中「读不到」的项目数。
   *
   * 两条失败路径以前都是静默的（审查报告 §5.6）：
   * - `getAgentReadinessScore` 抛异常 → `Promise.allSettled` 的 rejected 被丢弃
   * - API 层 `catch → return null`（这是更常见的一条）→ `r` 为 null 被跳过
   *
   * 两种情况下那个项目在界面上都变成「未扫描」，和「真的还没扫」一模一样。
   * 计数不为 0 时调用方必须提示「下面的判定不完整」。
   */
  const [failedCount, setFailedCount] = useState(0);
  const prevRefreshToken = useRef(portfolioRefreshToken);

  useEffect(() => {
    if (scanning || projects.length === 0) return;

    let cancelled = false;

    const forceRefresh =
      portfolioRefreshToken > 0 &&
      portfolioRefreshToken !== prevRefreshToken.current;

    prevRefreshToken.current = portfolioRefreshToken;

    const fetchReadiness = async () => {
      setLoading(true);
      const config = getConfig();
      const { getAgentReadinessScore } = await import("@/api/aiInsight");

      const results = await Promise.allSettled(
        projects.map(async (p) => {
          const r = await getAgentReadinessScore(
            p.path,
            config,
            forceRefresh,
            targetApp,
            scanEffective,
          );
          if (!r) return { id: p.id, entry: null };
          const details = r.details ?? [];
          return {
            id: p.id,
            entry: {
              score: r.score,
              driftCount: countDriftItems(details),
              scannedAt: pickScannedAt(r.evaluated_at, details),
              details,
              assessmentState: r.assessment_state ?? null,
            } satisfies AgentReadinessBatchEntry,
          };
        }),
      );

      if (cancelled) return;

      const next = new Map<string, AgentReadinessBatchEntry>();
      let failed = 0;
      for (const res of results) {
        if (res.status === "fulfilled" && res.value.entry) {
          next.set(res.value.id, res.value.entry);
        } else {
          failed += 1;
        }
      }

      setAgentReadinessMap(next);
      setFailedCount(failed);
      setLoading(false);
    };

    void fetchReadiness();

    return () => {
      cancelled = true;
    };
  }, [
    scanning,
    projects,
    scanEpoch,
    portfolioRefreshToken,
    getConfig,
    targetApp,
    scanEffective,
  ]);

  return { agentReadinessMap, loading, failedCount };
}
