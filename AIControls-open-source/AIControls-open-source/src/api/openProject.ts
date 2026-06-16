import { invoke } from "@tauri-apps/api/core";

function formatInvokeError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

/**
 * 用用户为该项目指定的应用打开项目目录；未指定时桌面端依次尝试 VS Code、Cursor，再打开所在文件夹。
 */
export async function openProjectPath(
  path: string,
  options?: {
    /** 应用程序路径（如 `/Applications/Cursor.app`）或可执行文件；不传则走 VS Code → Cursor → 文件夹。 */
    applicationPath?: string | null;
    alertOnError?: boolean;
  },
): Promise<void> {
  const applicationPath = options?.applicationPath?.trim() || null;
  try {
    await invoke("open_project_path", {
      path,
      applicationPath,
    });
  } catch (e) {
    if (options?.alertOnError) {
      window.alert(`无法打开项目：${formatInvokeError(e)}`);
    }
  }
}
