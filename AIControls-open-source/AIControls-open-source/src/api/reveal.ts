import { invoke } from "@tauri-apps/api/core";

function formatInvokeError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

/** 在系统文件管理器中展示该路径（桌面端）；浏览器或无权限时静默失败。 */
export async function revealPathInFolder(
  path: string,
  options?: { alertOnError?: boolean },
): Promise<void> {
  try {
    await invoke("reveal_path_in_folder", { path });
  } catch (e) {
    if (options?.alertOnError) {
      window.alert(`无法打开所在目录：${formatInvokeError(e)}`);
    }
    /* Web 预览或路径无效 */
  }
}
