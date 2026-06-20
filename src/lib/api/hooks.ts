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

  async sync(): Promise<void> {
    return await invoke("sync_hooks");
  },
};
