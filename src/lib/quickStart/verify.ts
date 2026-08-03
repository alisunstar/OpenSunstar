import type { ProviderPreset } from "@/config/claudeProviderPresets";
import type { ClaudeDesktopProviderPreset } from "@/config/claudeDesktopProviderPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import type { GeminiProviderPreset } from "@/config/geminiProviderPresets";
import type { OpenCodeProviderPreset } from "@/config/opencodeProviderPresets";
import type { OpenClawProviderPreset } from "@/config/openclawProviderPresets";
import type { HermesProviderPreset } from "@/config/hermesProviderPresets";
import type { GrokBuildProviderPreset } from "@/config/grokBuildProviderPresets";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import { fetchModelsForConfig, type FetchedModel } from "@/lib/api/model-fetch";
import { providersApi, type VerifyProtocol } from "@/lib/api";
import { getCodexBaseUrl } from "@/utils/providerConfigUtils";
import type { QuickStartFormFields, QuickStartSelection } from "./types";
import { resolvePresetByName } from "./resolvePresets";

export function inferVerifyProtocol(
  appId: QuickStartAppId,
  selection: QuickStartSelection,
  fields: QuickStartFormFields,
): VerifyProtocol {
  if (selection.mode === "custom") {
    if (appId === "claude") {
      const format = fields.advancedClaude?.apiFormat;
      return format === "anthropic"
        ? "anthropic"
        : format === "gemini_native"
          ? "gemini"
          : "openai";
    }
    if (appId === "claude-desktop") {
      const format = fields.advancedDesktop?.apiFormat;
      return format === "anthropic"
        ? "anthropic"
        : format === "gemini_native"
          ? "gemini"
          : "openai";
    }
    if (appId === "gemini") {
      return "gemini";
    }
    if (appId === "grokbuild") {
      return "openai";
    }
    return "openai";
  }

  if (selection.mode === "official") {
    return "anthropic";
  }

  const preset = resolvePresetByName(appId, selection.presetName);
  if (!preset) return "anthropic";

  switch (appId) {
    case "claude": {
      const raw = preset.raw as ProviderPreset;
      const apiFormat = fields.advancedClaude?.apiFormat ?? raw.apiFormat;
      if (apiFormat === "gemini_native") return "gemini";
      if (apiFormat === "openai_chat" || apiFormat === "openai_responses") {
        return "openai";
      }
      return "anthropic";
    }
    case "claude-desktop": {
      const raw = preset.raw as ClaudeDesktopProviderPreset;
      const apiFormat = fields.advancedDesktop?.apiFormat ?? raw.apiFormat;
      if (apiFormat === "gemini_native") return "gemini";
      if (apiFormat === "openai_chat" || apiFormat === "openai_responses") {
        return "openai";
      }
      return "anthropic";
    }
    case "codex":
      return "openai";
    case "gemini":
      return (preset.raw as GeminiProviderPreset).apiFormat === "gemini_native"
        ? "gemini"
        : "openai";
    case "opencode": {
      const npm = (preset.raw as OpenCodeProviderPreset).settingsConfig.npm;
      return npm === "@ai-sdk/anthropic"
        ? "anthropic"
        : npm === "@ai-sdk/google"
          ? "gemini"
          : "openai";
    }
    case "openclaw": {
      const protocol = (preset.raw as OpenClawProviderPreset).settingsConfig
        .api;
      return protocol === "anthropic-messages"
        ? "anthropic"
        : protocol === "google-generative-ai"
          ? "gemini"
          : "openai";
    }
    case "hermes":
      return (preset.raw as HermesProviderPreset).settingsConfig.api_mode ===
        "anthropic_messages"
        ? "anthropic"
        : "openai";
    default:
      return "anthropic";
  }
}

