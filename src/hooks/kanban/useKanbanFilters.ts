import { useMemo, useState } from "react";
import type { Project } from "@/types/project";
import type { StageKey } from "@/hooks/useProjectStages";

export function useKanbanFilters(
  projects: Project[],
  getStage: (projectId: string) => StageKey,
) {
  const [searchQuery, setSearchQuery] = useState("");

  const filtered = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q),
    );
  }, [projects, searchQuery]);

  const grouped = useMemo(() => {
    const mvp = filtered.filter((p) => getStage(p.id) === "mvp");
    const rapid = filtered.filter((p) => getStage(p.id) === "rapid");
    const stable = filtered.filter((p) => getStage(p.id) === "stable");
    return { mvp, rapid, stable };
  }, [filtered, getStage]);

  const empty = projects.length === 0;
  const noResults = !empty && filtered.length === 0;

  return {
    searchQuery,
    setSearchQuery,
    filtered,
    grouped,
    empty,
    noResults,
  };
}
