import { invoke } from "@tauri-apps/api/core";

function formatInvokeError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

/** 递归读取目录下最新修改时间（Unix 毫秒）；桌面端失败时返回 null。 */
export async function getProjectLatestMtimeMs(root: string): Promise<number | null> {
  try {
    return await invoke<number>("get_project_latest_mtime_ms", { root });
  } catch (e) {
    // Web 预览/无权限/路径无效等
    console.warn("[getProjectLatestMtimeMs] failed:", formatInvokeError(e));
    return null;
  }
}

