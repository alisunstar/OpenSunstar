import { useEffect, useMemo, useRef, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Plus,
  Download,
  FolderArchive,
  RefreshCw,
  Search,
  Settings,
  History,
} from "lucide-react";
import type { AppId } from "@/lib/api";
import { OnboardingWizard } from "@/components/onboarding/OnboardingWizard";
import {
  isWindows,
  isLinux,
  DRAG_REGION_ATTR,
  DRAG_REGION_STYLE,
} from "@/lib/platform";
import { Button } from "@/components/ui/button";
import { AppSwitcher } from "@/components/AppSwitcher";
import UnifiedMcpPanel from "@/components/mcp/UnifiedMcpPanel";
import { McpDiscoveryPage } from "@/components/mcp/McpDiscoveryPage";
import type { McpDiscoveryPageHandle } from "@/components/mcp/McpDiscoveryPage";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { TooltipProvider } from "@/components/ui/tooltip";
import PromptPanel from "@/components/prompts/PromptPanel";
import CommandsPanel from "@/components/commands/CommandsPanel";
import HooksPanel from "@/components/hooks/HooksPanel";
import { ConvertPage } from "@/components/convert/ConvertPage";
import IgnorePanel from "@/components/ignore/IgnorePanel";
import PermissionsPanel from "@/components/permissions/PermissionsPanel";
import AgentsPanel from "@/components/agents/AgentsPanel";
import { SkillsPage } from "@/components/skills/SkillsPage";
import UnifiedSkillsPanel from "@/components/skills/UnifiedSkillsPanel";
import { SessionManagerPage } from "@/components/sessions/SessionManagerPage";
import { QuickStartPage } from "@/components/quickStart/QuickStartPage";
import { DeepLinkImportDialog } from "@/components/DeepLinkImportDialog";
import { SettingsPageContent } from "@/components/settings/SettingsPage";
import { Sidebar } from "@/components/layout/Sidebar";
import { SyncBackupPage } from "@/components/sync/SyncBackupPage";
import { TokenStatsPage } from "@/components/usage/TokenStatsPage";
import { KanbanPage } from "@/components/kanban/KanbanPage";
import { AddProjectDialog } from "@/components/projects/AddProjectDialog";
import { ShortcutsHelp } from "@/components/ShortcutsHelp";
import { useProjects } from "@/hooks/useProjects";
import { useBudgetAlerts } from "@/hooks/useBudgetAlerts";
import { useSettingsQuery } from "@/lib/query";
import {
  buildProxySettingsIntent,
  buildAiProviderSettingsIntent,
  type SettingsNavIntent,
} from "@/lib/settingsNavigation";

import type { WorkspaceTab } from "@/types/workspace";
import {
  getInitialWorkspaceTab,
  persistWorkspaceTab,
} from "@/types/workspace";
import type { ProjectDetailTab } from "@/types/projectDetail";

export type PageView =
  | "simpleConnect"
  | "mcp"
  | "mcpDiscovery"
  | "prompts"
  | "commands"
  | "hooks"
  | "convert"
  | "ignore"
  | "permissions"
  | "agents"
  | "skills"
  | "skillsDiscovery"
  | "sessions"
  | "syncBackup"
  | "kanban"
  | "tokenStats"
  | "settings";

// ── 常量 ──────────────────────────────────────────

const DEFAULT_DRAG_BAR_HEIGHT = isWindows() || isLinux() ? 0 : 28;
const HEADER_HEIGHT = 40;

const APP_STORAGE_KEY = "OpenSunstar-ext-last-app";
const VIEW_STORAGE_KEY = "OpenSunstar-ext-last-view";

