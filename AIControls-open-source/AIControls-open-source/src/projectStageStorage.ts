import { useSyncExternalStore } from "react";
import {
  normalizeProjectPath,
  pathsReferToSameDir,
} from "./projectPathsStorage";

export type StageKey = "mvp" | "rapid" | "stable";

const STORAGE_KEY = "aicontrols:projectStages";
const CHANGE_EVENT = "aicontrols:project-stages-changed";

let cachedRaw: string | null = null;
let cachedMap: ReadonlyMap<string, StageKey> = new Map();

function parseMap(raw: string): Map<string, StageKey> {
  const next = new Map<string, StageKey>();
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      return next;
    }
    for (const [k, v] of Object.entries(parsed)) {
      if (
        typeof k === "string" &&
        (v === "mvp" || v === "rapid" || v === "stable")
      ) {
        next.set(normalizeProjectPath(k), v);
      }
    }
  } catch {
    /* ignore */
  }
  return next;
}

export function readProjectStagesMap(): ReadonlyMap<string, StageKey> {
  if (typeof localStorage === "undefined") return cachedMap;
  try {
    const raw = localStorage.getItem(STORAGE_KEY) ?? "";
    if (raw === cachedRaw) return cachedMap;
    cachedRaw = raw;
    cachedMap = raw ? parseMap(raw) : new Map();
    return cachedMap;
  } catch {
    cachedRaw = null;
    cachedMap = new Map();
    return cachedMap;
  }
}

export function getStageForProject(projectPath: string): StageKey {
  const norm = normalizeProjectPath(projectPath);
  const m = readProjectStagesMap();
  for (const [k, v] of m) {
    if (pathsReferToSameDir(k, norm)) return v;
  }
  return "mvp";
}

export function setStageForProject(
  projectPath: string,
  stage: StageKey,
): void {
  if (typeof localStorage === "undefined") return;
  const norm = normalizeProjectPath(projectPath);
  const prev = readProjectStagesMap();
  const next = new Map(prev);

  for (const key of next.keys()) {
    if (pathsReferToSameDir(key, norm)) {
      next.delete(key);
      break;
    }
  }

  next.set(norm, stage);

  try {
    const obj: Record<string, StageKey> = {};
    for (const [k, v] of next) obj[k] = v;
    const raw = JSON.stringify(obj);
    localStorage.setItem(STORAGE_KEY, raw);
    cachedRaw = raw;
    cachedMap = next;
  } catch {
    return;
  }
  window.dispatchEvent(new Event(CHANGE_EVENT));
}

function subscribe(onChange: () => void): () => void {
  const handler = () => onChange();
  window.addEventListener(CHANGE_EVENT, handler);
  window.addEventListener("storage", handler);
  return () => {
    window.removeEventListener(CHANGE_EVENT, handler);
    window.removeEventListener("storage", handler);
  };
}

export function useProjectStagesMap(): ReadonlyMap<string, StageKey> {
  return useSyncExternalStore(subscribe, readProjectStagesMap, readProjectStagesMap);
}