export function resolveVerifyBaseUrl(
  appId: QuickStartAppId,
  selection: QuickStartSelection,
  fields: QuickStartFormFields,
): string {
  if (selection.mode === "custom") {
    const advancedBaseUrl =
      appId === "gemini" ? fields.advancedGemini?.baseUrl : undefined;
    return (advancedBaseUrl || fields.customBaseUrl).trim().replace(/\/+$/, "");
  }

  if (selection.mode === "official") {
    return "";
  }

  const preset = resolvePresetByName(appId, selection.presetName);
  if (!preset) return "";

  switch (appId) {
    case "claude": {
      const raw = preset.raw as ProviderPreset;
      const settingsConfig = raw.settingsConfig as
        | { env?: Record<string, string> }
        | undefined;
      const env = settingsConfig?.env;
      return (env?.ANTHROPIC_BASE_URL ?? "").trim().replace(/\/+$/, "");
    }
    case "claude-desktop": {
      const raw = preset.raw as ClaudeDesktopProviderPreset;
      return raw.baseUrl.trim().replace(/\/+$/, "");
    }
    case "codex": {
      const raw = preset.raw as CodexProviderPreset;
      return (
        getCodexBaseUrl({
          settingsConfig: { auth: raw.auth, config: raw.config },
        }) ??
        raw.endpointCandidates?.[0] ??
        ""
      )
        .trim()
        .replace(/\/+$/, "");
    }
    case "gemini": {
      const raw = preset.raw as GeminiProviderPreset;
      const settingsConfig = raw.settingsConfig as
        | { env?: Record<string, string> }
        | undefined;
      const env = settingsConfig?.env;
      return (
        fields.advancedGemini?.baseUrl ??
        env?.GOOGLE_GEMINI_BASE_URL ??
        raw.baseURL ??
        ""
      )
        .trim()
        .replace(/\/+$/, "");
    }
    case "grokbuild": {
      const raw = preset.raw as GrokBuildProviderPreset;
      return (
        raw.endpointCandidates?.[0] ??
        raw.config.match(/base_url\s*=\s*["']([^"']+)["']/)?.[1] ??
        ""
      )
        .trim()
        .replace(/\/+$/, "");
    }
    case "opencode": {
      const raw = preset.raw as OpenCodeProviderPreset;
      return (raw.settingsConfig.options?.baseURL ?? "")
        .trim()
        .replace(/\/+$/, "");
    }
    case "openclaw":
      return (
        (preset.raw as OpenClawProviderPreset).settingsConfig.baseUrl ?? ""
      )
        .trim()
        .replace(/\/+$/, "");
    case "hermes":
      return (
        (preset.raw as HermesProviderPreset).settingsConfig.base_url ?? ""
      )
        .trim()
        .replace(/\/+$/, "");
    default:
      return "";
  }
}

export interface VerifyKeyOutcome {
  ok: boolean;
  message: string;
  protocol: VerifyProtocol;
  models: FetchedModel[];
}

export async function verifyQuickStartKey(
  appId: QuickStartAppId,
  selection: QuickStartSelection,
  fields: QuickStartFormFields,
  t: (key: string, opts?: Record<string, unknown>) => string,
): Promise<VerifyKeyOutcome> {
  const protocol = inferVerifyProtocol(appId, selection, fields);
  const baseUrl = resolveVerifyBaseUrl(appId, selection, fields);
  const apiKey = fields.apiKey.trim();

  if (!apiKey) {
    return {
      ok: false,
      message: t("quickStart.error.emptyKey", {
        defaultValue: "请填写 API Key",
      }),
      protocol,
      models: [],
    };
  }

  if (!baseUrl) {
    return {
      ok: false,
      message: t("quickStart.error.noBaseUrl", {
        defaultValue: "缺少 Base URL，无法验证",
      }),
      protocol,
      models: [],
    };
  }

  const result = await providersApi.verifyProviderKey(
    baseUrl,
    apiKey,
    protocol,
  );

  if (!result.ok) {
    return {
      ok: false,
      message: result.error ?? "未知错误",
      protocol,
      models: [],
    };
  }

  let message =
    result.error ?? t("quickStart.verifyOk", { defaultValue: "Key 有效！" });

  let models: FetchedModel[] = [];

  if (protocol === "openai") {
    try {
      const preset =
        selection.mode === "preset"
          ? resolvePresetByName(appId, selection.presetName)
          : null;
      const modelsUrl =
        appId === "claude" && preset?.raw
          ? (preset.raw as ProviderPreset).modelsUrl
          : undefined;

      models = await fetchModelsForConfig(baseUrl, apiKey, false, modelsUrl);

      if (models.length > 0) {
        message = t("quickStart.verifyOkWithModels", {
          count: models.length,
          defaultValue: `Key 有效，已获取 ${models.length} 个模型`,
        });
      }
    } catch {
      message = t("quickStart.verifyOkNoModelList", {
        defaultValue: "Key 有效（模型列表拉取失败，将使用预设默认模型）",
      });
    }
  } else if (protocol === "anthropic") {
    message = t("quickStart.verifyOkAnthropic", {
      defaultValue:
        "Key 有效（该供应商不提供模型列表 API，将使用预设默认模型）",
    });
  } else {
    message = t("quickStart.verifyOkGemini", {
      defaultValue: "Key 有效，Gemini 原生接口验证通过",
    });
  }

  return { ok: true, message, protocol, models };
}
