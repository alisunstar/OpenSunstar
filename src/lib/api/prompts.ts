import { invoke } from "@tauri-apps/api/core";
import type { AppId } from "./types";

export interface Prompt {
  id: string;
  name: string;
  content: string;
  description?: string;
  enabled: boolean;
  targets?: string;
  globs?: string;
  priority?: number;
  isFragment?: boolean;
  parentPromptId?: string | null;
  createdAt?: number;
  updatedAt?: number;
}

export interface PromptActivationPreview {
  filePath: string;
  currentContent: string;
  newContent: string;
}

export const promptsApi = {
  async getPrompts(app: AppId): Promise<Record<string, Prompt>> {
    return await invoke("get_prompts", { app });
  },

  async upsertPrompt(app: AppId, id: string, prompt: Prompt): Promise<void> {
    return await invoke("upsert_prompt", { app, id, prompt });
  },

  async deletePrompt(app: AppId, id: string): Promise<void> {
    return await invoke("delete_prompt", { app, id });
  },

  async enablePrompt(app: AppId, id: string): Promise<void> {
    return await invoke("enable_prompt", { app, id });
  },

  async previewActivation(
    app: AppId,
    id: string,
  ): Promise<PromptActivationPreview> {
    return await invoke("preview_prompt_activation", { app, id });
  },

  async importFromFile(app: AppId): Promise<string> {
    return await invoke("import_prompt_from_file", { app });
  },

  async getCurrentFileContent(app: AppId): Promise<string | null> {
    return await invoke("get_current_prompt_file_content", { app });
  },
};

export const dryRunApi = {
  async getMode(): Promise<boolean> {
    return await invoke("get_dry_run_mode");
  },

  async setMode(enabled: boolean): Promise<void> {
    return await invoke("set_dry_run_mode", { enabled });
  },
};
