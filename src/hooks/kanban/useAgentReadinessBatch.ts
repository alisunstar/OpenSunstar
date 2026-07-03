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
            } satisfies AgentReadinessBatchEntry,
          };
        }),
      );

      if (cancelled) return;

      const next = new Map<string, AgentReadinessBatchEntry>();
      for (const res of results) {
        if (res.status === "fulfilled" && res.value.entry) {
          next.set(res.value.id, res.value.entry);
        }
      }

      setAgentReadinessMap(next);
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

  return { agentReadinessMap, loading };
}
