import { useState, useCallback } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";
import type { Project } from "@/types/project";

export function useKanbanRemoveProject(
  projects: Project[],
  onProjectRemove: (projectId: string) => void,
  t: TFunction,
  onRemoved?: (projectId: string) => void,
) {
  const [removeConfirm, setRemoveConfirm] = useState<{
    id: string;
    name: string;
  } | null>(null);

  const handleRemove = useCallback(
    (projectId: string) => {
      const project = projects.find((p) => p.id === projectId);
      const name = project?.name ?? projectId;
      setRemoveConfirm({ id: projectId, name });
    },
    [projects],
  );

  const confirmRemoveProject = useCallback(() => {
    if (!removeConfirm) return;
    const { id, name } = removeConfirm;
    onProjectRemove(id);
    onRemoved?.(id);
    toast.success(
      t("kanban.removed", {
        name,
        defaultValue: `已移除「${name}」`,
      }),
    );
    setRemoveConfirm(null);
  }, [removeConfirm, onProjectRemove, onRemoved, t]);

  const cancelRemove = useCallback(() => {
    setRemoveConfirm(null);
  }, []);

  return {
    removeConfirm,
    handleRemove,
    confirmRemoveProject,
    cancelRemove,
  };
}
