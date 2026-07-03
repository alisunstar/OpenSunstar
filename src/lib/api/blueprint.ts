import { invoke } from "@tauri-apps/api/core";

export interface Blueprint {
  id: string;
  name: string;
  description: string;
  projectType: string;
  targetApp: string;
  linkAllMcpForTarget?: boolean;
  linkAllSkillsForTarget?: boolean;
  linkAllPromptsForTarget?: boolean;
  linkAllCommandsForTarget?: boolean;
  linkAllHooksForTarget?: boolean;
  linkAllIgnoreForTarget?: boolean;
  linkAllPermissionsForTarget?: boolean;
  linkAllSubagentsForTarget?: boolean;
}

export interface BlueprintLinkAction {
  assetType: string;
  assetId: string;
  appType?: string | null;
  action: string;
}

export interface BlueprintApplyPreview {
  blueprintId: string;
  blueprintName: string;
  targetApp: string;
  toLink: BlueprintLinkAction[];
  warnings: string[];
}

export const blueprintApi = {
  async list(): Promise<Blueprint[]> {
    return await invoke<Blueprint[]>("list_project_blueprints");
  },

  async get(id: string): Promise<Blueprint> {
    return await invoke<Blueprint>("get_project_blueprint", { id });
  },

  async previewApply(
    projectId: string,
    blueprintId: string,
  ): Promise<BlueprintApplyPreview> {
    return await invoke<BlueprintApplyPreview>("preview_apply_project_blueprint", {
      projectId,
      blueprintId,
    });
  },

  async apply(
    projectId: string,
    blueprintId: string,
  ): Promise<BlueprintApplyPreview> {
    return await invoke<BlueprintApplyPreview>("apply_project_blueprint", {
      projectId,
      blueprintId,
    });
  },

  async exportBaselineSnapshot(projectId: string): Promise<string> {
    return await invoke<string>("export_project_baseline_snapshot", {
      projectId,
    });
  },
};
