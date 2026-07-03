import { invoke } from "@tauri-apps/api/core";
import type {
  ExtendedProjectAssetType,
  ProjectAllAssetCounts,
  ProjectAssetLink,
} from "@/types/projectAsset";

/** 项目信息 */
export interface Project {
  id: string;
  name: string;
  path: string;
  git_remote_url?: string | null;
  created_at: number;
  updated_at: number;
  target_app?: string | null;
  blueprint_id?: string | null;
}

/** 项目关联的配置项（MCP/Skills） */
export interface ProjectConfigLink {
  project_id: string;
  config_id: string;
  enabled: boolean;
  created_at: number;
}

/** 项目关联的 Prompt 配置项 */
export interface ProjectPromptLink {
  project_id: string;
  prompt_id: string;
  prompt_app_type: string;
  enabled: boolean;
  created_at: number;
}

export const projectsApi = {
  // ========== Projects CRUD ==========

  /** 获取所有项目 */
  async getAll(): Promise<Project[]> {
    return await invoke("get_all_projects");
  },

  /** 根据 ID 获取项目 */
  async getById(id: string): Promise<Project | null> {
    return await invoke("get_project", { id });
  },

  /** 根据路径获取项目 */
  async getByPath(path: string): Promise<Project | null> {
    return await invoke("get_project_by_path", { path });
  },

  /** 创建或更新项目 */
  async upsert(project: Project): Promise<void> {
    return await invoke("upsert_project", { project });
  },

  /** 删除项目（级联删除关联） */
  async delete(id: string): Promise<boolean> {
    return await invoke("delete_project", { id });
  },

  async setTargetApp(
    projectId: string,
    targetApp: string | null,
  ): Promise<void> {
    return await invoke("set_project_target_app", {
      projectId,
      targetApp,
    });
  },

  // ========== Project × MCP Servers ==========

  /** 获取项目关联的 MCP 服务器列表 */
  async getMcpServers(projectId: string): Promise<ProjectConfigLink[]> {
    return await invoke("get_project_mcp_servers", { projectId });
  },

  /** 关联 MCP 服务器到项目 */
  async linkMcpServer(
    projectId: string,
    mcpServerId: string,
    enabled: boolean = true,
  ): Promise<void> {
    return await invoke("link_project_mcp_server", {
      projectId,
      mcpServerId,
      enabled,
    });
  },

  /** 取消 MCP 服务器与项目的关联 */
  async unlinkMcpServer(
    projectId: string,
    mcpServerId: string,
  ): Promise<boolean> {
    return await invoke("unlink_project_mcp_server", {
      projectId,
      mcpServerId,
    });
  },

  /** 批量设置项目的 MCP 服务器关联（替换所有现有关联） */
  async setMcpServers(
    projectId: string,
    mcpServerIds: string[],
  ): Promise<void> {
    return await invoke("set_project_mcp_servers", {
      projectId,
      mcpServerIds,
    });
  },

  // ========== Project × Skills ==========

  /** 获取项目关联的 Skill 列表 */
  async getSkills(projectId: string): Promise<ProjectConfigLink[]> {
    return await invoke("get_project_skills", { projectId });
  },

  /** 关联 Skill 到项目 */
  async linkSkill(
    projectId: string,
    skillId: string,
    enabled: boolean = true,
  ): Promise<void> {
    return await invoke("link_project_skill", {
      projectId,
      skillId,
      enabled,
    });
  },

  /** 取消 Skill 与项目的关联 */
  async unlinkSkill(projectId: string, skillId: string): Promise<boolean> {
    return await invoke("unlink_project_skill", { projectId, skillId });
  },

  /** 批量设置项目的 Skill 关联（替换所有现有关联） */
  async setSkills(projectId: string, skillIds: string[]): Promise<void> {
    return await invoke("set_project_skills", { projectId, skillIds });
  },

  // ========== Project × Prompts ==========

  /** 获取项目关联的 Prompt 列表 */
  async getPrompts(projectId: string): Promise<ProjectPromptLink[]> {
    return await invoke("get_project_prompts", { projectId });
  },

  /** 关联 Prompt 到项目 */
  async linkPrompt(
    projectId: string,
    promptId: string,
    promptAppType: string,
    enabled: boolean = true,
  ): Promise<void> {
    return await invoke("link_project_prompt", {
      projectId,
      promptId,
      promptAppType,
      enabled,
    });
  },

  /** 取消 Prompt 与项目的关联 */
  async unlinkPrompt(
    projectId: string,
    promptId: string,
    promptAppType: string,
  ): Promise<boolean> {
    return await invoke("unlink_project_prompt", {
      projectId,
      promptId,
      promptAppType,
    });
  },

  /** 批量设置项目的 Prompt 关联（替换所有现有关联） */
  async setPrompts(
    projectId: string,
    prompts: [string, string][],
  ): Promise<void> {
    return await invoke("set_project_prompts", { projectId, prompts });
  },

  // ========== 项目资产（8 类均存 project_asset_links；此处为扩展 5 类 Tauri 命令）==========

  async getAllAssetCounts(projectId: string): Promise<ProjectAllAssetCounts> {
    return await invoke("get_project_all_asset_counts", { projectId });
  },

  async getAssetLinks(
    projectId: string,
    assetType?: ExtendedProjectAssetType,
  ): Promise<ProjectAssetLink[]> {
    return await invoke("get_project_asset_links", { projectId, assetType });
  },

  async linkAsset(
    projectId: string,
    assetType: ExtendedProjectAssetType,
    assetId: string,
    enabled = true,
    assetAppType = "",
  ): Promise<void> {
    return await invoke("link_project_asset", {
      projectId,
      assetType,
      assetId,
      assetAppType: assetAppType || null,
      enabled,
    });
  },

  async unlinkAsset(
    projectId: string,
    assetType: ExtendedProjectAssetType,
    assetId: string,
    assetAppType = "",
  ): Promise<boolean> {
    return await invoke("unlink_project_asset", {
      projectId,
      assetType,
      assetId,
      assetAppType: assetAppType || null,
    });
  },
};
