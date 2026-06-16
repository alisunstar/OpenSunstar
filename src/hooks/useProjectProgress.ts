import { useState, useCallback, useEffect } from "react";

const STORAGE_KEY = "OpenSunstar-project-progress";

function loadProgress(): Map<string, number> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Map();
    return new Map(JSON.parse(raw));
  } catch {
    return new Map();
  }
}

function saveProgress(map: Map<string, number>): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...map]));
}

export function useProjectProgress() {
  const [progress, setProgress] = useState<Map<string, number>>(() => loadProgress());

  useEffect(() => {
    const handler = () => setProgress(loadProgress());
    window.addEventListener("project-progress-changed", handler);
    return () => window.removeEventListener("project-progress-changed", handler);
  }, []);

  const getProgress = useCallback(
    (projectId: string): number | undefined => {
      const val = progress.get(projectId);
      return val !== undefined ? val : undefined; // 未设置 → undefined，不显示进度条
    },
    [progress],
  );

  const setProjectProgress = useCallback(
    (projectId: string, value: number) => {
      const clamped = Math.max(0, Math.min(100, Math.round(value)));
      const next = new Map(progress);
      if (clamped === 0) {
        next.delete(projectId);
      } else {
        next.set(projectId, clamped);
      }
      saveProgress(next);
      setProgress(next);
      window.dispatchEvent(new Event("project-progress-changed"));
    },
    [progress],
  );

  return { progress, getProgress, setProjectProgress };
}
