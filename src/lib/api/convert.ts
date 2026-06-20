import { invoke } from "@tauri-apps/api/core";
import type { BridgePreview } from "./bridge";

export interface ConvertSourceItem {
  contentType: string;
  label: string;
  path: string;
  exists: boolean;
  content?: string | null;
}

export interface ConvertApplyRequest {
  sourceApp: string;
  targetApp: string;
  contentType: string;
  content: string;
  overwrite: boolean;
}

export interface ConvertApplyResult {
  writtenPaths: string[];
  warnings: string[];
}

export async function detectConvertSources(
  sourceApp: string,
): Promise<ConvertSourceItem[]> {
  return invoke("detect_convert_sources", { sourceApp });
}

export async function previewConvert(
  sourceApp: string,
  targetApp: string,
  content: string,
  contentType: string,
): Promise<BridgePreview> {
  return invoke("preview_convert", {
    sourceApp,
    targetApp,
    content,
    contentType,
  });
}

export async function applyConvert(
  req: ConvertApplyRequest,
): Promise<ConvertApplyResult> {
  return invoke("apply_convert", { req });
}
