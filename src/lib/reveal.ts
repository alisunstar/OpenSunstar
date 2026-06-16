//! 系统文件管理器操作 — 调用 Tauri Rust 后端

/**
 * 在系统文件管理器中打开并选中指定路径。
 */
export async function revealPathInFolder(
  path: string,
  options?: { alertOnError?: boolean },
): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("reveal_path_in_folder", { path });
    return;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    console.warn("[reveal] failed:", msg);

    // 降级：复制路径到剪贴板
    try {
      await navigator.clipboard.writeText(path);
      if (options?.alertOnError) {
        window.alert(`已复制路径：\n${path}`);
      }
    } catch {
      if (options?.alertOnError) {
        window.alert(`路径：\n${path}`);
      }
    }
  }
}
