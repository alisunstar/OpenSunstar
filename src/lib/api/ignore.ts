import { invoke } from "@tauri-apps/api/core";
import type { AppId } from "./types";

export interface IgnoreRule {
  id: string;
  pattern: string;
  description?: string;
  enabledClaude: boolean;
  enabledCodex: boolean;
  enabledGemini: boolean;
  enabledOpencode: boolean;
  enabledHermes: boolean;
  sortIndex: number;
  createdAt?: number;
}

export const ignoreApi = {
  async getAll(): Promise<IgnoreRule[]> {
    return await invoke("get_all_ignore_rules");
  },

  async upsert(rule: IgnoreRule): Promise<void> {
    return await invoke("upsert_ignore_rule", { rule });
  },

  async delete(id: string): Promise<boolean> {
    return await invoke("delete_ignore_rule", { id });
  },

  async toggleApp(ruleId: string, app: AppId, enabled: boolean): Promise<void> {
    return await invoke("toggle_ignore_app", { ruleId, app, enabled });
  },

  async importFromGitignore(filePath: string): Promise<number> {
    return await invoke("import_ignore_from_gitignore", { filePath });
  },

  async sync(): Promise<void> {
    return await invoke("sync_ignore_rules");
  },
};
