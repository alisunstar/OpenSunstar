import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import {
  EyeOff,
  Plug2,
  Shield,
  ArrowRightLeft,
  LayoutDashboard,
  Server,
  BookOpen,
  Wrench,
  History,
  Terminal,
  Webhook,
  Bot,
  LayoutGrid,
  Coins,
  Folder,
  Settings,
  Plus,
  Rocket,
  FolderOpen,
  ExternalLink,
  Trash2,
  PanelLeftClose,
  PanelLeftOpen,
  Sun,
  Moon,
  Sparkles,
  Cpu,
  Cloud,
  Users,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { useTheme } from "@/components/theme-provider";
import { SidebarItem } from "./SidebarItem";
import { SidebarMenu } from "./SidebarMenu";
import { SyncStatusBar } from "./SyncStatusBar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { PageView } from "@/app/navigation";
import type { Project } from "@/types/project";
import type { WorkspaceTab } from "@/types/workspace";
import appIcon from "@/assets/icons/app-icon-128.png";

// ── 类型 ──────────────────────────────────────────

interface SidebarProps {
  activeView: PageView;
  workspaceTab?: WorkspaceTab;
  onNavigate: (view: PageView) => void;
  onWorkspaceTabChange?: (tab: WorkspaceTab) => void;
  onOpenProjectAssets?: (projectId: string) => void;
  onAddProject?: () => void;
  projects?: Project[];
  activeProjectId?: string;
  onProjectClick?: (projectId: string) => void;
  onProjectRemove?: (projectId: string) => void;
}

// ── 辅助 ──────────────────────────────────────────

const COLLAPSED_STORAGE_KEY = "OpenSunstar-sidebar-collapsed";

const AGENT_CONFIG_VIEWS: PageView[] = [
  "mcp",
  "mcpDiscovery",
  "skills",
  "skillsDiscovery",
  "prompts",
  "commands",
  "hooks",
  "ignore",
  "permissions",
  "agents",
  "convert",
];

const AI_MODEL_VIEWS: PageView[] = ["simpleConnect", "sessions", "tokenStats"];

function isAgentConfigActive(view: PageView): boolean {
  return AGENT_CONFIG_VIEWS.includes(view);
}

function isAiModelActive(view: PageView): boolean {
  return AI_MODEL_VIEWS.includes(view);
}

function SectionLabel({
  children,
  collapsed,
}: {
  children: string;
  collapsed?: boolean;
}) {
  if (collapsed) return null;
  return (
    <div className="px-3 pt-4 pb-1.5">
      <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/60">
        {children}
      </span>
    </div>
  );
}

// ── 组件 ──────────────────────────────────────────

function isWorkspaceActive(view: PageView): boolean {
  return view === "kanban";
}

function isProjectConfigActive(view: PageView): boolean {
  return view === "projectAiConfig" || view === "methodology";
}

export function Sidebar({
  activeView,
  workspaceTab = "dashboard",
  onNavigate,
  onWorkspaceTabChange,
  onOpenProjectAssets,
  onAddProject,
  projects = [],
  activeProjectId,
  onProjectClick,
  onProjectRemove,
}: SidebarProps) {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const agentConfigActive = isAgentConfigActive(activeView);
  const aiModelActive = isAiModelActive(activeView);
  const workspaceActive = isWorkspaceActive(activeView);
  const projectConfigActive = isProjectConfigActive(activeView);
  const activeProject = activeProjectId
    ? projects.find((p) => p.id === activeProjectId)
    : undefined;

  const goWorkspace = (tab: WorkspaceTab) => {
    if (onWorkspaceTabChange) {
      onWorkspaceTabChange(tab);
      return;
    }
    onNavigate("kanban");
  };

  /*
   * 有选中项目时走 `onOpenProjectAssets`（它会顺手把「当前项目」钉到这一个），
   * 没有就直接导航 —— 页面自己会认领第一个项目。两条路的落点是同一页，
   * 区别只在「带不带项目一起过去」。
   */
  const openProjectAiConfig = useCallback(() => {
    if (activeProject && onOpenProjectAssets) {
      onOpenProjectAssets(activeProject.id);
      return;
    }
    onNavigate("projectAiConfig");
  }, [activeProject, onOpenProjectAssets, onNavigate]);

  // ── 折叠状态 ──────────────────────────────
  const [collapsed, setCollapsed] = useState<boolean>(() => {
    try {
      return localStorage.getItem(COLLAPSED_STORAGE_KEY) === "true";
    } catch {
      return false;
    }
  });

  useEffect(() => {
    localStorage.setItem(COLLAPSED_STORAGE_KEY, String(collapsed));
  }, [collapsed]);

  const toggleCollapsed = useCallback(() => setCollapsed((c) => !c), []);

  // ── 全局快捷键：Ctrl+B / Cmd+B ──────────────
  useEffect(() => {
    const handler = () => setCollapsed((c) => !c);
    window.addEventListener("toggle-sidebar", handler);
    return () => window.removeEventListener("toggle-sidebar", handler);
  }, []);

  const sidebarWidth = collapsed ? 56 : 240; // w-14 : w-60

  return (
    <motion.aside
      className={cn(
        "h-full flex flex-col shrink-0 border-r border-border/60",
        "bg-background/60 backdrop-blur-md",
        "sidebar-scroll overflow-hidden",
      )}
      animate={{ width: sidebarWidth }}
      initial={{ width: collapsed ? 56 : 240 }}
      transition={{ type: "spring", stiffness: 300, damping: 28 }}
      style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
    >
      {/* ── Logo 区域 ─────────────────────────── */}
      <div
        className={cn(
          "shrink-0 flex items-center border-b border-border/40 transition-all",
          collapsed ? "h-14 px-2 justify-center" : "h-14 px-4 gap-3",
        )}
      >
        <img
          src={appIcon}
          alt="OpenSunstar"
          className="w-8 h-8 shrink-0 object-contain"
          draggable={false}
        />
        {!collapsed && (
          <span className="text-sm font-semibold text-foreground tracking-tight truncate">
            OpenSunstar
          </span>
        )}
      </div>

      {/* ── 导航区域 ─────────────────────────── */}
      <nav className="flex-1 overflow-y-auto px-2.5 py-2 space-y-0.5">
        {/* ▸ Agent 配置 */}
        {collapsed ? (
          <div className="space-y-0.5">
            <SidebarItem
              icon={<Rocket className="w-4 h-4" />}
              label=""
              active={workspaceActive}
              onClick={() => goWorkspace("dashboard")}
              accent={workspaceActive}
              collapsed
              title={t("workspace.title", { defaultValue: "项目驾驶舱" })}
            />
            <SidebarItem
              icon={<Folder className="w-4 h-4" />}
              label=""
              active={false}
              onClick={() => onAddProject?.()}
              accent={false}
              collapsed
              title={t("sidebar.section.myProjects", {
                defaultValue: "我的项目",
              })}
            />
            <SidebarItem
              icon={<LayoutDashboard className="w-4 h-4" />}
              label=""
              active={agentConfigActive}
              onClick={() => onNavigate("mcp")}
              accent={agentConfigActive}
              collapsed
            />
            <SidebarItem
              icon={<Sparkles className="w-4 h-4" />}
              label=""
              active={activeView === "projectAiConfig"}
              onClick={openProjectAiConfig}
              accent={activeView === "projectAiConfig"}
              collapsed
              title={t("projectAiConfig.title", {
                defaultValue: "AI资产配置",
              })}
            />
            <SidebarItem
              icon={<Cpu className="w-4 h-4" />}
              label=""
              active={aiModelActive}
              onClick={() => onNavigate("simpleConnect")}
              accent={aiModelActive}
              collapsed
            />
            <SidebarItem
              icon={<BookOpen className="w-4 h-4" />}
              label=""
              active={activeView === "methodology"}
              onClick={() => onNavigate("methodology")}
              accent={activeView === "methodology"}
              collapsed
              title={t("methodology.sidebar", { defaultValue: "工作流编排" })}
            />
            <SidebarItem
              icon={<Cloud className="w-4 h-4" />}
              label=""
              active={activeView === "cloudSync"}
              onClick={() => onNavigate("cloudSync")}
              accent={activeView === "cloudSync"}
              collapsed
              title={t("sidebar.section.cloudSync", {
                defaultValue: "跨设备云同步",
              })}
            />
            <SidebarItem
              icon={<Users className="w-4 h-4" />}
              label=""
              active={activeView === "teamCollaboration"}
              onClick={() => onNavigate("teamCollaboration")}
              accent={activeView === "teamCollaboration"}
              collapsed
              title={t("sidebar.section.teamCollab", {
                defaultValue: "团队协作配置",
              })}
            />
          </div>
        ) : (
          <>
            {/* ▸ 项目驾驶舱 */}
            <SidebarMenu
              icon={<Rocket className="w-4 h-4" />}
              label={t("workspace.title", { defaultValue: "项目驾驶舱" })}
              defaultOpen
              active={workspaceActive}
            >
              <SidebarItem
                icon={<LayoutDashboard className="w-4 h-4" />}
                label={t("workspace.tabs.dashboard", {
                  defaultValue: "今日告警",
                })}
                active={activeView === "kanban" && workspaceTab === "dashboard"}
                onClick={() => goWorkspace("dashboard")}
                indent
              />
              <SidebarItem
                icon={<LayoutGrid className="w-4 h-4" />}
                label={t("workspace.tabs.board", { defaultValue: "项目看板" })}
                active={
                  activeView === "kanban" &&
                  workspaceTab === "board" &&
                  !activeProjectId
                }
                onClick={() => goWorkspace("board")}
                indent
              />
            </SidebarMenu>

            {/* ▸ 我的项目 */}
            <SidebarMenu
              icon={<Folder className="w-4 h-4" />}
              label={t("sidebar.section.myProjects", {
                defaultValue: "我的项目",
              })}
              defaultOpen={false}
            >
              {projects.map((project) => (
                <ProjectItem
                  key={project.id}
                  project={project}
                  active={
                    activeView === "kanban" &&
                    workspaceTab === "board" &&
                    activeProjectId === project.id
                  }
                  onClick={() => onProjectClick?.(project.id)}
                  onRemove={() => onProjectRemove?.(project.id)}
                />
              ))}
              <SidebarItem
                icon={<Plus className="w-4 h-4" />}
                label={t("sidebar.addProject", { defaultValue: "添加项目" })}
                onClick={() => onAddProject?.()}
                indent
                accent={false}
              />
            </SidebarMenu>

            {/* ▸ 项目配置 */}
            <SidebarMenu
              icon={<Settings className="w-4 h-4" />}
              label={t("sidebar.projectConfig", {
                defaultValue: "项目配置",
              })}
              defaultOpen
              active={projectConfigActive}
            >
              <SidebarItem
                icon={<Sparkles className="w-4 h-4" />}
                label={t("projectAiConfig.title", {
                  defaultValue: "AI资产配置",
                })}
                active={activeView === "projectAiConfig"}
                onClick={openProjectAiConfig}
                badge={
                  activeProject ? (
                    <span className="max-w-[72px] truncate text-[10px] text-muted-foreground/70">
                      {activeProject.name}
                    </span>
                  ) : undefined
                }
                indent
              />
              <SidebarItem
                icon={<BookOpen className="w-4 h-4" />}
                label={t("methodology.sidebar", {
                  defaultValue: "工作流编排",
                })}
                active={activeView === "methodology"}
                onClick={() => onNavigate("methodology")}
                indent
              />
            </SidebarMenu>

            {/*
             * 「Agent 配置」vs「AI 资产总览」这对名字暗示的是「编辑 vs 查看」，
             * 但两边的实体集合完全相同（8 类）。真实边界是**作用域**：这里是
             * 全局库的增删改，工作区里那个 Tab 是项目级的关联落地。
             *
             * 审查报告 §2.5 建议改名成「资产库 / 项目落地」把作用域写进名字，
             * 一度也这么改了；产品决定保留沿用已久的旧名（老用户的肌肉记忆比
             * 命名的自解释性更贵），只去掉「跨Agent配置」的「跨」字前缀 ——
             * 它和「跨项目工作区」的「跨」一样，是在描述实现而不是用户的事。
             * 作用域这层意思因此仍然只活在下面这句 hint 和文档里，是已知欠账。
             */}
            <SidebarMenu
              icon={<LayoutDashboard className="w-4 h-4" />}
              label={t("sidebar.agentConfig", { defaultValue: "Agent 配置" })}
              badge={
                <span className="rounded-full border border-border/70 bg-muted/50 px-1.5 py-0.5 text-[9px] font-medium text-muted-foreground">
                  {t("scope.globalScope", { defaultValue: "全局" })}
                </span>
              }
              defaultOpen={false}
              active={agentConfigActive}
            >
              <SidebarItem
                icon={<Server className="w-4 h-4" />}
                label="MCP"
                active={activeView === "mcp" || activeView === "mcpDiscovery"}
                onClick={() => onNavigate("mcp")}
                indent
              />
              <SidebarItem
                icon={<Wrench className="w-4 h-4" />}
                label="Skills"
                active={
                  activeView === "skills" || activeView === "skillsDiscovery"
                }
                onClick={() => onNavigate("skills")}
                indent
              />
              <SidebarItem
                icon={<BookOpen className="w-4 h-4" />}
                label={t("prompts.manage", {
                  defaultValue: "Prompt & Rules",
                })}
                active={activeView === "prompts"}
                onClick={() => onNavigate("prompts")}
                indent
              />
              <SidebarItem
                icon={<Terminal className="w-4 h-4" />}
                label="Commands"
                active={activeView === "commands"}
                onClick={() => onNavigate("commands")}
                indent
              />
              <SidebarItem
                icon={<Webhook className="w-4 h-4" />}
                label="Hooks"
                active={activeView === "hooks"}
                onClick={() => onNavigate("hooks")}
                indent
              />
              <SidebarItem
                icon={<EyeOff className="w-4 h-4" />}
                label="Ignore"
                active={activeView === "ignore"}
                onClick={() => onNavigate("ignore")}
                indent
              />
              <SidebarItem
                icon={<Shield className="w-4 h-4" />}
                label="Permissions"
                active={activeView === "permissions"}
                onClick={() => onNavigate("permissions")}
                indent
              />
              <SidebarItem
                icon={<Bot className="w-4 h-4" />}
                label="Subagents"
                active={activeView === "agents"}
                onClick={() => onNavigate("agents")}
                indent
              />
              <SidebarItem
                icon={<ArrowRightLeft className="w-4 h-4" />}
                label="Convert"
                active={activeView === "convert"}
                onClick={() => onNavigate("convert")}
                indent
              />
            </SidebarMenu>

            {/* ▸ AI 模型 */}
            <SectionLabel>
              {t("sidebar.section.aiModels", { defaultValue: "AI模型" })}
            </SectionLabel>

            <SidebarItem
              icon={<Plug2 className="w-4 h-4" />}
              label={t("quickStart.nav", { defaultValue: "快速接入" })}
              active={activeView === "simpleConnect"}
              onClick={() => onNavigate("simpleConnect")}
            />

            <SidebarItem
              icon={<History className="w-4 h-4" />}
              label={t("sessionManager.title", { defaultValue: "Context" })}
              active={activeView === "sessions"}
              onClick={() => onNavigate("sessions")}
            />

            <SidebarItem
              icon={<Coins className="w-4 h-4" />}
              label={t("sidebar.tokenStats", { defaultValue: "AI Tokens" })}
              active={activeView === "tokenStats"}
              onClick={() => onNavigate("tokenStats")}
            />

            {/*
             * ▸ 同步与协作
             *
             * 此前这里是 SectionLabel「跨设备云同步」+ 同名的唯一一个菜单项，
             * 而「团队协作配置」**没有自己的 SectionLabel**，于是它在视觉上
             * 被吞进了上一组（§2.3 附带的真 bug）。修法不是给它单独补一个标题
             * ——那又是第五个空嵌套——是承认这两条本来就是一类东西（把配置搬到
             * 别处：一个搬到自己的另一台设备，一个搬到队友那里），给它们一个
             * 共同的家。顺带这个标题还负责终结上面的「AI模型」分组。
             */}
            <SectionLabel>
              {t("sidebar.section.syncCollab", { defaultValue: "同步与协作" })}
            </SectionLabel>
            <SidebarItem
              icon={<Cloud className="w-4 h-4" />}
              label={t("cloudSyncDashboard.title", {
                defaultValue: "跨设备云同步",
              })}
              active={activeView === "cloudSync"}
              onClick={() => onNavigate("cloudSync")}
            />
            <SidebarItem
              icon={<Users className="w-4 h-4" />}
              label={t("sidebar.section.teamCollab", {
                defaultValue: "团队协作配置",
              })}
              badge={
                // Invite-only beta: the control plane answers 403 on every paid
                // team route until an entitlement is granted, so the badge has to
                // read "Beta" — under "MVP" that denial looks like a bug.
                <span className="rounded-full border border-primary/20 bg-primary/10 px-1.5 py-0.5 text-[9px] font-semibold text-primary">
                  Beta
                </span>
              }
              active={activeView === "teamCollaboration"}
              onClick={() => onNavigate("teamCollaboration")}
            />
          </>
        )}
      </nav>

      {/* ── 底部：同步状态 + 设置 + 折叠 ──────────────── */}
      <div
        className={cn(
          "shrink-0 border-t border-border/40",
          "bg-background/40 backdrop-blur-sm",
        )}
      >
        <SyncStatusBar collapsed={collapsed} />
        <div className={cn("px-2.5 py-1.5 space-y-1", collapsed && "px-1.5")}>
          <SidebarItem
            icon={<Settings className="w-4 h-4" />}
            label={
              collapsed ? "" : t("common.settings", { defaultValue: "设置" })
            }
            active={activeView === "settings"}
            onClick={() => onNavigate("settings")}
            collapsed={collapsed}
            title={
              collapsed
                ? t("common.settings", { defaultValue: "设置" })
                : undefined
            }
          />

          {/* 折叠按钮 + 主题切换 */}
          <div className={cn("flex gap-1", collapsed && "flex-col")}>
            <Button
              variant="ghost"
              size="sm"
              onClick={toggleCollapsed}
              className={cn(
                "rounded-lg text-muted-foreground hover:text-foreground hover:bg-muted/50",
                collapsed
                  ? "flex-1 h-9 justify-center"
                  : "flex-1 h-9 justify-start gap-3 pl-3",
              )}
              title={
                collapsed
                  ? t("sidebar.expand", { defaultValue: "展开侧边栏" })
                  : `${t("sidebar.collapse", { defaultValue: "折叠侧边栏" })} (Ctrl+B)`
              }
            >
              {collapsed ? (
                <PanelLeftOpen className="w-4 h-4" />
              ) : (
                <>
                  <PanelLeftClose className="w-4 h-4 shrink-0" />
                  <span className="text-sm font-normal truncate">
                    {t("sidebar.collapse", { defaultValue: "折叠" })}
                  </span>
                </>
              )}
            </Button>

            <Button
              variant="ghost"
              size="sm"
              onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
              className={cn(
                "rounded-lg text-muted-foreground hover:text-foreground hover:bg-muted/50 shrink-0",
                collapsed
                  ? "h-9 w-full justify-center"
                  : "h-9 w-9 justify-center",
              )}
              title={
                theme === "dark"
                  ? t("common.lightMode", { defaultValue: "切换浅色模式" })
                  : t("common.darkMode", { defaultValue: "切换深色模式" })
              }
            >
              <Sun className="h-4 w-4 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
              <Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
            </Button>
          </div>
        </div>
      </div>
    </motion.aside>
  );
}

// ── 项目列表项（带右键菜单）───────────────────

function ProjectItem({
  project,
  active,
  onClick,
  onRemove,
}: {
  project: Project;
  active: boolean;
  onClick: () => void;
  onRemove: () => void;
}) {
  const [contextOpen, setContextOpen] = useState(false);
  const [contextPos, setContextPos] = useState({ x: 0, y: 0 });

  return (
    <div
      className="w-full"
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setContextPos({ x: e.clientX, y: e.clientY });
        setContextOpen(true);
      }}
    >
      <SidebarItem
        icon={<FolderOpen className="w-4 h-4" />}
        label={project.name}
        active={active}
        onClick={onClick}
        indent
      />

      {/* 通过隐藏 trigger 定位右键菜单，避免左键误触 */}
      <DropdownMenu open={contextOpen} onOpenChange={setContextOpen}>
        <DropdownMenuTrigger asChild>
          <span
            className="fixed pointer-events-none"
            style={{
              left: contextPos.x,
              top: contextPos.y,
              width: 1,
              height: 1,
            }}
          />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" side="bottom" className="w-44">
          <DropdownMenuItem onClick={onClick}>
            <ExternalLink className="h-3.5 w-3.5 mr-2" />
            查看项目
          </DropdownMenuItem>
          <DropdownMenuItem
            className="text-destructive focus:text-destructive"
            onClick={onRemove}
          >
            <Trash2 className="h-3.5 w-3.5 mr-2" />
            移除项目
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
