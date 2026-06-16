import { invoke } from "@tauri-apps/api/core";

export type MySkillItem = {
  id: string;
  title: string;
  description: string;
  path: string;
  sourcePath?: string | null;
  sourceKind?: "prompt" | null;
  createdAt: number;
  updatedAt: number;
};

export type MySkillsLibraryFile = {
  version: number;
  items: MySkillItem[];
};

export async function getMySkillsLibrary(): Promise<MySkillsLibraryFile> {
  return invoke<MySkillsLibraryFile>("get_my_skills_library");
}

export async function addSkillToMyLibrary(sourcePath: string): Promise<MySkillItem> {
  return invoke<MySkillItem>("add_skill_to_my_library", { sourcePath });
}

export async function removeMySkill(id: string): Promise<void> {
  await invoke("remove_my_skill", { id });
}
