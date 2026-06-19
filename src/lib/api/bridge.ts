import { invoke } from "@tauri-apps/api/core";

export interface BridgePreview {
  convertedContent: string;
  unmappedSections: string[];
  warnings: string[];
}

export interface BridgeCandidate {
  id: string;
  name: string;
  appType: string;
  contentPreview: string;
  bridgeSource: string | null;
}

export async function bridgePrompt(
  sourceApp: string,
  targetApp: string,
  id: string,
) {
  return invoke("bridge_prompt", { sourceApp, targetApp, id });
}

export async function getBridgeablePrompts(
  sourceApp: string,
): Promise<BridgeCandidate[]> {
  return invoke("get_bridgeable_prompts", { sourceApp });
}

export async function pushBridgeChanges(
  sourceApp: string,
  sourceId: string,
) {
  return invoke("push_bridge_changes", { sourceApp, sourceId });
}

export async function unlinkBridge(appType: string, id: string) {
  return invoke("unlink_bridge", { appType, id });
}

export async function previewBridge(
  sourceApp: string,
  targetApp: string,
  content: string,
): Promise<BridgePreview> {
  return invoke("preview_bridge", { sourceApp, targetApp, content });
}
