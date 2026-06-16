import { invoke } from "@tauri-apps/api/core";

export type ResourceItem = {
  id: string;
  title: string;
  url: string | null;
  tags: string[];
  note: string;
  pinned: boolean;
  createdAt: number;
  updatedAt: number;
};

export type ResourceLibraryFile = {
  version: number;
  items: ResourceItem[];
};

export async function getResourceLibrary(): Promise<ResourceLibraryFile> {
  return invoke<ResourceLibraryFile>("get_resource_library");
}

export async function saveResourceLibrary(library: ResourceLibraryFile): Promise<void> {
  await invoke("save_resource_library", { library });
}
