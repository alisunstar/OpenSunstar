import { invoke } from "@tauri-apps/api/core";

export type HookEventType =
  | "PreToolUse"
  | "PostToolUse"
  | "Notification"
  | "Stop";

export interface Hook {
  id: string;
  eventType: HookEventType;
  toolPattern: string;
  hookCommand: string;
  timeoutSeconds: number;
  enabledClaude: boolean;
  enabledCodex?: boolean;
  enabledGemini?: boolean;
  enabledOpencode?: boolean;
  enabledHermes?: boolean;
  description?: string;
  sortIndex: number;
  createdAt?: number;
}

export const hooksApi = {
  async getAll(): Promise<Hook[]> {
    return await invoke("get_all_hooks");
  },

  async upsert(hook: Hook): Promise<void> {
    return await invoke("upsert_hook", { hook });
  },

  async delete(id: string): Promise<boolean> {
    return await invoke("delete_hook", { id });
  },

  async toggleApp(
    hookId: string,
    app: string,
    enabled: boolean,
  ): Promise<void> {
    return await invoke("toggle_hook_app", { hookId, app, enabled });
  },

  async sync(): Promise<void> {
    return await invoke("sync_hooks");
  },
};
