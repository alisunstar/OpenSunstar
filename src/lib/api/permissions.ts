import { invoke } from "@tauri-apps/api/core";

export type PermissionType = "allowedTools" | "deniedTools" | "autoApprove";

export interface ToolPermission {
  id: string;
  permissionType: PermissionType;
  toolPattern: string;
  enabledClaude: boolean;
  description?: string;
  sortIndex: number;
  createdAt?: number;
}

export interface PermissionPreset {
  id: string;
  label: string;
  description: string;
}

export const permissionsApi = {
  async getAll(): Promise<ToolPermission[]> {
    return await invoke("get_all_tool_permissions");
  },

  async upsert(permission: ToolPermission): Promise<void> {
    return await invoke("upsert_tool_permission", { permission });
  },

  async delete(id: string): Promise<boolean> {
    return await invoke("delete_tool_permission", { id });
  },

  async sync(): Promise<void> {
    return await invoke("sync_tool_permissions");
  },

  async getPresets(): Promise<PermissionPreset[]> {
    return await invoke("get_permission_presets");
  },

  async applyPreset(presetId: string): Promise<void> {
    return await invoke("apply_permission_preset", { presetId });
  },
};
