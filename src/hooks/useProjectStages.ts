import { useState, useCallback, useEffect } from "react";

export type StageKey = "mvp" | "rapid" | "stable";

const STORAGE_KEY = "OpenSunstar-project-stages";

function loadStages(): Map<string, StageKey> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Map();
    return new Map(JSON.parse(raw));
  } catch {
    return new Map();
  }
}

function saveStages(map: Map<string, StageKey>): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...map]));
}

export function useProjectStages() {
  const [stages, setStages] = useState<Map<string, StageKey>>(() => loadStages());

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY) setStages(loadStages());
    };
    const handleCustom = () => setStages(loadStages());
    window.addEventListener("storage", handleStorage);
    window.addEventListener("project-stages-changed", handleCustom);
    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("project-stages-changed", handleCustom);
    };
  }, []);

  const getStage = useCallback(
    (projectId: string): StageKey => stages.get(projectId) ?? "mvp",
    [stages],
  );

  const setStage = useCallback((projectId: string, stage: StageKey) => {
    const next = new Map(stages);
    next.set(projectId, stage);
    saveStages(next);
    setStages(next);
    window.dispatchEvent(new Event("project-stages-changed"));
  }, [stages]);

  return { stages, getStage, setStage };
}
