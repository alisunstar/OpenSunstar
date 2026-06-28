import { useCallback, useEffect, useMemo, useState } from "react";

import type { Project } from "@/types/project";

import { projectsApi } from "@/lib/api/projects";

import type { ProjectAllAssetCounts } from "@/types/projectAsset";



export type ProjectAssetCounts = ProjectAllAssetCounts;



export type PortfolioAssetSummaryMap = Map<string, ProjectAssetCounts>;



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

  const [lastUpdatedAt, setLastUpdatedAt] = useState<number | null>(null);

  const projectIdsKey = useMemo(

    () => projects.map((p) => p.id).join("\0"),

    [projects],

  );



  const refresh = useCallback(async () => {

    if (projects.length === 0) {

      setAssetMap(new Map());

      return;

    }

    setLoading(true);

    try {

      setAssetMap(await loadAssetMap(projects));

      setLastUpdatedAt(Date.now());

    } catch (err) {

      console.warn("[usePortfolioAssetSummary] load failed", err);

    } finally {

      setLoading(false);

    }

  }, [projects]);



  useEffect(() => {

    let cancelled = false;



    const run = async () => {

      if (projects.length === 0) {

        setAssetMap(new Map());

        return;

      }

      setLoading(true);

      try {

        const next = await loadAssetMap(projects);

        if (!cancelled) {

          setAssetMap(next);

          setLastUpdatedAt(Date.now());

        }

      } catch (err) {

        console.warn("[usePortfolioAssetSummary] load failed", err);

      } finally {

        if (!cancelled) setLoading(false);

      }

    };



    void run();

    return () => {

      cancelled = true;

    };

  }, [projectIdsKey, refreshToken, projects]);



  return { assetMap, loading, lastUpdatedAt, refresh };

}

