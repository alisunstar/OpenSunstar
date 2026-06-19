import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import {
  LayoutDashboard,
  Server,
  BookOpen,
  Wrench,
  History,
  Activity,
  LayoutGrid,
  Coins,
  Settings,
  Plus,
  FolderOpen,
  ExternalLink,
  Trash2,
  PanelLeftClose,
  PanelLeftOpen,
  Sun,
  Moon,
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
import type { PageView } from "@/App";
import type { Project } from "@/types/project";

// ── 类型 ──────────────────────────────────────────

interface SidebarProps {
  activeView: PageView;
  onNavigate: (view: PageView) => void;
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
  "prompts",
  "skills",
  "skillsDiscovery",
];

const MONITOR_VIEWS: PageView[] = ["sessions", "tokenStats"];

function isAgentConfigActive(view: PageView): boolean {
  return AGENT_CONFIG_VIEWS.includes(view);
}

function isMonitorActive(view: PageView): boolean {
  return MONITOR_VIEWS.includes(view);
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

export function Sidebar({
  activeView,
  onNavigate,
  onAddProject,
  projects = [],
  activeProjectId,
  onProjectClick,
  onProjectRemove,
}: SidebarProps) {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const agentConfigActive = isAgentConfigActive(activeView);
  const monitorActive = isMonitorActive(activeView);

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
        <div className="w-8 h-8 rounded-lg bg-blue-500/10 flex items-center justify-center shrink-0">
          <Server className="w-4.5 h-4.5 text-blue-500" />
        </div>
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
              icon={<LayoutDashboard className="w-4 h-4" />}
              label=""
              active={agentConfigActive}
              onClick={() => onNavigate("mcp")}
              accent={agentConfigActive}
              collapsed
            />
            <SidebarItem
              icon={<Activity className="w-4 h-4" />}
              label=""
              active={monitorActive}
              onClick={() => onNavigate("sessions")}
              accent={monitorActive}
              collapsed
            />
            <SidebarItem
              icon={<LayoutGrid className="w-4 h-4" />}
              label=""
              active={activeView === "kanban"}
              onClick={() => onNavigate("kanban")}
              collapsed
            />
          </div>
        ) : (
          <>
            <SidebarMenu
              icon={<LayoutDashboard className="w-4 h-4" />}
              label={t("sidebar.agentConfig", { defaultValue: "Agent 配置" })}
              defaultOpen
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
                icon={<BookOpen className="w-4 h-4" />}
                label="Prompts"
                active={activeView === "prompts"}
                onClick={() => onNavigate("prompts")}
                indent
              />
              <SidebarItem
                icon={<Wrench className="w-4 h-4" />}
                label={t("skills.manage", { defaultValue: "Skills" })}
                active={
                  activeView === "skills" || activeView === "skillsDiscovery"
                }
                onClick={() => onNavigate("skills")}
                indent
              />
            </SidebarMenu>

            {/* ▸ 运行监控 */}
            <SectionLabel>
              {t("sidebar.section.monitor", { defaultValue: "运行监控" })}
            </SectionLabel>

            <SidebarItem
              icon={<History className="w-4 h-4" />}
              label={t("sessionManager.title", { defaultValue: "Context" })}
              active={activeView === "sessions"}
              onClick={() => onNavigate("sessions")}
            />

            <SidebarItem
              icon={<Coins className="w-4 h-4" />}
              label={t("sidebar.tokenStats", { defaultValue: "AI 用量" })}
              active={activeView === "tokenStats"}
              onClick={() => onNavigate("tokenStats")}
            />

            {/* ▸ 项目 */}
            <SectionLabel>
              {t("sidebar.section.projects", { defaultValue: "项目" })}
            </SectionLabel>

            <SidebarItem
              icon={<LayoutGrid className="w-4 h-4" />}
              label={t("sidebar.kanban", { defaultValue: "看板总览" })}
              active={activeView === "kanban" && !activeProjectId}
              onClick={() => onNavigate("kanban")}
            />

            {projects.map((project) => (
              <ProjectItem
                key={project.id}
                project={project}
                active={
                  activeView === "kanban" && activeProjectId === project.id
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
            label={collapsed ? "" : t("common.settings", { defaultValue: "设置" })}
            active={activeView === "settings"}
            onClick={() => onNavigate("settings")}
            collapsed={collapsed}
            title={collapsed ? t("common.settings", { defaultValue: "设置" }) : undefined}
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
                collapsed ? "h-9 w-full justify-center" : "h-9 w-9 justify-center",
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
