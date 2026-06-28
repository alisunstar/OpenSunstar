import { useRef, useState, useEffect } from "react";

import type { Project } from "@/types/project";

import type { AIProviderConfig } from "@/api/aiInsight";



interface AgentReadinessBatchInput {
  projects: Project[];
  scanning: boolean;
  scanEpoch: number;
  portfolioRefreshToken?: number;
  getConfig: () => AIProviderConfig | null;
  targetApp?: string | null;
}



export function useAgentReadinessBatch({

  projects,

  scanning,

  scanEpoch,

  portfolioRefreshToken = 0,

  getConfig,

  targetApp,

}: AgentReadinessBatchInput) {

  const [agentReadinessMap, setAgentReadinessMap] = useState<

    Map<string, number>

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

      const results = await Promise.allSettled(

        projects.map(async (p) => {

          const r = await import("@/api/aiInsight").then((m) =>

            m.getAgentReadinessScore(p.path, config, forceRefresh, targetApp),

          );

          return { id: p.id, score: r?.score };

        }),

      );

      if (cancelled) return;

      const next = new Map<string, number>();

      for (const res of results) {

        if (res.status === "fulfilled" && typeof res.value.score === "number") {

          next.set(res.value.id, res.value.score);

        }

      }

      setAgentReadinessMap(next);

      setLoading(false);

    };



    void fetchReadiness();

    return () => {

      cancelled = true;

    };

  }, [scanning, projects, scanEpoch, portfolioRefreshToken, getConfig, targetApp]);



  return { agentReadinessMap, loading };

}