const VALID_APPS: AppId[] = [
  "claude",
  "claude-desktop",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

const VALID_VIEWS: PageView[] = [
  "simpleConnect",
  "mcp",
  "mcpDiscovery",
  "prompts",
  "commands",
  "hooks",
  "convert",
  "ignore",
  "permissions",
  "agents",
  "skills",
  "skillsDiscovery",
  "sessions",
  "syncBackup",
  "kanban",
  "tokenStats",
  "settings",
];

const ALL_VISIBLE_APPS: Record<AppId, boolean> = {
  claude: true,
  "claude-desktop": true,
  codex: true,
  gemini: true,
  opencode: true,
  openclaw: true,
  hermes: true,
};

const getInitialApp = (): AppId => {
  const saved = localStorage.getItem(APP_STORAGE_KEY) as AppId | null;
  if (saved && VALID_APPS.includes(saved)) return saved;
  return "claude";
};

const getInitialView = (): PageView => {
  const saved = localStorage.getItem(VIEW_STORAGE_KEY) as PageView | null;
  if (saved && VALID_VIEWS.includes(saved)) return saved;
  return "kanban";
};

// ── 视图元数据 ────────────────────────────────────

interface PageMeta {
  titleKey: string;
  defaultTitle: string;
}

const PAGE_META: Record<PageView, PageMeta> = {
  simpleConnect: {
    titleKey: "simpleConnect.pageTitle",
    defaultTitle: "API Access",
  },
  mcp: { titleKey: "mcp.title", defaultTitle: "MCP" },
  mcpDiscovery: { titleKey: "mcp.discover", defaultTitle: "Discover MCP" },
  prompts: { titleKey: "prompts.title", defaultTitle: "Prompts" },
  commands: { titleKey: "commands.title", defaultTitle: "Commands" },
  hooks: { titleKey: "hooks.title", defaultTitle: "Hooks" },
  convert: { titleKey: "convert.title", defaultTitle: "Convert" },
  ignore: { titleKey: "ignore.title", defaultTitle: "Ignore" },
  permissions: { titleKey: "permissions.title", defaultTitle: "Permissions" },
  agents: { titleKey: "agents.title", defaultTitle: "Subagents" },
  skills: { titleKey: "skills.manage", defaultTitle: "Skills" },
  skillsDiscovery: { titleKey: "skills.discover", defaultTitle: "Discover Skills" },
  sessions: { titleKey: "sessionManager.title", defaultTitle: "Context" },
  syncBackup: { titleKey: "sidebar.syncBackup", defaultTitle: "同步备份" },
  kanban: { titleKey: "workspace.title", defaultTitle: "工作区" },
  tokenStats: { titleKey: "sidebar.tokenStats", defaultTitle: "AI Tokens" },
  settings: { titleKey: "common.settings", defaultTitle: "设置" },
};

/** Agent 资产页顶栏标题：与侧栏一致，固定英文 */
const AGENT_ASSET_PAGE_TITLES: Partial<Record<PageView, string>> = {
  commands: "Commands",
  hooks: "Hooks",
  ignore: "Ignore",
  permissions: "Permissions",
  agents: "Subagents",
  convert: "Convert",
};

// ── 组件 ──────────────────────────────────────────

function App() {
  const { t } = useTranslation();
  const [currentView, setCurrentView] = useState<PageView>(getInitialView);
  const [workspaceTab, setWorkspaceTab] = useState<WorkspaceTab>(
    getInitialWorkspaceTab,
  );
  const [detailIntentKey, setDetailIntentKey] = useState(0);
  const [detailIntentTab, setDetailIntentTab] =
    useState<ProjectDetailTab>("overview");
  const [targetApp, setTargetApp] = useState<AppId>(getInitialApp);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    null,
  );
  const [addProjectOpen, setAddProjectOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [settingsNavIntent, setSettingsNavIntent] =
    useState<SettingsNavIntent | null>(null);

  const openProxySettings = useCallback(() => {
    setSettingsNavIntent(buildProxySettingsIntent());
    setCurrentView("settings");
  }, []);

  const openAiProviderSettings = useCallback(() => {
    setSettingsNavIntent(buildAiProviderSettingsIntent());
    setCurrentView("settings");
  }, []);

  useBudgetAlerts();
  const { data: settings } = useSettingsQuery();

  useEffect(() => {
    invoke<boolean>("is_onboarding_needed")
      .then((needed) => setShowOnboarding(needed))
      .catch(() => setShowOnboarding(false));
  }, []);

  // ── 项目管理 ────────────────────────────────
  const { projects, add: addProject, remove: removeProject } = useProjects();

  // ── localStorage 同步 ──────────────────────
  useEffect(() => {
    localStorage.setItem(VIEW_STORAGE_KEY, currentView);
  }, [currentView]);

  useEffect(() => {
    persistWorkspaceTab(workspaceTab);
  }, [workspaceTab]);

  useEffect(() => {
    if (currentView === "settings" && settingsNavIntent) {
      const timer = window.setTimeout(() => setSettingsNavIntent(null), 0);
      return () => window.clearTimeout(timer);
    }
  }, [currentView, settingsNavIntent]);

  useEffect(() => {
    localStorage.setItem(APP_STORAGE_KEY, targetApp);
  }, [targetApp]);

  const dragBarHeight = DEFAULT_DRAG_BAR_HEIGHT;
  const useAppWindowControls = settings?.useAppWindowControls ?? false;
  // macOS 叠加标题栏需要自定义拖拽区；Win/Linux 默认用系统标题栏，不额外占位
  const needsCustomTitlebar = dragBarHeight > 0 || useAppWindowControls;
  const contentTopOffset = needsCustomTitlebar
    ? dragBarHeight + HEADER_HEIGHT
    : 0;

  // ── Refs ────────────────────────────────────
  const commandsPanelRef = useRef<any>(null);
  const hooksPanelRef = useRef<any>(null);
  const ignorePanelRef = useRef<any>(null);
  const permissionsPanelRef = useRef<any>(null);
  const agentsPanelRef = useRef<any>(null);
  const promptPanelRef = useRef<any>(null);
  const mcpPanelRef = useRef<any>(null);
  const mcpDiscoveryPageRef = useRef<McpDiscoveryPageHandle>(null);
  const unifiedSkillsPanelRef = useRef<any>(null);
  const skillsPageRef = useRef<any>(null);

  // ── 键盘快捷键 ────────────────────────────
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Ctrl/Cmd + B → 折叠/展开侧边栏
      if ((event.ctrlKey || event.metaKey) && event.key === "b") {
        event.preventDefault();
        window.dispatchEvent(new Event("toggle-sidebar"));
        return;
      }
      // Ctrl+/ 或 ? → 快捷键帮助面板
      if (
        ((event.ctrlKey || event.metaKey) && event.key === "/") ||
        (!event.ctrlKey && !event.metaKey && !event.altKey && event.key === "?")
      ) {
        event.preventDefault();
        setShortcutsOpen((prev) => !prev);
        return;
      }
      // Alt+1~6 → 快速切换视图
      if (event.altKey && !event.ctrlKey && !event.metaKey) {
        const workspaceMap: Record<string, PageView> = {
          "1": "mcp",
          "2": "prompts",
          "3": "skills",
          "4": "sessions",
          "5": "tokenStats",
          "6": "kanban",
        };
        const view = workspaceMap[event.key];
        if (view) {
          event.preventDefault();
          setCurrentView(view);
          return;
        }
      }
      // Escape → 返回
      if (event.key === "Escape" && !event.defaultPrevented) {
        if (document.body.style.overflow === "hidden") return;
        if (currentView === "mcpDiscovery") setCurrentView("mcp");
        if (currentView === "skillsDiscovery") setCurrentView("skills");
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [currentView]);

  // ── 派生值 ─────────────────────────────────
  const effectiveTargetApp = useMemo(() => {
    if (targetApp === "claude-desktop") return "claude";
    return targetApp;
  }, [targetApp]);

  const pageMeta = PAGE_META[currentView];

  const showAppSwitcher =
    currentView === "prompts" || currentView === "sessions";
  const showBackButton =
    currentView === "mcpDiscovery" || currentView === "skillsDiscovery";

  // ── 导航 ────────────────────────────────────
  const handleNavigate = useCallback((view: PageView) => {
    setCurrentView(view);
    setSelectedProjectId(null);
  }, []);

  const openWorkspace = useCallback((tab: WorkspaceTab) => {
    setCurrentView("kanban");
    setWorkspaceTab(tab);
    setSelectedProjectId(null);
  }, []);

  const handleBack = useCallback(() => {
    if (currentView === "mcpDiscovery") setCurrentView("mcp");
    if (currentView === "skillsDiscovery") setCurrentView("skills");
  }, [currentView]);

  const handleProjectClick = useCallback((projectId: string) => {
    setSelectedProjectId(projectId);
    setCurrentView("kanban");
    setWorkspaceTab("board");
    setDetailIntentTab("overview");
    setDetailIntentKey((key) => key + 1);
  }, []);

  const handleOpenProjectAssets = useCallback((projectId: string) => {
    setSelectedProjectId(projectId);
    setCurrentView("kanban");
    setWorkspaceTab("board");
    setDetailIntentTab("aiAssets");
    setDetailIntentKey((key) => key + 1);
  }, []);

  const projectDetailIntent = useMemo(() => {
    if (!selectedProjectId) return null;
    return {
      projectId: selectedProjectId,
      tab: detailIntentTab,
      key: detailIntentKey,
    };
  }, [selectedProjectId, detailIntentTab, detailIntentKey]);

  const handleAddProject = useCallback(
    (name: string, path: string, description?: string) => {
      addProject(name, path, description);
    },
    [addProject],
  );

  // ── 渲染页面内容 ────────────────────────────
  const renderPageContent = () => {
    switch (currentView) {
      case "simpleConnect":
        return (
          <QuickStartPage onOpenSettings={openProxySettings} />
        );
      case "mcp":
        return (
          <UnifiedMcpPanel ref={mcpPanelRef} onOpenChange={() => {}} />
        );
      case "mcpDiscovery":
        return (
          <ErrorBoundary
            fallbackTitle="发现 MCP 页面加载失败"
            fallbackDescription="MCP 注册表页面渲染出错，可能是数据异常或网络问题。"
            onGoBack={() => setCurrentView("mcp")}
          >
            <McpDiscoveryPage ref={mcpDiscoveryPageRef} />
          </ErrorBoundary>
        );
      case "prompts":
        return (
          <PromptPanel
            ref={promptPanelRef}
            open={true}
            onOpenChange={() => {}}
            appId={effectiveTargetApp}
          />
        );
      case "commands":
        return <CommandsPanel ref={commandsPanelRef} open={true} />;
      case "hooks":
        return <HooksPanel ref={hooksPanelRef} open={true} />;
      case "convert":
        return <ConvertPage />;
      case "ignore":
        return <IgnorePanel ref={ignorePanelRef} open={true} />;
      case "permissions":
        return <PermissionsPanel ref={permissionsPanelRef} open={true} />;
      case "agents":
        return <AgentsPanel ref={agentsPanelRef} open={true} />;
      case "skills":
        return (
          <UnifiedSkillsPanel
            ref={unifiedSkillsPanelRef}
            onOpenDiscovery={() => setCurrentView("skillsDiscovery")}
            currentApp={
              effectiveTargetApp === "openclaw" ? "claude" : effectiveTargetApp
            }
          />
        );
      case "skillsDiscovery":
        return (
          <SkillsPage
            ref={skillsPageRef}
            initialApp={
              effectiveTargetApp === "openclaw" ? "claude" : effectiveTargetApp
            }
          />
        );
      case "sessions":
        return (
          <SessionManagerPage
            key={effectiveTargetApp}
            appId={effectiveTargetApp}
          />
        );
      case "syncBackup":
        return <SyncBackupPage />;
      case "kanban":
        return (
          <KanbanPage
            projects={projects}
            selectedProjectId={selectedProjectId ?? undefined}
            projectDetailIntent={projectDetailIntent}
            workspaceTab={workspaceTab}
            onWorkspaceTabChange={setWorkspaceTab}
            targetApp={effectiveTargetApp}
            onProjectClick={(project) => {
              setSelectedProjectId(project.id);
            }}
            onProjectRemove={(projectId) => {
              if (selectedProjectId === projectId) {
                setSelectedProjectId(null);
              }
              void removeProject(projectId);
            }}
            onAddProject={() => setAddProjectOpen(true)}
            onClearSelection={() => setSelectedProjectId(null)}
            onOpenSettings={openAiProviderSettings}
            onNavigate={(view) => setCurrentView(view)}
          />
        );
      case "tokenStats":
        return <TokenStatsPage />;
      case "settings":
        return (
          <SettingsPageContent
            settingsNavIntent={settingsNavIntent}
            defaultTab={settingsNavIntent?.tab ?? "general"}
          />
        );
      default:
        return null;
    }
  };

  // ── 渲染内容区操作按钮 ──────────────────────
  const renderActions = () => {
    switch (currentView) {
      case "prompts":
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => promptPanelRef.current?.openAdd()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <Plus className="w-4 h-4 mr-1" />
            {t("prompts.add", { defaultValue: "添加" })}
          </Button>
        );
      case "commands":
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => commandsPanelRef.current?.openAdd()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <Plus className="w-4 h-4 mr-1" />
            {t("commands.add", { defaultValue: "添加命令" })}
          </Button>
        );
      case "hooks":
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => hooksPanelRef.current?.openAdd()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <Plus className="w-4 h-4 mr-1" />
            {t("hooks.add", { defaultValue: "添加钩子" })}
          </Button>
        );
      case "ignore":
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => ignorePanelRef.current?.openAdd()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <Plus className="w-4 h-4 mr-1" />
            {t("ignore.add", { defaultValue: "添加规则" })}
          </Button>
        );
      case "permissions":
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => permissionsPanelRef.current?.openAdd()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <Plus className="w-4 h-4 mr-1" />
            {t("permissions.add", { defaultValue: "添加权限" })}
          </Button>
        );
      case "agents":
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => agentsPanelRef.current?.openAdd()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <Plus className="w-4 h-4 mr-1" />
            {t("agents.add", { defaultValue: "添加 Subagent" })}
          </Button>
        );
      case "mcp":
        return (
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => mcpPanelRef.current?.openImport()}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <Download className="w-4 h-4 mr-1" />
              {t("mcp.importExisting", { defaultValue: "导入" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => mcpPanelRef.current?.openAdd()}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <Plus className="w-4 h-4 mr-1" />
              {t("mcp.addMcp", { defaultValue: "添加" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setCurrentView("mcpDiscovery")}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <Search className="w-4 h-4 mr-1" />
              {t("mcp.discover", { defaultValue: "发现MCP" })}
            </Button>
          </>
        );
      case "mcpDiscovery":
        return null;
      case "skills":
        return (
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() =>
                unifiedSkillsPanelRef.current?.openRestoreFromBackup()
              }
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <History className="w-4 h-4 mr-1" />
              {t("skills.restoreFromBackup.button", { defaultValue: "恢复" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() =>
                unifiedSkillsPanelRef.current?.openInstallFromZip()
              }
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <FolderArchive className="w-4 h-4 mr-1" />
              {t("skills.installFromZip.button", { defaultValue: "安装" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => unifiedSkillsPanelRef.current?.openImport()}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <Download className="w-4 h-4 mr-1" />
              {t("skills.import", { defaultValue: "导入" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setCurrentView("skillsDiscovery")}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <Search className="w-4 h-4 mr-1" />
              {t("skills.discover", { defaultValue: "发现" })}
            </Button>
          </>
        );
      case "skillsDiscovery":
        return (
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => skillsPageRef.current?.refresh()}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <RefreshCw className="w-4 h-4 mr-1" />
              {t("skills.refresh", { defaultValue: "刷新" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => skillsPageRef.current?.openRepoManager()}
              className="hover:bg-black/5 dark:hover:bg-white/5"
            >
              <Settings className="w-4 h-4 mr-1" />
              {t("skills.repoManager", { defaultValue: "仓库" })}
            </Button>
          </>
        );
      default:
        return null;
    }
  };

  const hasActions = [
    "mcp",
    "mcpDiscovery",
    "prompts",
    "commands",
    "hooks",
    "agents",
    "skills",
    "skillsDiscovery",
  ].includes(currentView);

  // 是否隐藏内容区顶栏（设置页 / 看板页自带 header）
  const hideContentHeader =
    currentView === "settings" ||
    currentView === "syncBackup" ||
    currentView === "tokenStats" ||
    currentView === "kanban";

  // ── 渲染 ────────────────────────────────────
  return (
    <div
      className="flex flex-col h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30"
      style={{ overflowX: "hidden" }}
    >
      {showOnboarding && (
        <OnboardingWizard onComplete={() => setShowOnboarding(false)} />
      )}

      {/* ── Titlebar（macOS 叠加标题栏 / 应用级窗口按钮时显示）── */}
      {needsCustomTitlebar && (
        <header
          className="fixed z-50 w-full bg-background/80 backdrop-blur-md border-b border-border/40"
          {...DRAG_REGION_ATTR}
          style={
            {
              ...DRAG_REGION_STYLE,
              top: dragBarHeight,
              height: HEADER_HEIGHT,
            } as React.CSSProperties
          }
        />
      )}

      {/* ── Body：Sidebar + Content ──────────── */}
      <TooltipProvider delayDuration={300}>
        <div
          className="flex flex-1 min-h-0"
          style={{ paddingTop: contentTopOffset }}
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
            onProjectRemove={(projectId) => {
              if (selectedProjectId === projectId) {
                setSelectedProjectId(null);
              }
              void removeProject(projectId);
            }}
          />

          {/* ── 内容区 ─────────────────────────── */}
          <main className="flex-1 min-h-0 flex flex-col overflow-y-auto">
            {/* 内容区顶栏：sticky 置顶，免疫页面组件内部布局抖动 */}
            {!hideContentHeader && (
              <div
                className="sticky top-0 z-10 shrink-0 flex items-center justify-between gap-4 px-6 py-2.5 border-b border-border/30 bg-background/90 backdrop-blur-sm"
                style={
                  { WebkitAppRegion: "no-drag" } as React.CSSProperties
                }
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
                  <h1 className="text-sm font-semibold text-foreground truncate">
                    {AGENT_ASSET_PAGE_TITLES[currentView] ??
                      t(pageMeta.titleKey, {
                        defaultValue: pageMeta.defaultTitle,
                      })}
                  </h1>
                  {showAppSwitcher && (
                    <AppSwitcher
                      activeApp={targetApp}
                      onSwitch={setTargetApp}
                      visibleApps={ALL_VISIBLE_APPS}
                      compact
                    />
                  )}
                </div>

                {hasActions && (
                  <div className="flex items-center gap-1 shrink-0">
                    {renderActions()}
                  </div>
                )}
              </div>
            )}

            {/* 页面主体 */}
            <ErrorBoundary
              fallbackTitle="页面加载失败"
              fallbackDescription="当前视图渲染过程中发生了未预期的错误，请尝试切换到其他页面或刷新。"
              onGoBack={() => setCurrentView("mcp")}
            >
              <div
                key={currentView}
                className="flex-1 min-h-0 flex flex-col"
              >
                {renderPageContent()}
              </div>
            </ErrorBoundary>
          </main>
        </div>
      </TooltipProvider>

      {/* ── 添加项目 Dialog ──────────────────── */}
      <AddProjectDialog
        open={addProjectOpen}
        onOpenChange={setAddProjectOpen}
        onAdd={handleAddProject}
      />

      {/* ── 快捷键帮助 ────────────────────────── */}
      <ShortcutsHelp open={shortcutsOpen} onOpenChange={setShortcutsOpen} />

      <DeepLinkImportDialog />
    </div>
  );
}

export default App;
