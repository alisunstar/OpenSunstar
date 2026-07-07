import type { StageKey } from "@/hooks/useProjectStages";
import type { Project } from "@/lib/api/projects";
import { projectsApi } from "@/lib/api/projects";

const STAGES_KEY = "OpenSunstar-project-stages";
const PROGRESS_KEY = "OpenSunstar-project-progress";
const MIGRATED_KEY = "OpenSunstar-board-metadata-db-v1";

function loadLegacyStages(): Map<string, StageKey> {
  try {
    const raw = localStorage.getItem(STAGES_KEY);
    if (!raw) return new Map();
    return new Map(JSON.parse(raw));
  } catch {
    return new Map();
  }
}

function loadLegacyProgress(): Map<string, number> {
  try {
    const raw = localStorage.getItem(PROGRESS_KEY);
    if (!raw) return new Map();
    return new Map(JSON.parse(raw));
  } catch {
    return new Map();
  }
}

/** 与 Rust `path_legacy_id` 对齐：SHA-256 前 8 字节 hex */
export async function pathLegacyId(projectPath: string): Promise<string> {
  const data = new TextEncoder().encode(projectPath);
  const hash = await crypto.subtle.digest("SHA-256", data);
  const bytes = new Uint8Array(hash).slice(0, 8);
  const hex = [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
  return `path_${hex}`;
}

function resolveLegacyStage(
  stages: Map<string, StageKey>,
  project: Project,
  legacyId: string,
): StageKey {
  return stages.get(project.id) ?? stages.get(legacyId) ?? "mvp";
}

function resolveLegacyProgress(
  progress: Map<string, number>,
  project: Project,
  legacyId: string,
): number | null {
  const value =
    progress.get(project.id) ?? progress.get(legacyId);
  if (value === undefined || value <= 0) return null;
  return Math.max(0, Math.min(100, Math.round(value)));
}

/**
 * 将看板阶段 / MVP 进度从 localStorage 一次性迁入 SQLite。
 */
export async function migrateBoardMetadataToDb(
  projects: Project[],
): Promise<void> {
  if (localStorage.getItem(MIGRATED_KEY)) return;
  if (projects.length === 0) return;

  const stages = loadLegacyStages();
  const progress = loadLegacyProgress();
  const hasLegacy = stages.size > 0 || progress.size > 0;

  if (!hasLegacy) {
    localStorage.setItem(MIGRATED_KEY, "1");
    return;
  }

  for (const project of projects) {
    const legacyId = await pathLegacyId(project.path);
    const stage = resolveLegacyStage(stages, project, legacyId);
    const mvpProgress =
      project.mvp_progress ??
      resolveLegacyProgress(progress, project, legacyId);

    try {
      await projectsApi.updateBoardMetadata(project.id, stage, mvpProgress);
    } catch (e) {
      console.warn("[migrateBoardMetadata] failed for", project.id, e);
    }
  }

  localStorage.removeItem(STAGES_KEY);
  localStorage.removeItem(PROGRESS_KEY);
  localStorage.setItem(MIGRATED_KEY, "1");
}
