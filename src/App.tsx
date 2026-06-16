import { useEffect, useMemo, useRef, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
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
import { SkillsPage } from "@/components/skills/SkillsPage";
import UnifiedSkillsPanel from "@/components/skills/UnifiedSkillsPanel";
import { SessionManagerPage } from "@/components/sessions/SessionManagerPage";
import { SettingsPageContent } from "@/components/settings/SettingsPage";
import { Sidebar } from "@/components/layout/Sidebar";
import { SyncBackupPage } from "@/components/sync/SyncBackupPage";
import { TokenStatsPage } from "@/components/usage/TokenStatsPage";
import { KanbanPage } from "@/components/kanban/KanbanPage";
import { AddProjectDialog } from "@/components/projects/AddProjectDialog";
import { ShortcutsHelp } from "@/components/ShortcutsHelp";
import { useProjects } from "@/hooks/useProjects";

// ── 类型 ──────────────────────────────────────────

export type PageView =
  | "mcp"
  | "mcpDiscovery"
  | "prompts"
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
  "mcp",
  "mcpDiscovery",
  "prompts",
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
  openclaw: false,
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
  return "mcp";
};

// ── 视图元数据 ────────────────────────────────────

interface PageMeta {
  titleKey: string;
  defaultTitle: string;
}

const PAGE_META: Record<PageView, PageMeta> = {
  mcp: { titleKey: "mcp.title", defaultTitle: "MCP" },
  mcpDiscovery: { titleKey: "mcp.discover", defaultTitle: "发现 MCP" },
  prompts: { titleKey: "prompts.title", defaultTitle: "Prompts" },
  skills: { titleKey: "skills.manage", defaultTitle: "Skills" },
  skillsDiscovery: { titleKey: "skills.discover", defaultTitle: "发现 Skills" },
  sessions: { titleKey: "sessionManager.title", defaultTitle: "会话" },
  syncBackup: { titleKey: "sidebar.syncBackup", defaultTitle: "同步备份" },
  kanban: { titleKey: "sidebar.kanban", defaultTitle: "项目看板" },
  tokenStats: { titleKey: "sidebar.tokenStats", defaultTitle: "Tokens 统计" },
  settings: { titleKey: "common.settings", defaultTitle: "设置" },
};

// ── 组件 ──────────────────────────────────────────

function App() {
  const { t } = useTranslation();
  const [currentView, setCurrentView] = useState<PageView>(getInitialView);
  const [targetApp, setTargetApp] = useState<AppId>(getInitialApp);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    null,
  );
  const [addProjectOpen, setAddProjectOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);

  // ── 项目管理 ────────────────────────────────
  const { projects, add: addProject, remove: removeProject } = useProjects();

  // ── localStorage 同步 ──────────────────────
  useEffect(() => {
    localStorage.setItem(VIEW_STORAGE_KEY, currentView);
  }, [currentView]);

  useEffect(() => {
    localStorage.setItem(APP_STORAGE_KEY, targetApp);
  }, [targetApp]);

  const dragBarHeight = DEFAULT_DRAG_BAR_HEIGHT;
  const contentTopOffset = dragBarHeight + HEADER_HEIGHT;

  // ── Refs ────────────────────────────────────
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
      // Alt+1~4 → 快速切换工作台视图
      if (event.altKey && !event.ctrlKey && !event.metaKey) {
        const workspaceMap: Record<string, PageView> = {
          "1": "mcp",
          "2": "prompts",
          "3": "skills",
          "4": "sessions",
          "5": "syncBackup",
          "6": "kanban",
          "7": "tokenStats",
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

  const handleBack = useCallback(() => {
    if (currentView === "mcpDiscovery") setCurrentView("mcp");
    if (currentView === "skillsDiscovery") setCurrentView("skills");
  }, [currentView]);

  const handleProjectClick = useCallback((projectId: string) => {
    setSelectedProjectId(projectId);
    setCurrentView("kanban");
  }, []);

  const handleAddProject = useCallback(
    (name: string, path: string, description?: string) => {
      addProject(name, path, description);
    },
    [addProject],
  );

  // ── 渲染页面内容 ────────────────────────────
  const renderPageContent = () => {
    switch (currentView) {
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
            onProjectClick={(project) => {
              setSelectedProjectId(project.id);
            }}
            onProjectRemove={(projectId) => {
              if (selectedProjectId === projectId) {
                setSelectedProjectId(null);
              }
              removeProject(projectId);
            }}
            onAddProject={() => setAddProjectOpen(true)}
            onClearSelection={() => setSelectedProjectId(null)}
          />
        );
      case "tokenStats":
        return <TokenStatsPage />;
      case "settings":
        return <SettingsPageContent />;
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
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => mcpDiscoveryPageRef.current?.refresh()}
            className="hover:bg-black/5 dark:hover:bg-white/5"
          >
            <RefreshCw className="w-4 h-4 mr-1" />
            {t("common.refresh", { defaultValue: "刷新" })}
          </Button>
        );
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
    "skills",
    "skillsDiscovery",
  ].includes(currentView);

  // 是否隐藏内容区顶栏（设置页自带 header）
  const hideContentHeader =
    currentView === "settings" || currentView === "syncBackup" || currentView === "tokenStats";

  // ── 渲染 ────────────────────────────────────
  return (
    <div
      className="flex flex-col h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30"
      style={{ overflowX: "hidden" }}
    >
      {/* ── Titlebar（拖拽区域）───────────────── */}
      <header
        className="fixed z-50 w-full bg-background/80 backdrop-blur-md border-b border-border/40"
        {...DRAG_REGION_ATTR}
        style={{
          ...DRAG_REGION_STYLE,
          top: dragBarHeight,
          height: HEADER_HEIGHT,
        } as React.CSSProperties}
      />

      {/* ── Body：Sidebar + Content ──────────── */}
      <TooltipProvider delayDuration={300}>
        <div
          className="flex flex-1 min-h-0"
          style={{ paddingTop: contentTopOffset }}
        >
          <Sidebar
            activeView={currentView}
            onNavigate={handleNavigate}
            onAddProject={() => setAddProjectOpen(true)}
            projects={projects}
            activeProjectId={selectedProjectId ?? undefined}
            onProjectClick={handleProjectClick}
            onProjectRemove={(projectId) => {
              if (selectedProjectId === projectId) {
                setSelectedProjectId(null);
              }
              removeProject(projectId);
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
                    {t(pageMeta.titleKey, {
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
    </div>
  );
}

export default App;
