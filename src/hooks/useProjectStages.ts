import { useMemo, useCallback } from "react";
import { projectsApi } from "@/lib/api/projects";

export type StageKey = "mvp" | "rapid" | "stable";

type BoardProject = {
  id: string;
  stage?: string;
  mvp_progress?: number | null;
};

export function useProjectStages(
  projects: BoardProject[],
  onProjectsReload: () => void | Promise<void>,
) {
  const stages = useMemo(() => {
    const map = new Map<string, StageKey>();
    for (const project of projects) {
      const stage = (project.stage as StageKey | undefined) ?? "mvp";
      map.set(project.id, stage);
    }
    return map;
  }, [projects]);

  const getStage = useCallback(
    (projectId: string): StageKey => stages.get(projectId) ?? "mvp",
    [stages],
  );

  const setStage = useCallback(
    async (projectId: string, stage: StageKey) => {
      const project = projects.find((p) => p.id === projectId);
      const mvpProgress = project?.mvp_progress ?? null;
      await projectsApi.updateBoardMetadata(projectId, stage, mvpProgress);
      await onProjectsReload();
      window.dispatchEvent(new Event("project-stages-changed"));
    },
    [projects, onProjectsReload],
  );

  return { stages, getStage, setStage };
}
