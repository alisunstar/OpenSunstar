import { useState, useCallback } from "react";
import type { Project } from "@/types/project";

export type DupGroup = { reason: string; projects: Project[] };

export function useDuplicateProjectScan(projects: Project[]) {
  const [dupGroups, setDupGroups] = useState<DupGroup[] | null>(null);
  const [dupScanning, setDupScanning] = useState(false);

  const scanDuplicates = useCallback(() => {
    if (projects.length < 2) {
      setDupGroups([]);
      return;
    }
    setDupScanning(true);
    const groups: DupGroup[] = [];

    const byName = new Map<string, Project[]>();
    for (const p of projects) {
      const key = p.name.toLowerCase();
      if (!byName.has(key)) byName.set(key, []);
      byName.get(key)!.push(p);
    }
    for (const [, list] of byName) {
      if (list.length > 1) {
        groups.push({
          reason: `项目名「${list[0].name}」重复（${list.length} 个）`,
          projects: list,
        });
      }
    }

    const byPath = new Map<string, Project[]>();
    for (const p of projects) {
      const key = p.path.toLowerCase().replace(/\\/g, "/");
      if (!byPath.has(key)) byPath.set(key, []);
      byPath.get(key)!.push(p);
    }
    for (const [, list] of byPath) {
      if (list.length > 1) {
        const alreadyInName = groups.some((g) =>
          g.projects.some((gp) => list.some((lp) => lp.id === gp.id)),
        );
        if (!alreadyInName) {
          groups.push({
            reason: `路径「${list[0].path}」重复（${list.length} 个）`,
            projects: list,
          });
        }
      }
    }

    setDupGroups(groups);
    setDupScanning(false);
  }, [projects]);

  const removeFromDupGroups = useCallback((projectId: string) => {
    setDupGroups((prev) =>
      (prev ?? [])
        .map((g) => ({
          ...g,
          projects: g.projects.filter((gp) => gp.id !== projectId),
        }))
        .filter((g) => g.projects.length > 1),
    );
  }, []);

  return {
    dupGroups,
    dupScanning,
    scanDuplicates,
    removeFromDupGroups,
  };
}
