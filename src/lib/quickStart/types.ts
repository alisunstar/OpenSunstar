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
  haikuModelName: string;
  sonnetModel: string;
  sonnetModelName: string;
  sonnetSupports1m: boolean;
  opusModel: string;
  opusModelName: string;
  opusSupports1m: boolean;
  fableModel: string;
  fableModelName: string;
  fableSupports1m: boolean;
  subagentModel: string;
  subagentSupports1m: boolean;
  fallbackModel: string;
}

export interface QuickStartAdvancedCodex {
  apiFormat: "openai_chat" | "openai_responses";
  defaultModel: string;
}

export interface QuickStartAdvancedGemini {
  /** Gemini 客户端当前按原生 generateContent 协议透传。 */
  apiFormat: "gemini_native";
  baseUrl: string;
  model: string;
}

export interface QuickStartAdvancedDesktop {
  apiFormat: ClaudeApiFormat;
  sonnetModel: string;
  sonnetLabel: string;
  sonnetSupports1m: boolean;
  opusModel: string;
  opusLabel: string;
  opusSupports1m: boolean;
  haikuModel: string;
  haikuLabel: string;
  haikuSupports1m: boolean;
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
  /** preset 库中的稳定内部名称。 */
  name: string;
  /** 快速接入列表中的产品显示名。 */
  displayName?: string;
  nameKey?: string;
  websiteUrl: string;
  apiKeyUrl?: string;
  category?: string;
  icon?: string;
  iconColor?: string;
  isOfficial?: boolean;
  isPartner?: boolean;
  authMode?: "api_key" | "oauth";
  unavailable?: boolean;
  /** 禁用卡片的具体协议原因。 */
  unavailableReason?: string;
  unavailableReasonKey?: string;
  /** 原始预设对象，供 buildProvider 使用 */
  raw: unknown;
}
