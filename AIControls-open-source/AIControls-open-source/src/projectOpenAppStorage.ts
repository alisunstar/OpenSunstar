import { useSyncExternalStore } from "react";
import {
  normalizeProjectPath,
  pathsReferToSameDir,
} from "./projectPathsStorage";

const STORAGE_KEY = "aicontrols:projectOpenApps";
const CHANGE_EVENT = "aicontrols:project-open-apps-changed";

let cachedRaw: string | null = null;
let cachedMap: ReadonlyMap<string, string> = new Map();

function parseMap(raw: string): Map<string, string> {
  const next = new Map<string, string>();
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      return next;
    }
    for (const [k, v] of Object.entries(parsed)) {
      if (typeof k === "string" && typeof v === "string" && v.trim().length > 0) {
        next.set(normalizeProjectPath(k), v.trim());
      }
    }
  } catch {
    /* ignore */
  }
  return next;
}

export function readProjectOpenAppsMap(): ReadonlyMap<string, string> {
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

export function getOpenAppForProject(projectPath: string): string | undefined {
  const norm = normalizeProjectPath(projectPath);
  const m = readProjectOpenAppsMap();
  for (const [k, v] of m) {
    if (pathsReferToSameDir(k, norm)) return v;
  }
  return undefined;
}

/** `null` 清除该项目自定义打开方式，恢复系统默认。 */
export function setOpenAppForProject(
  projectPath: string,
  applicationPath: string | null,
): void {
  if (typeof localStorage === "undefined") return;
  const norm = normalizeProjectPath(projectPath);
  const prev = readProjectOpenAppsMap();
  const next = new Map(prev);

  for (const key of next.keys()) {
    if (pathsReferToSameDir(key, norm)) {
      next.delete(key);
      break;
    }
  }

  if (applicationPath != null && applicationPath.trim().length > 0) {
    next.set(norm, applicationPath.trim());
  }

  try {
    if (next.size === 0) {
      localStorage.removeItem(STORAGE_KEY);
      cachedRaw = "";
    } else {
      const obj: Record<string, string> = {};
      for (const [k, v] of next) obj[k] = v;
      const raw = JSON.stringify(obj);
      localStorage.setItem(STORAGE_KEY, raw);
      cachedRaw = raw;
    }
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

export function useProjectOpenAppsMap(): ReadonlyMap<string, string> {
  return useSyncExternalStore(
    subscribe,
    readProjectOpenAppsMap,
    readProjectOpenAppsMap,
  );
}
