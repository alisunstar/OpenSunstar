import { useMemo, useCallback } from "react";
import { projectsApi } from "@/lib/api/projects";

type BoardProject = {
  id: string;
  stage?: string;
  mvp_progress?: number | null;
};

export function useProjectProgress(
  projects: BoardProject[],
  onProjectsReload: () => void | Promise<void>,
) {
  const progress = useMemo(() => {
    const map = new Map<string, number>();
    for (const project of projects) {
      if (
        project.mvp_progress !== null &&
        project.mvp_progress !== undefined &&
        project.mvp_progress > 0
      ) {
        map.set(project.id, project.mvp_progress);
      }
    }
    return map;
  }, [projects]);

  const getProgress = useCallback(
    (projectId: string): number | undefined => {
      const val = progress.get(projectId);
      return val !== undefined ? val : undefined;
    },
    [progress],
  );

  const setProjectProgress = useCallback(
    async (projectId: string, value: number) => {
      const clamped = Math.max(0, Math.min(100, Math.round(value)));
      const project = projects.find((p) => p.id === projectId);
      const stage = (project?.stage as "mvp" | "rapid" | "stable") ?? "mvp";
      const mvpProgress = clamped === 0 ? null : clamped;
      await projectsApi.updateBoardMetadata(projectId, stage, mvpProgress);
      await onProjectsReload();
      window.dispatchEvent(new Event("project-progress-changed"));
    },
    [projects, onProjectsReload],
  );

  return { progress, getProgress, setProjectProgress };
}
