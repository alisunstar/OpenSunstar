import {
  getAgentGlobalInventory,
  scanProjectDirectory,
  type AgentInventory,
} from "./agents";
import { getProjectLatestMtimeMs } from "./projectMtime";
import { normalizeProjectPath } from "../projectPathsStorage";

const projectInvCache = new Map<string, AgentInventory | null>();
const projectInflight = new Map<string, Promise<AgentInventory | null>>();

const agentGlobalInvCache = new Map<string, AgentInventory | null>();
const agentGlobalInflight = new Map<string, Promise<AgentInventory | null>>();

const projectMtimeCache = new Map<string, number | null>();
const projectMtimeInflight = new Map<string, Promise<number | null>>();

/** 会话内缓存的项目扫描结果；切换路由复用，避免重复读盘。 */
export async function scanProjectDirectoryCached(
  root: string,
): Promise<AgentInventory | null> {
  const key = normalizeProjectPath(root);
  if (projectInvCache.has(key)) {
    return projectInvCache.get(key)!;
  }
  let p = projectInflight.get(key);
  if (!p) {
    p = scanProjectDirectory(root).then((inv) => {
      projectInvCache.set(key, inv);
      projectInflight.delete(key);
      return inv;
    });
    projectInflight.set(key, p);
  }
  return p;
}

/** 会话内缓存的项目「最新修改时间」；切换路由复用，避免重复读盘。 */
export async function getProjectLatestMtimeMsCached(
  root: string,
): Promise<number | null> {
  const key = normalizeProjectPath(root);
  if (projectMtimeCache.has(key)) {
    return projectMtimeCache.get(key)!;
  }
  let p = projectMtimeInflight.get(key);
  if (!p) {
    p = getProjectLatestMtimeMs(root).then((ms) => {
      projectMtimeCache.set(key, ms);
      projectMtimeInflight.delete(key);
      return ms;
    });
    projectMtimeInflight.set(key, p);
  }
  return p;
}

/** 会话内缓存的 Agent 用户全局目录扫描结果。 */
export async function getAgentGlobalInventoryCached(
  agentId: string,
): Promise<AgentInventory | null> {
  if (agentGlobalInvCache.has(agentId)) {
    return agentGlobalInvCache.get(agentId)!;
  }
  let p = agentGlobalInflight.get(agentId);
  if (!p) {
    p = getAgentGlobalInventory(agentId).then((inv) => {
      agentGlobalInvCache.set(agentId, inv);
      agentGlobalInflight.delete(agentId);
      return inv;
    });
    agentGlobalInflight.set(agentId, p);
  }
  return p;
}

export function invalidateCachedProjectInventory(root?: string): void {
  if (root === undefined) {
    projectInvCache.clear();
    projectInflight.clear();
    projectMtimeCache.clear();
    projectMtimeInflight.clear();
    return;
  }
  const key = normalizeProjectPath(root);
  projectInvCache.delete(key);
  projectInflight.delete(key);
  projectMtimeCache.delete(key);
  projectMtimeInflight.delete(key);
}

export function invalidateCachedAgentGlobalInventory(agentId?: string): void {
  if (agentId === undefined) {
    agentGlobalInvCache.clear();
    agentGlobalInflight.clear();
    return;
  }
  agentGlobalInvCache.delete(agentId);
  agentGlobalInflight.delete(agentId);
}
