import type { ProviderPreset } from "@/config/claudeProviderPresets";
import type { ClaudeDesktopProviderPreset } from "@/config/claudeDesktopProviderPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import type { GeminiProviderPreset } from "@/config/geminiProviderPresets";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import {
  fetchModelsForConfig,
  type FetchedModel,
} from "@/lib/api/model-fetch";
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
    // 自定义默认按 OpenAI 兼容验证（Codex/Gemini 网关）；Claude 自定义走 Anthropic
    if (appId === "claude" || appId === "claude-desktop") {
      return "anthropic";
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
      if (
        raw.apiFormat === "openai_chat" ||
        raw.apiFormat === "openai_responses"
      ) {
        return "openai";
      }
      return "anthropic";
    }
    case "claude-desktop": {
      const raw = preset.raw as ClaudeDesktopProviderPreset;
      if (
        raw.apiFormat === "openai_chat" ||
        raw.apiFormat === "openai_responses"
      ) {
        return "openai";
      }
      return "anthropic";
    }
    case "codex": {
      const raw = preset.raw as CodexProviderPreset;
      return raw.apiFormat === "openai_responses" ? "openai" : "openai";
    }
    case "gemini":
      return "openai";
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
    return fields.customBaseUrl.trim().replace(/\/+$/, "");
  }

  if (selection.mode === "official") {
    return "";
  }

  const preset = resolvePresetByName(appId, selection.presetName);
  if (!preset) return "";

  switch (appId) {
    case "claude": {
      const raw = preset.raw as ProviderPreset;
      const env = raw.settingsConfig?.env as Record<string, string> | undefined;
      return (env?.ANTHROPIC_BASE_URL ?? "").trim().replace(/\/+$/, "");
    }
    case "claude-desktop": {
      const raw = preset.raw as ClaudeDesktopProviderPreset;
      return raw.baseUrl.trim().replace(/\/+$/, "");
    }
    case "codex": {
      const raw = preset.raw as CodexProviderPreset;
      return getCodexBaseUrl({ auth: raw.auth, config: raw.config }) ?? "";
    }
    case "gemini": {
      const raw = preset.raw as GeminiProviderPreset;
      const env = raw.settingsConfig?.env as Record<string, string> | undefined;
      return (
        env?.GOOGLE_GEMINI_BASE_URL ??
        raw.baseURL ??
        ""
      )
        .trim()
        .replace(/\/+$/, "");
    }
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
      message: t("quickStart.error.emptyKey", { defaultValue: "请填写 API Key" }),
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

  const result = await providersApi.verifyProviderKey(baseUrl, apiKey, protocol);

  if (!result.ok) {
    return {
      ok: false,
      message: result.error ?? "未知错误",
      protocol,
      models: [],
    };
  }

  let message =
    result.error ??
    t("quickStart.verifyOk", { defaultValue: "Key 有效！" });

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

      models = await fetchModelsForConfig({
        baseUrl,
        apiKey,
        modelsUrl,
      });

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
  } else {
    message = t("quickStart.verifyOkAnthropic", {
      defaultValue:
        "Key 有效（该供应商不提供模型列表 API，将使用预设默认模型）",
    });
  }

  return { ok: true, message, protocol, models };
}
