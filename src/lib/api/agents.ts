import { invoke } from "@tauri-apps/api/core";

export interface Agent {
  id: string;
  name: string;
  description?: string;
  content: string;
  enabledClaude: boolean;
  enabledCodex: boolean;
  enabledGemini: boolean;
  enabledOpencode: boolean;
  enabledHermes: boolean;
  createdAt?: number;
  updatedAt?: number;
}

export const agentsApi = {
  async getAll(): Promise<Record<string, Agent>> {
    return await invoke("get_all_agents");
  },

  async upsert(agent: Agent): Promise<void> {
    return await invoke("upsert_agent", { agent });
  },

  async delete(id: string): Promise<boolean> {
    return await invoke("delete_agent", { id });
  },

  async toggleApp(
    agentId: string,
    app: string,
    enabled: boolean,
  ): Promise<void> {
    return await invoke("toggle_agent_app", { agentId, app, enabled });
  },
};
