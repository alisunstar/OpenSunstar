/** 8 类项目 AI 资产类型 */
export type ProjectAssetType =
  | "mcp"
  | "skill"
  | "prompt"
  | "command"
  | "hook"
  | "ignore"
  | "permission"
  | "subagent";

/** 扩展资产类型（`project_asset_links` 中 command~subagent；mcp/skill/prompt 亦存同表） */
export type ExtendedProjectAssetType =
  | "command"
  | "hook"
  | "ignore"
  | "permission"
  | "subagent";

export type ProjectAssetScope = "project" | "global_baseline" | "repo_detected";

export type ProjectAssetSource =
  | "manual"
  | "imported"
  | "detected"
  | "template";

export interface ProjectAssetLink {
  project_id: string;
  asset_type: string;
  asset_id: string;
  asset_app_type: string;
  enabled: boolean;
  scope: ProjectAssetScope;
  source: ProjectAssetSource;
  created_at: number;
  updated_at: number;
}

export interface ProjectAllAssetCounts {
  mcp: number;
  skills: number;
  prompts: number;
  commands: number;
  hooks: number;
  ignore: number;
  permissions: number;
  subagents: number;
}

export type ProjectAssetSection = ProjectAssetType;
