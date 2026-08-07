import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ChevronRight } from "lucide-react";

import { AppPageActions, hasPageActions } from "@/app/AppPageActions";
import { AppPageRouter } from "@/app/AppPageRouter";
import {
  AGENT_ASSET_PAGE_TITLES,
  ALL_VISIBLE_APPS,
  APP_STORAGE_KEY,
  getInitialApp,
  getInitialView,
  isAgentConfigView,
  PAGE_META,
  VIEW_STORAGE_KEY,
  type MethodologyNavigationIntent,
  type PageView,
  type ProjectAiConfigNavigationIntent,
} from "@/app/navigation";
import { usePageActionRefs } from "@/app/pageActionRefs";
import { useAppKeyboardShortcuts } from "@/app/useAppKeyboardShortcuts";
import { AppSwitcher } from "@/components/AppSwitcher";
import { DeepLinkImportDialog } from "@/components/DeepLinkImportDialog";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { OnboardingWizard } from "@/components/onboarding/OnboardingWizard";
import { AddProjectDialog } from "@/components/projects/AddProjectDialog";
import { PageScopeBadge } from "@/components/shared/PageScopeBadge";
import { ShortcutsHelp } from "@/components/ShortcutsHelp";
import { Sidebar } from "@/components/layout/Sidebar";
import type { SkillsFocusIntent } from "@/components/skills/UnifiedSkillsPanel";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import { useBudgetAlerts } from "@/hooks/useBudgetAlerts";
import { useProjects } from "@/hooks/useProjects";
import { DRAG_REGION_ATTR, DRAG_REGION_STYLE } from "@/lib/platform";
import { useSettingsQuery } from "@/lib/query";
import {
  buildAiProviderSettingsIntent,
  type SettingsNavIntent,
} from "@/lib/settingsNavigation";
import {
  getInitialWorkspaceTab,
  persistWorkspaceTab,
  type WorkspaceTab,
} from "@/types/workspace";

export type { PageView } from "@/app/navigation";

const OVERLAY_TITLEBAR_SAFE_TOP = 34;
const DEFAULT_DRAG_BAR_HEIGHT = OVERLAY_TITLEBAR_SAFE_TOP;

