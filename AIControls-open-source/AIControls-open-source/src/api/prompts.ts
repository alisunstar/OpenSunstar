import { invoke } from "@tauri-apps/api/core";

export type PromptType = "image" | "code" | "doc" | "text";

export type PromptFolder = {
  id: string;
  name: string;
  parentId: string | null;
};

export type PromptItem = {
  id: string;
  type: PromptType;
  title: string;
  prompt: string;
  commandName?: string | null;
  commandEnabled?: boolean;
  convertedSkillId?: string | null;
  outputType?: PromptType;
  outputExample?: string;
  relatedLink?: string | null;
  imageDataUrl?: string | null;
  tags: string[];
  note?: string;
  folderId: string;
  createdAt: number;
  updatedAt: number;
};

export type PromptLibraryFile = {
  version: number;
  folders: PromptFolder[];
  items: PromptItem[];
};

export async function getPromptLibrary(): Promise<PromptLibraryFile> {
  return invoke<PromptLibraryFile>("get_prompt_library");
}

export async function savePromptLibrary(library: PromptLibraryFile): Promise<void> {
  await invoke("save_prompt_library", { library });
}

export type ConvertPromptToSkillInput = {
  title: string;
  prompt: string;
  outputType: PromptType;
  outputExample?: string;
  commandName?: string | null;
};

export type ConvertedPromptSkill = {
  id: string;
  title: string;
  description: string;
  path: string;
  createdAt: number;
  updatedAt: number;
};

export async function convertPromptToMySkill(
  input: ConvertPromptToSkillInput,
): Promise<ConvertedPromptSkill> {
  return invoke<ConvertedPromptSkill>("convert_prompt_to_my_skill", {
    title: input.title,
    prompt: input.prompt,
    outputType: input.outputType,
    outputExample: input.outputExample ?? "",
    commandName: input.commandName ?? null,
  });
}

export async function applyPromptCommandToAgent(input: {
  agentId: string;
  title: string;
  prompt: string;
  commandName: string;
}): Promise<{ path: string }> {
  const path = await invoke<string>("apply_prompt_command_to_agent", {
    agentId: input.agentId,
    title: input.title,
    prompt: input.prompt,
    commandName: input.commandName,
  });
  return { path };
}
