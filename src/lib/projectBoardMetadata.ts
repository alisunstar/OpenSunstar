import type { StageKey } from "@/hooks/useProjectStages";

const STAGES_KEY = "OpenSunstar-project-stages";
const PROGRESS_KEY = "OpenSunstar-project-progress";

function loadStagesMap(): Map<string, StageKey> {
  try {
    const raw = localStorage.getItem(STAGES_KEY);
    if (!raw) return new Map();
    return new Map(JSON.parse(raw));
  } catch {
    return new Map();
  }
}

function loadProgressMap(): Map<string, number> {
  try {
    const raw = localStorage.getItem(PROGRESS_KEY);
    if (!raw) return new Map();
    return new Map(JSON.parse(raw));
  } catch {
    return new Map();
  }
}

/** 移除项目时清理看板 localStorage 元数据（阶段 / MVP 进度） */
export function clearProjectBoardMetadata(projectId: string): void {
  const stages = loadStagesMap();
  if (stages.delete(projectId)) {
    localStorage.setItem(STAGES_KEY, JSON.stringify([...stages]));
    window.dispatchEvent(new Event("project-stages-changed"));
  }

  const progress = loadProgressMap();
  if (progress.delete(projectId)) {
    localStorage.setItem(PROGRESS_KEY, JSON.stringify([...progress]));
    window.dispatchEvent(new Event("project-progress-changed"));
  }
}
