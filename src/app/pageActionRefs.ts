import { useRef, type RefObject } from "react";
import type { AgentsPanelHandle } from "@/components/agents/AgentsPanel";
import type { CommandsPanelHandle } from "@/components/commands/CommandsPanel";
import type { HooksPanelHandle } from "@/components/hooks/HooksPanel";
import type { IgnorePanelHandle } from "@/components/ignore/IgnorePanel";
import type { McpDiscoveryPageHandle } from "@/components/mcp/McpDiscoveryPage";
import type { UnifiedMcpPanelHandle } from "@/components/mcp/UnifiedMcpPanel";
import type { PermissionsPanelHandle } from "@/components/permissions/PermissionsPanel";
import type { PromptPanelHandle } from "@/components/prompts/PromptPanel";
import type { SkillsPageHandle } from "@/components/skills/SkillsPage";
import type { UnifiedSkillsPanelHandle } from "@/components/skills/UnifiedSkillsPanel";

export interface PageActionRefs {
  commandsPanel: RefObject<CommandsPanelHandle>;
  hooksPanel: RefObject<HooksPanelHandle>;
  ignorePanel: RefObject<IgnorePanelHandle>;
  permissionsPanel: RefObject<PermissionsPanelHandle>;
  agentsPanel: RefObject<AgentsPanelHandle>;
  promptPanel: RefObject<PromptPanelHandle>;
  mcpPanel: RefObject<UnifiedMcpPanelHandle>;
  mcpDiscoveryPage: RefObject<McpDiscoveryPageHandle>;
  unifiedSkillsPanel: RefObject<UnifiedSkillsPanelHandle>;
  skillsPage: RefObject<SkillsPageHandle>;
}

/**
 * The legacy panels expose imperative handles. They are intentionally isolated
 * here so new routes do not add refs or action knowledge to App.tsx.
 */
export function usePageActionRefs(): PageActionRefs {
  return {
    commandsPanel: useRef<CommandsPanelHandle>(null),
    hooksPanel: useRef<HooksPanelHandle>(null),
    ignorePanel: useRef<IgnorePanelHandle>(null),
    permissionsPanel: useRef<PermissionsPanelHandle>(null),
    agentsPanel: useRef<AgentsPanelHandle>(null),
    promptPanel: useRef<PromptPanelHandle>(null),
    mcpPanel: useRef<UnifiedMcpPanelHandle>(null),
    mcpDiscoveryPage: useRef<McpDiscoveryPageHandle>(null),
    unifiedSkillsPanel: useRef<UnifiedSkillsPanelHandle>(null),
    skillsPage: useRef<SkillsPageHandle>(null),
  };
}
