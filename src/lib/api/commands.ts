import { invoke } from "@tauri-apps/api/core";

export interface Command {
  id: string;
  name: string;
  description?: string;
  content: string;
  arguments: string;
  enabledClaude: boolean;
  enabledCodex: boolean;
  enabledGemini: boolean;
  enabledOpencode: boolean;
  enabledHermes: boolean;
  createdAt?: number;
  updatedAt?: number;
}

export const commandsApi = {
  async getAll(): Promise<Record<string, Command>> {
    return await invoke("get_all_commands");
  },

  async upsert(command: Command): Promise<void> {
    return await invoke("upsert_command", { command });
  },

  async delete(id: string): Promise<boolean> {
    return await invoke("delete_command", { id });
  },

  async toggleApp(
    commandId: string,
    app: string,
    enabled: boolean,
  ): Promise<void> {
    return await invoke("toggle_command_app", { commandId, app, enabled });
  },
};
