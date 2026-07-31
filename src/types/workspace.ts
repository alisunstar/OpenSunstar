/**
 * 工作区 Tab（工作区重构 2026-07-30）。
 *
 * 三砍二：「AI 资产总览」（assetsMatrix）并入「项目看板」——两者同为
 * 项目维度的巡视内容，分两个 Tab 是信息超载。「今日工作台」改为告警制
 * 首屏后，这条边界变成：今日答「有没有事」，看板答「项目们怎么样」。
 */
export type WorkspaceTab = "dashboard" | "board";

export const WORKSPACE_TAB_STORAGE_KEY = "OpenSunstar-workspace-tab";

export const WORKSPACE_TABS: WorkspaceTab[] = ["dashboard", "board"];

export function getInitialWorkspaceTab(): WorkspaceTab {
  try {
    const saved = localStorage.getItem(WORKSPACE_TAB_STORAGE_KEY);
    // 旧值 assetsMatrix 平滑迁移到 board：老用户升级后不落空，
    // 他们要的内容（治理面板 + 资产矩阵）就在这一屏里。
    if (saved === "assetsMatrix") return "board";
    if (saved && WORKSPACE_TABS.includes(saved as WorkspaceTab)) {
      return saved as WorkspaceTab;
    }
  } catch {
    /* ignore */
  }
  return "dashboard";
}

export function persistWorkspaceTab(tab: WorkspaceTab): void {
  try {
    localStorage.setItem(WORKSPACE_TAB_STORAGE_KEY, tab);
  } catch {
    /* ignore */
  }
}
