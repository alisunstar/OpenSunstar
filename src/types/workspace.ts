export type WorkspaceTab = "dashboard" | "board" | "assetsMatrix";

export const WORKSPACE_TAB_STORAGE_KEY = "OpenSunstar-workspace-tab";

export const WORKSPACE_TABS: WorkspaceTab[] = [
  "dashboard",
  "board",
  "assetsMatrix",
];

export function getInitialWorkspaceTab(): WorkspaceTab {
  try {
    const saved = localStorage.getItem(WORKSPACE_TAB_STORAGE_KEY);
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
