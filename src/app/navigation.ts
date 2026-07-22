import type { AppId } from "@/lib/api";

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
  | "kanban"
  | "tokenStats"
  | "methodology"
  | "cloudSync"
  | "teamCollaboration"
  | "settings";

export interface PageMeta {
  titleKey: string;
  defaultTitle: string;
}

export const APP_STORAGE_KEY = "OpenSunstar-ext-last-app";
export const VIEW_STORAGE_KEY = "OpenSunstar-ext-last-view";

export const VALID_APPS: readonly AppId[] = [
  "claude",
  "claude-desktop",
  "codex",
  "gemini",
  "opencode",
  "openclaw",
  "hermes",
];

export const VALID_VIEWS: readonly PageView[] = [
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
  "kanban",
  "tokenStats",
  "methodology",
  "cloudSync",
  "teamCollaboration",
  "settings",
];

export const ALL_VISIBLE_APPS: Record<AppId, boolean> = {
  claude: true,
  "claude-desktop": true,
  codex: true,
  gemini: true,
  opencode: true,
  openclaw: true,
  hermes: true,
};

export const PAGE_META: Record<PageView, PageMeta> = {
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
  skillsDiscovery: {
    titleKey: "skills.discover",
    defaultTitle: "Discover Skills",
  },
  sessions: { titleKey: "sessionManager.title", defaultTitle: "Context" },
  kanban: { titleKey: "workspace.title", defaultTitle: "Workspace" },
  tokenStats: { titleKey: "sidebar.tokenStats", defaultTitle: "AI Tokens" },
  methodology: {
    titleKey: "methodology.title",
    defaultTitle: "Workflow & Governance",
  },
  cloudSync: {
    titleKey: "cloudSyncDashboard.title",
    defaultTitle: "Cloud Sync",
  },
  teamCollaboration: {
    titleKey: "sidebar.section.teamCollab",
    defaultTitle: "Team Collaboration",
  },
  settings: { titleKey: "common.settings", defaultTitle: "Settings" },
};

export const AGENT_ASSET_PAGE_TITLES: Partial<Record<PageView, string>> = {
  commands: "Commands",
  hooks: "Hooks",
  ignore: "Ignore",
  permissions: "Permissions",
  agents: "Subagents",
  convert: "Convert",
};

const AGENT_CONFIG_VIEWS: readonly PageView[] = [
  "mcp",
  "skills",
  "prompts",
  "commands",
  "hooks",
  "ignore",
  "permissions",
  "agents",
];

export function isAgentConfigView(view: PageView): boolean {
  return AGENT_CONFIG_VIEWS.includes(view);
}

export function getInitialApp(): AppId {
  const saved = localStorage.getItem(APP_STORAGE_KEY) as AppId | null;
  return saved !== null && VALID_APPS.includes(saved) ? saved : "claude";
}

export function getInitialView(): PageView {
  const saved = localStorage.getItem(VIEW_STORAGE_KEY);
  if (saved === "syncBackup") return "settings";
  return saved !== null && VALID_VIEWS.includes(saved as PageView)
    ? (saved as PageView)
    : "kanban";
}