function App() {
  const { t } = useTranslation();
  const [currentView, setCurrentView] = useState<PageView>(getInitialView);
  const [workspaceTab, setWorkspaceTab] = useState<WorkspaceTab>(
    getInitialWorkspaceTab,
  );
  const [detailIntentKey, setDetailIntentKey] = useState(0);
  const [targetApp, setTargetApp] = useState(getInitialApp);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    null,
  );
  const [methodologyIntent, setMethodologyIntent] =
    useState<MethodologyNavigationIntent | null>(null);
  const [projectAiConfigIntent, setProjectAiConfigIntent] =
    useState<ProjectAiConfigNavigationIntent | null>(null);
  const [addProjectOpen, setAddProjectOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [settingsNavIntent, setSettingsNavIntent] =
    useState<SettingsNavIntent | null>(null);
  const [skillsFocusIntent, setSkillsFocusIntent] =
    useState<SkillsFocusIntent | null>(null);

  const handleNavigate = useCallback((view: PageView) => {
    setCurrentView(view);
  }, []);
  const handleFocusSkillInMainPanel = useCallback(
    (directory: string) => {
      setSkillsFocusIntent({ directory, key: Date.now() });
      handleNavigate("skills");
    },
    [handleNavigate],
  );
  const openProjectWorkflow = useCallback(
    (projectId: string) => {
      setSelectedProjectId(projectId);
      setMethodologyIntent({
        key: Date.now(),
        projectId,
        tab: "orchestration",
      });
      handleNavigate("methodology");
    },
    [handleNavigate],
  );
  const openAiProviderSettings = useCallback(() => {
    setSettingsNavIntent(buildAiProviderSettingsIntent());
    handleNavigate("settings");
  }, [handleNavigate]);

  useBudgetAlerts();
  const { data: settings } = useSettingsQuery();
  const {
    projects,
    add: addProject,
    remove: removeProject,
    reload: reloadProjects,
  } = useProjects();

  useEffect(() => {
    invoke<boolean>("is_onboarding_needed")
      .then(setShowOnboarding)
      .catch(() => setShowOnboarding(false));
  }, []);
  useEffect(() => {
    localStorage.setItem(VIEW_STORAGE_KEY, currentView);
  }, [currentView]);
  useEffect(() => {
    persistWorkspaceTab(workspaceTab);
  }, [workspaceTab]);
  useEffect(() => {
    if (currentView !== "settings" || settingsNavIntent === null) return;
    const timer = window.setTimeout(() => setSettingsNavIntent(null), 0);
    return () => window.clearTimeout(timer);
  }, [currentView, settingsNavIntent]);
  useEffect(() => {
    localStorage.setItem(APP_STORAGE_KEY, targetApp);
  }, [targetApp]);

  // 托盘告警条目点击 → 跳转到工作区今日告警面板（工作区重构 2026-07-30）
  useEffect(() => {
    const unlisten = listen("tray-open-alerts", () => {
      handleNavigate("kanban");
      setWorkspaceTab("dashboard");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleNavigate]);

  const pageActionRefs = usePageActionRefs();
  useAppKeyboardShortcuts({
    currentView,
    onNavigate: handleNavigate,
    onToggleShortcuts: () => setShortcutsOpen((open) => !open),
  });

  const selectedProject = useMemo(
    () => projects.find((project) => project.id === selectedProjectId) ?? null,
    [projects, selectedProjectId],
  );
  const projectDetailIntent = useMemo(
    () =>
      selectedProjectId === null
        ? null
        : {
            projectId: selectedProjectId,
            key: detailIntentKey,
          },
    [detailIntentKey, selectedProjectId],
  );

  const openWorkspace = useCallback(
    (tab: WorkspaceTab) => {
      handleNavigate("kanban");
      setWorkspaceTab(tab);
      setSelectedProjectId(null);
    },
    [handleNavigate],
  );
  const handleProjectClick = useCallback(
    (projectId: string) => {
      setSelectedProjectId(projectId);
      handleNavigate("kanban");
      setWorkspaceTab("board");
      setDetailIntentKey((key) => key + 1);
    },
    [handleNavigate],
  );
  /*
   * 「配置 AI 资产」以前是：跳到工作区 → 强行把 Tab 切到「项目看板」→ 弹抽屉
   * → 抽屉再切到第二个 Tab。四步里有三步是为了把用户搬到那个 480px 的抽屉
   * 跟前，而且会顺手改掉他当前正在看的 Tab。现在它是一个一级页面，直接去。
   *
   * `intent` 是可选的精确落点（工作区重构 2026-07-30）：告警卡/资产矩阵的
   * 「去修复」不只是去那一页，还要落到具体子 Tab 与具体资产区。
   */
  const handleOpenProjectAssets = useCallback(
    (
      projectId: string,
      intent?: Pick<
        ProjectAiConfigNavigationIntent,
        "tab" | "section"
      > | null,
    ) => {
      setSelectedProjectId(projectId);
      if (intent) {
        setProjectAiConfigIntent({
          key: Date.now(),
          projectId,
          tab: intent.tab,
          section: intent.section,
        });
      }
      handleNavigate("projectAiConfig");
    },
    [handleNavigate],
  );
  const handleRemoveProject = useCallback(
    (projectId: string) => {
      if (selectedProjectId === projectId) setSelectedProjectId(null);
      void removeProject(projectId);
    },
    [removeProject, selectedProjectId],
  );
  const handleAddProject = useCallback(
    (name: string, path: string, description?: string) => {
      addProject(name, path, description);
    },
    [addProject],
  );
  const handleBack = useCallback(() => {
    if (currentView === "mcpDiscovery") handleNavigate("mcp");
    if (currentView === "skillsDiscovery") handleNavigate("skills");
  }, [currentView, handleNavigate]);

  const isGlobalAgentConfigView = isAgentConfigView(currentView);
  const pageMeta = PAGE_META[currentView];
  const showAppSwitcher =
    currentView === "prompts" || currentView === "sessions";
  const showBackButton =
    currentView === "mcpDiscovery" || currentView === "skillsDiscovery";
  const hideContentHeader = [
    "settings",
    "tokenStats",
    "kanban",
    // 这一页自带标题栏（含项目切换器），外层再画一条就是两行标题。
    "projectAiConfig",
  ].includes(currentView);
  const dragBarHeight = DEFAULT_DRAG_BAR_HEIGHT;
  const needsCustomTitlebar =
    dragBarHeight > 0 || (settings?.useAppWindowControls ?? false);

  return (
    <div
      className="flex flex-col h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30"
      style={{ overflowX: "hidden" }}
    >
      {showOnboarding && (
        <OnboardingWizard onComplete={() => setShowOnboarding(false)} />
      )}
      {needsCustomTitlebar && (
        <header
          className="fixed top-0 z-50 w-full bg-background/80 backdrop-blur-md border-b border-border/40"
          {...DRAG_REGION_ATTR}
          style={
            {
              ...DRAG_REGION_STYLE,
              height: dragBarHeight,
            } as React.CSSProperties
          }
        />
      )}
      <TooltipProvider delayDuration={300}>
        <div
          className="flex flex-1 min-h-0"
          style={{ paddingTop: needsCustomTitlebar ? dragBarHeight : 0 }}
        >
          <Sidebar
            activeView={currentView}
            workspaceTab={workspaceTab}
            onNavigate={handleNavigate}
            onWorkspaceTabChange={openWorkspace}
            onOpenProjectAssets={handleOpenProjectAssets}
            onAddProject={() => setAddProjectOpen(true)}
            projects={projects}
            activeProjectId={selectedProjectId ?? undefined}
            onProjectClick={handleProjectClick}
            onProjectRemove={handleRemoveProject}
          />
          <main className="flex-1 min-h-0 flex flex-col overflow-y-auto">
            {!hideContentHeader && (
              <div
                className="sticky top-0 z-10 shrink-0 flex items-center justify-between gap-4 px-6 py-2.5 border-b border-border/30 bg-background/90 backdrop-blur-sm [@media(max-height:640px)]:py-1.5"
                style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
              >
                <div className="flex items-center gap-3 min-w-0">
                  {showBackButton && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleBack}
                      className="rounded-lg shrink-0"
                    >
                      {t("common.back", { defaultValue: "返回" })}
                    </Button>
                  )}
                  {isGlobalAgentConfigView && selectedProject && (
                    <button
                      onClick={() => handleProjectClick(selectedProject.id)}
                      className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors shrink-0"
                      title={t("scope.backToProject", {
                        name: selectedProject.name,
                        defaultValue: `返回项目：${selectedProject.name}`,
                      })}
                    >
                      {selectedProject.name}
                      <ChevronRight className="h-3 w-3" />
                    </button>
                  )}
                  <h1 className="text-base font-semibold text-foreground truncate">
                    {AGENT_ASSET_PAGE_TITLES[currentView] ??
                      t(pageMeta.titleKey, {
                        defaultValue: pageMeta.defaultTitle,
                      })}
                  </h1>
                  {isGlobalAgentConfigView && <PageScopeBadge scope="global" />}
                  {showAppSwitcher && (
                    <AppSwitcher
                      activeApp={targetApp}
                      onSwitch={setTargetApp}
                      visibleApps={ALL_VISIBLE_APPS}
                      compact
                    />
                  )}
                </div>
                {hasPageActions(currentView) && (
                  <div className="flex items-center gap-1 shrink-0">
                    <AppPageActions
                      view={currentView}
                      refs={pageActionRefs}
                      onNavigate={handleNavigate}
                    />
                  </div>
                )}
              </div>
            )}
            <ErrorBoundary
              fallbackTitle={t("errors.pageLoadFailed")}
              fallbackDescription={t("errors.pageLoadFailedDescription")}
              onGoBack={() => handleNavigate("mcp")}
            >
              <div key={currentView} className="flex-1 min-h-0 flex flex-col">
                <AppPageRouter
                  view={currentView}
                  refs={pageActionRefs}
                  targetApp={targetApp}
                  projects={projects}
                  selectedProjectId={selectedProjectId}
                  projectDetailIntent={projectDetailIntent}
                  workspaceTab={workspaceTab}
                  settingsNavIntent={settingsNavIntent}
                  onNavigate={handleNavigate}
                  onOpenAiProviderSettings={openAiProviderSettings}
                  onWorkspaceTabChange={setWorkspaceTab}
                  onProjectClick={handleProjectClick}
                  onOpenProjectAiConfig={handleOpenProjectAssets}
                  onOpenProjectWorkflow={openProjectWorkflow}
                  onSelectProject={setSelectedProjectId}
                  onProjectRemove={handleRemoveProject}
                  onAddProject={() => setAddProjectOpen(true)}
                  onClearProjectSelection={() => setSelectedProjectId(null)}
                  onProjectsReload={reloadProjects}
                  onSettingsNavIntent={setSettingsNavIntent}
                  methodologyIntent={methodologyIntent}
                  onMethodologyIntentConsumed={() => setMethodologyIntent(null)}
                  projectAiConfigIntent={projectAiConfigIntent}
                  onProjectAiConfigIntentConsumed={() =>
                    setProjectAiConfigIntent(null)
                  }
                  skillsFocusIntent={skillsFocusIntent}
                  onSkillsFocusConsumed={() => setSkillsFocusIntent(null)}
                  onFocusSkillInMainPanel={handleFocusSkillInMainPanel}
                />
              </div>
            </ErrorBoundary>
          </main>
        </div>
      </TooltipProvider>
      <AddProjectDialog
        open={addProjectOpen}
        onOpenChange={setAddProjectOpen}
        onAdd={handleAddProject}
      />
      <ShortcutsHelp open={shortcutsOpen} onOpenChange={setShortcutsOpen} />
      <DeepLinkImportDialog />
    </div>
  );
}

export default App;
