import type { ClaudeApiFormat } from "@/types";
import type { QuickStartAppId } from "@/config/quickStartCurated";

/** 已选中的预设或自定义供应商。 */
export type QuickStartSelection =
  | {
      mode: "preset";
      appId: QuickStartAppId;
      presetName: string;
      isOfficial: boolean;
    }
  | {
      mode: "custom";
      appId: QuickStartAppId;
    }
  | {
      mode: "official";
      appId: QuickStartAppId;
      presetName: string;
    };

export interface QuickStartAdvancedClaude {
  apiFormat: ClaudeApiFormat;
  apiKeyField: "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
}

export interface QuickStartAdvancedCodex {
  apiFormat: "openai_chat" | "openai_responses";
  defaultModel: string;
}

export interface QuickStartAdvancedGemini {
  baseUrl: string;
  model: string;
}

export interface QuickStartAdvancedDesktop {
  apiFormat: ClaudeApiFormat;
  upstreamModel: string;
}

export interface QuickStartFormFields {
  apiKey: string;
  customName: string;
  customBaseUrl: string;
  customModel: string;
  advancedClaude?: QuickStartAdvancedClaude;
  advancedCodex?: QuickStartAdvancedCodex;
  advancedGemini?: QuickStartAdvancedGemini;
  advancedDesktop?: QuickStartAdvancedDesktop;
}

export interface ResolvedQuickStartPreset {
  name: string;
  nameKey?: string;
  websiteUrl: string;
  apiKeyUrl?: string;
  category?: string;
  icon?: string;
  iconColor?: string;
  isOfficial?: boolean;
  isPartner?: boolean;
  /** 原始预设对象，供 buildProvider 使用 */
  raw: unknown;
}
