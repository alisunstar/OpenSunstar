import { ErrorBoundary } from "@/components/ErrorBoundary";
import { useTranslation } from "react-i18next";
import AgentsPanel from "@/components/agents/AgentsPanel";
import CommandsPanel from "@/components/commands/CommandsPanel";
import { ConvertPage } from "@/components/convert/ConvertPage";
import HooksPanel from "@/components/hooks/HooksPanel";
import IgnorePanel from "@/components/ignore/IgnorePanel";
import { KanbanPage } from "@/components/kanban/KanbanPage";
import { McpDiscoveryPage } from "@/components/mcp/McpDiscoveryPage";
import UnifiedMcpPanel from "@/components/mcp/UnifiedMcpPanel";
import { MethodologyPage } from "@/components/methodology/MethodologyPage";
import PermissionsPanel from "@/components/permissions/PermissionsPanel";
import PromptPanel from "@/components/prompts/PromptPanel";
import { ProjectAiConfigPage } from "@/components/projects/ProjectAiConfigPage";
import { QuickStartPage } from "@/components/quickStart/QuickStartPage";
import { SessionManagerPage } from "@/components/sessions/SessionManagerPage";
import { SettingsPageContent } from "@/components/settings/SettingsPage";
import { SkillsPage } from "@/components/skills/SkillsPage";
import UnifiedSkillsPanel from "@/components/skills/UnifiedSkillsPanel";
import { CloudSyncDashboard } from "@/components/sync/CloudSyncDashboard";
import { TeamCollaborationPage } from "@/components/team/TeamCollaborationPage";
import { TokenStatsPage } from "@/components/usage/TokenStatsPage";
import type { AppId } from "@/lib/api";
import type { SettingsNavIntent } from "@/lib/settingsNavigation";
import type { Project } from "@/types/project";
import type { ProjectDetailIntent } from "@/types/projectDetail";
import type { WorkspaceTab } from "@/types/workspace";

import type { PageView } from "./navigation";
import type { PageActionRefs } from "./pageActionRefs";

interface AppPageRouterProps {
  view: PageView;
  refs: PageActionRefs;
  targetApp: AppId;
  projects: Project[];
  selectedProjectId: string | null;
  projectDetailIntent: ProjectDetailIntent | null;
  workspaceTab: WorkspaceTab;
  settingsNavIntent: SettingsNavIntent | null;
  onNavigate: (view: PageView) => void;
  onOpenProxySettings: () => void;
  onOpenSubscriptionAccounts: () => void;
  onOpenAiProviderSettings: () => void;
  onWorkspaceTabChange: (tab: WorkspaceTab) => void;
  onProjectClick: (projectId: string) => void;
  /**
   * 去「项目资产配置」页配这个项目（会顺手把「当前项目」钉到它）。
   * 和 `onProjectClick`（开抽屉看概览）是两个动作、两个落点 —— 它们以前
   * 挤在同一个回调里靠 `{ assetsTab: true }` 分流。
   */
  onOpenProjectAiConfig: (projectId: string) => void;
  /** 只改「当前项目」，不跳页 —— 供「项目资产配置」页内的切换器使用。 */
  onSelectProject: (projectId: string) => void;
  onProjectRemove: (projectId: string) => void;
  onAddProject: () => void;
  onClearProjectSelection: () => void;
  onProjectsReload: () => void | Promise<void>;
  onSettingsNavIntent: (intent: SettingsNavIntent) => void;
}

export function AppPageRouter({
  view,
  refs,
  targetApp,
  projects,
  selectedProjectId,
  projectDetailIntent,
  workspaceTab,
  settingsNavIntent,
  onNavigate,
  onOpenProxySettings,
  onOpenSubscriptionAccounts,
  onOpenAiProviderSettings,
  onWorkspaceTabChange,
  onProjectClick,
  onOpenProjectAiConfig,
  onSelectProject,
  onProjectRemove,
  onAddProject,
  onClearProjectSelection,
  onProjectsReload,
  onSettingsNavIntent,
}: AppPageRouterProps) {
  const { t } = useTranslation();
  const effectiveTargetApp =
    targetApp === "claude-desktop" ? "claude" : targetApp;

  switch (view) {
    case "simpleConnect":
      return (
        <QuickStartPage
          onOpenSettings={onOpenProxySettings}
          onOpenSubscriptionAccounts={onOpenSubscriptionAccounts}
        />
      );
    case "mcp":
      return <UnifiedMcpPanel ref={refs.mcpPanel} onOpenChange={() => {}} />;
    case "mcpDiscovery":
      return (
        <ErrorBoundary
          fallbackTitle={t("errors.mcpPageLoadFailed")}
          fallbackDescription={t("errors.mcpPageLoadDescription")}
          onGoBack={() => onNavigate("mcp")}
        >
          <McpDiscoveryPage ref={refs.mcpDiscoveryPage} />
        </ErrorBoundary>
      );
    case "prompts":
      return (
        <PromptPanel
          ref={refs.promptPanel}
          open
          onOpenChange={() => {}}
          appId={effectiveTargetApp}
        />
      );
    case "commands":
      return <CommandsPanel ref={refs.commandsPanel} open />;
    case "hooks":
      return <HooksPanel ref={refs.hooksPanel} open />;
    case "convert":
      return <ConvertPage />;
    case "ignore":
      return <IgnorePanel ref={refs.ignorePanel} open />;
    case "permissions":
      return <PermissionsPanel ref={refs.permissionsPanel} open />;
    case "agents":
      return <AgentsPanel ref={refs.agentsPanel} open />;
    case "skills":
      return (
        <UnifiedSkillsPanel
          ref={refs.unifiedSkillsPanel}
          onOpenDiscovery={() => onNavigate("skillsDiscovery")}
          currentApp={
            effectiveTargetApp === "openclaw" ? "claude" : effectiveTargetApp
          }
        />
      );
    case "skillsDiscovery":
      return (
        <SkillsPage
          ref={refs.skillsPage}
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
    case "kanban":
      return (
        <KanbanPage
          projects={projects}
          selectedProjectId={selectedProjectId ?? undefined}
          projectDetailIntent={projectDetailIntent}
          workspaceTab={workspaceTab}
          onWorkspaceTabChange={onWorkspaceTabChange}
          targetApp={effectiveTargetApp}
          onProjectClick={(project) => onProjectClick(project.id)}
          onOpenProjectAiConfig={onOpenProjectAiConfig}
          onProjectRemove={onProjectRemove}
          onAddProject={onAddProject}
          onClearSelection={onClearProjectSelection}
          onOpenSettings={onOpenAiProviderSettings}
          onProjectsReload={onProjectsReload}
        />
      );
    case "projectAiConfig":
      return (
        <ProjectAiConfigPage
          projects={projects}
          selectedProjectId={selectedProjectId}
          onSelectProject={onSelectProject}
          onNavigate={onNavigate}
          onAddProject={onAddProject}
          targetApp={effectiveTargetApp}
        />
      );
    case "tokenStats":
      return <TokenStatsPage />;
    case "methodology":
      return (
        <MethodologyPage
          projects={projects}
          initialProjectId={selectedProjectId}
        />
      );
    case "cloudSync":
      return (
        <CloudSyncDashboard
          onNavigate={onNavigate}
          onSettingsNavIntent={onSettingsNavIntent}
        />
      );
    case "teamCollaboration":
      return <TeamCollaborationPage />;
    case "settings":
      return (
        <SettingsPageContent
          settingsNavIntent={settingsNavIntent}
          defaultTab={settingsNavIntent?.tab ?? "general"}
        />
      );
  }
}
