import type { ProviderPreset } from "@/config/claudeProviderPresets";
import {
  CLAUDE_DESKTOP_ROLE_ROUTE_IDS,
  type ClaudeDesktopProviderPreset,
} from "@/config/claudeDesktopProviderPresets";
import {
  generateThirdPartyAuth,
  generateThirdPartyConfig,
  type CodexProviderPreset,
} from "@/config/codexProviderPresets";
import { getCodexBaseUrl } from "@/utils/providerConfigUtils";
import type { GeminiProviderPreset } from "@/config/geminiProviderPresets";
import type { OpenCodeProviderPreset } from "@/config/opencodeProviderPresets";
import {
  rebaseOpenClawSuggestedDefaults,
  type OpenClawProviderPreset,
} from "@/config/openclawProviderPresets";
import type { HermesProviderPreset } from "@/config/hermesProviderPresets";
import type { GrokBuildProviderPreset } from "@/config/grokBuildProviderPresets";
import { GROK_BUILD_DEFAULT_MODEL } from "@/utils/grokBuildConfig";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type {
  ClaudeDesktopModelRoute,
  Provider,
  ProviderCategory,
  ProviderMeta,
} from "@/types";
import type { QuickStartFormFields, QuickStartSelection } from "./types";
import { resolvePresetByName } from "./resolvePresets";

export interface QuickStartOpenClawSuggestedDefaults {
  model?: { primary: string; fallbacks?: string[] };
  models?: Record<string, { alias?: string }>;
}

export type QuickStartProviderInput = Omit<Provider, "id"> & {
  ensureClaudeDesktopOfficialSeed?: boolean;
  openclawSuggestedDefaults?: QuickStartOpenClawSuggestedDefaults;
};

function grokTomlString(value: string): string {
  return JSON.stringify(value);
}

function buildGrokConfig(
  providerName: string,
  baseUrl: string,
  model: string,
  apiKey: string,
): string {
  const profile = "custom";
  return `[models]\ndefault = ${grokTomlString(profile)}\n\n[model.${profile}]\nmodel = ${grokTomlString(model || GROK_BUILD_DEFAULT_MODEL)}\nbase_url = ${grokTomlString(baseUrl)}\nname = ${grokTomlString(providerName)}\napi_key = ${grokTomlString(apiKey)}\napi_backend = "responses"\ncontext_window = 500000\n`;
}

function parseGrokPreset(preset: GrokBuildProviderPreset): {
  baseUrl: string;
  model: string;
} {
  const baseUrl =
    preset.endpointCandidates?.[0] ??
    preset.config.match(/base_url\s*=\s*["']([^"']+)["']/)?.[1] ??
    "";
  const model =
    preset.config.match(/^model\s*=\s*["']([^"']+)["']/m)?.[1] ??
    GROK_BUILD_DEFAULT_MODEL;
  return { baseUrl, model };
}

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function stripOneM(model: string): string {
  return model
    .trim()
    .replace(/\[1m\]$/i, "")
    .trim();
}

function withOneM(model: string, enabled: boolean): string {
  const normalized = stripOneM(model);
  return normalized && enabled ? `${normalized}[1M]` : normalized;
}

function setOptionalEnv(
  env: Record<string, string>,
  key: string,
  value: string,
): void {
  const normalized = value.trim();
  if (normalized) env[key] = normalized;
  else delete env[key];
}

function buildMetaCustomEndpoints(urls: string[]): ProviderMeta | undefined {
  const filtered = urls
    .map((u) => u.trim().replace(/\/+$/, ""))
    .filter((u) => u.startsWith("http"));
  if (filtered.length === 0) return undefined;
  const now = Date.now();
  const custom_endpoints: ProviderMeta["custom_endpoints"] = {};
  for (const url of filtered) {
    custom_endpoints![url] = { url, addedAt: now };
  }
  return { custom_endpoints };
}

function openCodeApiFormat(
  preset: OpenCodeProviderPreset,
): NonNullable<ProviderMeta["apiFormat"]> {
  if (preset.settingsConfig.npm === "@ai-sdk/anthropic") return "anthropic";
  if (preset.settingsConfig.npm === "@ai-sdk/google") return "gemini_native";
  return "openai_chat";
}

function openClawApiFormat(
  preset: OpenClawProviderPreset,
): NonNullable<ProviderMeta["apiFormat"]> {
  if (preset.settingsConfig.api === "anthropic-messages") return "anthropic";
  if (preset.settingsConfig.api === "google-generative-ai") {
    return "gemini_native";
  }
  return "openai_chat";
}

function hermesApiFormat(
  preset: HermesProviderPreset,
): NonNullable<ProviderMeta["apiFormat"]> {
  return preset.settingsConfig.api_mode === "anthropic_messages"
    ? "anthropic"
    : "openai_chat";
}

export function buildQuickStartProviderInput(
  appId: QuickStartAppId,
  selection: QuickStartSelection,
  fields: QuickStartFormFields,
  displayName: string,
  providerId?: string,
): QuickStartProviderInput {
  if (selection.mode === "official" && appId === "claude-desktop") {
    return {
      name: displayName,
      settingsConfig: { env: {} },
      category: "official",
      websiteUrl: "https://claude.ai/download",
      createdAt: Date.now(),
      ensureClaudeDesktopOfficialSeed: true,
    };
  }

  if (selection.mode === "custom") {
    return buildCustomProvider(appId, fields, displayName);
  }

  if (selection.mode !== "preset") {
    throw new Error("Invalid selection for provider build");
  }

  const preset = resolvePresetByName(appId, selection.presetName);
  if (!preset) {
    throw new Error(`Preset not found: ${selection.presetName}`);
  }

  switch (appId) {
    case "claude":
      return buildClaudeFromPreset(
        preset.raw as ProviderPreset,
        fields,
        displayName,
      );
    case "claude-desktop":
      return buildDesktopFromPreset(
        preset.raw as ClaudeDesktopProviderPreset,
        fields,
        displayName,
      );
    case "codex":
      return buildCodexFromPreset(
        preset.raw as CodexProviderPreset,
        fields,
        displayName,
      );
    case "gemini":
      return buildGeminiFromPreset(
        preset.raw as GeminiProviderPreset,
        fields,
        displayName,
      );
    case "grokbuild":
      return buildGrokBuildFromPreset(
        preset.raw as GrokBuildProviderPreset,
        fields,
        displayName,
      );
    case "opencode":
      return buildOpenCodeFromPreset(
        preset.raw as OpenCodeProviderPreset,
        fields,
        displayName,
      );
    case "openclaw":
      return buildOpenClawFromPreset(
        preset.raw as OpenClawProviderPreset,
        fields,
        displayName,
        providerId,
      );
    case "hermes":
      return buildHermesFromPreset(
        preset.raw as HermesProviderPreset,
        fields,
        displayName,
      );
    default:
      throw new Error(`Unsupported app: ${appId}`);
  }
}

function buildGrokBuildFromPreset(
  preset: GrokBuildProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const { baseUrl, model } = parseGrokPreset(preset);
  return {
    name: displayName,
    settingsConfig: {
      config: buildGrokConfig(
        displayName,
        baseUrl,
        model,
        fields.apiKey.trim(),
      ),
    },
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon ?? "grok",
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: {
      apiFormat: "openai_responses",
      ...buildMetaCustomEndpoints([
        baseUrl,
        ...(preset.endpointCandidates ?? []),
      ]),
    },
  };
}

function buildOpenCodeFromPreset(
  preset: OpenCodeProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<
    string,
    any
  >;
  settingsConfig.options = {
    ...(settingsConfig.options ?? {}),
    apiKey: fields.apiKey.trim(),
  };
  return {
    name: displayName,
    settingsConfig,
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: { apiFormat: openCodeApiFormat(preset) },
  };
}

function buildOpenClawFromPreset(
  preset: OpenClawProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
  providerId?: string,
): QuickStartProviderInput {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<
    string,
    any
  >;
  settingsConfig.apiKey = fields.apiKey.trim();
  const suggestedDefaults = preset.suggestedDefaults
    ? rebaseOpenClawSuggestedDefaults(
        preset.suggestedDefaults,
        providerId ?? "",
      )
    : undefined;
  return {
    name: displayName,
    settingsConfig,
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: { apiFormat: openClawApiFormat(preset) },
    ...(suggestedDefaults
      ? {
          openclawSuggestedDefaults: {
            model: suggestedDefaults.model,
            models: suggestedDefaults.modelCatalog,
          },
        }
      : {}),
  };
}

function buildHermesFromPreset(
  preset: HermesProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<
    string,
    any
  >;
  settingsConfig.api_key = fields.apiKey.trim();
  return {
    name: displayName,
    settingsConfig,
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: { apiFormat: hermesApiFormat(preset) },
  };
}

function buildClaudeFromPreset(
  preset: ProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<
    string,
    any
  >;
  const keyField =
    fields.advancedClaude?.apiKeyField ??
    preset.apiKeyField ??
    "ANTHROPIC_AUTH_TOKEN";
  const env = (settingsConfig.env ?? {}) as Record<string, string>;
  env[keyField] = fields.apiKey.trim();

  if (fields.advancedClaude) {
    const advanced = fields.advancedClaude;
    setOptionalEnv(env, "ANTHROPIC_MODEL", advanced.fallbackModel);
    setOptionalEnv(env, "ANTHROPIC_DEFAULT_HAIKU_MODEL", advanced.haikuModel);
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
      advanced.haikuModelName,
    );
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_SONNET_MODEL",
      withOneM(advanced.sonnetModel, advanced.sonnetSupports1m),
    );
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
      advanced.sonnetModelName,
    );
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_OPUS_MODEL",
      withOneM(advanced.opusModel, advanced.opusSupports1m),
    );
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
      advanced.opusModelName,
    );
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_FABLE_MODEL",
      withOneM(advanced.fableModel, advanced.fableSupports1m),
    );
    setOptionalEnv(
      env,
      "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME",
      advanced.fableModelName,
    );
    setOptionalEnv(
      env,
      "CLAUDE_CODE_SUBAGENT_MODEL",
      withOneM(advanced.subagentModel, advanced.subagentSupports1m),
    );
  }
  settingsConfig.env = env;

  const apiFormat =
    fields.advancedClaude?.apiFormat ?? preset.apiFormat ?? "anthropic";
  const baseUrl = env.ANTHROPIC_BASE_URL ?? "";
  const meta: ProviderMeta = {
    apiFormat,
    ...(keyField !== "ANTHROPIC_AUTH_TOKEN" ? { apiKeyField: keyField } : {}),
    ...buildMetaCustomEndpoints([
      baseUrl,
      ...(preset.endpointCandidates ?? []),
    ]),
  };

  return {
    name: displayName,
    settingsConfig,
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta,
  };
}

function buildDesktopFromPreset(
  preset: ClaudeDesktopProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const baseUrl = preset.baseUrl.trim().replace(/\/+$/, "");
  const keyField = preset.apiKeyField ?? "ANTHROPIC_AUTH_TOKEN";
  const advanced = fields.advancedDesktop;

  const routeMap: Record<string, ClaudeDesktopModelRoute> = {};
  if (preset.modelRoutes?.length) {
    for (const route of preset.modelRoutes) {
      routeMap[route.routeId] = {
        model:
          preset.mode === "direct"
            ? route.upstreamModel
            : route.routeId === CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus
              ? advanced?.opusModel || route.upstreamModel
              : route.routeId === CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku
                ? advanced?.haikuModel || route.upstreamModel
                : advanced?.sonnetModel || route.upstreamModel,
        labelOverride:
          route.routeId === CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus
            ? advanced?.opusLabel || route.labelOverride
            : route.routeId === CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku
              ? advanced?.haikuLabel || route.labelOverride
              : advanced?.sonnetLabel || route.labelOverride,
        supports1m:
          (route.routeId === CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus
            ? advanced?.opusSupports1m
            : route.routeId === CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku
              ? advanced?.haikuSupports1m
              : advanced?.sonnetSupports1m) || undefined,
      };
    }
    if (preset.mode === "proxy" && advanced) {
      routeMap[CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet] ??= {
        model: advanced.sonnetModel,
        labelOverride: advanced.sonnetLabel || undefined,
        supports1m: advanced.sonnetSupports1m || undefined,
      };
      routeMap[CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus] ??= {
        model: advanced.opusModel,
        labelOverride: advanced.opusLabel || undefined,
        supports1m: advanced.opusSupports1m || undefined,
      };
      routeMap[CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku] ??= {
        model: advanced.haikuModel,
        labelOverride: advanced.haikuLabel || undefined,
        supports1m: advanced.haikuSupports1m || undefined,
      };
    }
  } else {
    routeMap[CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet] = {
      model: advanced?.sonnetModel || "deepseek-v4-pro",
      labelOverride: advanced?.sonnetLabel || undefined,
      supports1m: advanced?.sonnetSupports1m || undefined,
    };
  }

  const settingsConfig = {
    env: {
      ANTHROPIC_BASE_URL: baseUrl,
      [keyField]: fields.apiKey.trim(),
    },
  };

  const apiFormat =
    fields.advancedDesktop?.apiFormat ?? preset.apiFormat ?? "anthropic";

  return {
    name: displayName,
    settingsConfig,
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: {
      claudeDesktopMode: preset.mode,
      claudeDesktopModelRoutes: routeMap,
      apiFormat: preset.mode === "proxy" ? apiFormat : "anthropic",
      ...buildMetaCustomEndpoints([
        baseUrl,
        ...(preset.endpointCandidates ?? []),
      ]),
    },
  };
}

function buildCodexFromPreset(
  preset: CodexProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const auth = generateThirdPartyAuth(fields.apiKey.trim());
  let config = preset.config;
  const defaultModel =
    fields.advancedCodex?.defaultModel ??
    preset.modelCatalog?.[0]?.model ??
    "gpt-5.6-sol";

  if (!preset.isOfficial && config) {
    config = generateThirdPartyConfig(
      preset.name.toLowerCase().replace(/\s+/g, "_"),
      getCodexBaseUrlFromPreset(preset),
      defaultModel,
    );
  }

  const settingsConfig: Record<string, unknown> = {
    auth,
    config,
  };

  if (preset.modelCatalog?.length) {
    settingsConfig.modelCatalog = { models: preset.modelCatalog };
  }

  const apiFormat =
    fields.advancedCodex?.apiFormat ?? preset.apiFormat ?? "openai_chat";

  return {
    name: displayName,
    settingsConfig: settingsConfig as Provider["settingsConfig"],
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: {
      apiFormat,
      codexChatReasoning: preset.codexChatReasoning,
      ...buildMetaCustomEndpoints(preset.endpointCandidates ?? []),
    },
  };
}

function getCodexBaseUrlFromPreset(preset: CodexProviderPreset): string {
  const fromUtil = getCodexBaseUrl({
    settingsConfig: { auth: preset.auth, config: preset.config },
  });
  if (fromUtil) return fromUtil;
  return preset.endpointCandidates?.[0] ?? "https://api.openai.com/v1";
}

function buildGeminiFromPreset(
  preset: GeminiProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<
    string,
    any
  >;
  const env = (settingsConfig.env ?? {}) as Record<string, string>;
  env.GEMINI_API_KEY = fields.apiKey.trim();
  if (fields.advancedGemini?.baseUrl) {
    env.GOOGLE_GEMINI_BASE_URL = fields.advancedGemini.baseUrl;
  }
  if (fields.advancedGemini?.model) {
    env.GEMINI_MODEL = fields.advancedGemini.model;
  }
  settingsConfig.env = env;

  const baseUrl =
    fields.advancedGemini?.baseUrl ??
    env.GOOGLE_GEMINI_BASE_URL ??
    preset.baseURL ??
    "";

  return {
    name: displayName,
    settingsConfig,
    websiteUrl: preset.websiteUrl,
    category: (preset.category ?? "third_party") as ProviderCategory,
    createdAt: Date.now(),
    icon: preset.icon,
    iconColor: preset.iconColor,
    isPartner: preset.isPartner,
    meta: {
      apiFormat:
        fields.advancedGemini?.apiFormat ??
        (preset.apiFormat === "gemini_native"
          ? preset.apiFormat
          : "gemini_native"),
      ...buildMetaCustomEndpoints([
        baseUrl,
        ...(preset.endpointCandidates ?? []),
      ]),
    },
  };
}

function buildCustomProvider(
  appId: QuickStartAppId,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const name = fields.customName.trim() || displayName;
  const baseUrl = fields.customBaseUrl.trim().replace(/\/+$/, "");
  const model = fields.customModel.trim();

  switch (appId) {
    case "claude": {
      const advanced = fields.advancedClaude;
      const keyField = advanced?.apiKeyField ?? "ANTHROPIC_AUTH_TOKEN";
      const selectedModel = advanced?.sonnetModel.trim() || model;
      const env: Record<string, string> = {
        ANTHROPIC_BASE_URL: baseUrl,
        [keyField]: fields.apiKey.trim(),
      };
      setOptionalEnv(
        env,
        "ANTHROPIC_MODEL",
        advanced?.fallbackModel || selectedModel,
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        advanced?.haikuModel || selectedModel,
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
        advanced?.haikuModelName ||
          stripOneM(advanced?.haikuModel || selectedModel),
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        withOneM(
          advanced?.sonnetModel || selectedModel,
          advanced?.sonnetSupports1m ?? false,
        ),
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
        advanced?.sonnetModelName || stripOneM(selectedModel),
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        withOneM(
          advanced?.opusModel || selectedModel,
          advanced?.opusSupports1m ?? false,
        ),
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
        advanced?.opusModelName ||
          stripOneM(advanced?.opusModel || selectedModel),
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_FABLE_MODEL",
        withOneM(
          advanced?.fableModel || advanced?.opusModel || selectedModel,
          advanced?.fableSupports1m ?? false,
        ),
      );
      setOptionalEnv(
        env,
        "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME",
        advanced?.fableModelName ||
          stripOneM(
            advanced?.fableModel || advanced?.opusModel || selectedModel,
          ),
      );
      setOptionalEnv(
        env,
        "CLAUDE_CODE_SUBAGENT_MODEL",
        withOneM(
          advanced?.subagentModel || "",
          advanced?.subagentSupports1m ?? false,
        ),
      );
      return {
        name,
        settingsConfig: {
          env,
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: advanced?.apiFormat ?? "anthropic",
          ...(keyField !== "ANTHROPIC_AUTH_TOKEN"
            ? { apiKeyField: keyField }
            : {}),
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    }
    case "claude-desktop": {
      const routeId = CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet;
      const advanced = fields.advancedDesktop;
      const sonnetModel = advanced?.sonnetModel.trim() || model;
      const opusModel = advanced?.opusModel.trim() || sonnetModel;
      const haikuModel = advanced?.haikuModel.trim() || sonnetModel;
      return {
        name,
        settingsConfig: {
          env: {
            ANTHROPIC_BASE_URL: baseUrl,
            ANTHROPIC_AUTH_TOKEN: fields.apiKey.trim(),
          },
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          claudeDesktopMode: "proxy",
          apiFormat: advanced?.apiFormat ?? "anthropic",
          claudeDesktopModelRoutes: {
            [routeId]: {
              model: sonnetModel,
              labelOverride: advanced?.sonnetLabel || undefined,
              supports1m: advanced?.sonnetSupports1m || undefined,
            },
            [CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus]: {
              model: opusModel,
              labelOverride: advanced?.opusLabel || undefined,
              supports1m: advanced?.opusSupports1m || undefined,
            },
            [CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku]: {
              model: haikuModel,
              labelOverride: advanced?.haikuLabel || undefined,
              supports1m: advanced?.haikuSupports1m || undefined,
            },
          },
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    }
    case "codex": {
      const advanced = fields.advancedCodex;
      const defaultModel = advanced?.defaultModel.trim() || model;
      const auth = generateThirdPartyAuth(fields.apiKey.trim());
      const config = generateThirdPartyConfig("custom", baseUrl, defaultModel);
      return {
        name,
        settingsConfig: { auth, config },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: advanced?.apiFormat ?? "openai_chat",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    }
    case "gemini": {
      const advanced = fields.advancedGemini;
      const resolvedBaseUrl = advanced?.baseUrl.trim() || baseUrl;
      const resolvedModel = advanced?.model.trim() || model;
      return {
        name,
        settingsConfig: {
          env: {
            GOOGLE_GEMINI_BASE_URL: resolvedBaseUrl,
            GEMINI_API_KEY: fields.apiKey.trim(),
            GEMINI_MODEL: resolvedModel,
          },
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: advanced?.apiFormat ?? "gemini_native",
          ...buildMetaCustomEndpoints([resolvedBaseUrl]),
        },
      };
    }
    case "opencode":
      return {
        name,
        settingsConfig: {
          npm: "@ai-sdk/openai-compatible",
          name,
          options: { baseURL: baseUrl, apiKey: fields.apiKey.trim() },
          models: model ? { [model]: { name: model } } : {},
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: "openai_chat",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    case "openclaw":
      return {
        name,
        settingsConfig: {
          baseUrl,
          apiKey: fields.apiKey.trim(),
          api: "openai-completions",
          models: model ? [{ id: model, name: model }] : [],
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: "openai_chat",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    case "hermes":
      return {
        name,
        settingsConfig: {
          name,
          base_url: baseUrl,
          api_key: fields.apiKey.trim(),
          api_mode: "chat_completions",
          models: model ? [{ id: model, name: model }] : [],
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: "openai_chat",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    case "grokbuild":
      return {
        name,
        settingsConfig: {
          config: buildGrokConfig(
            name,
            baseUrl,
            model || GROK_BUILD_DEFAULT_MODEL,
            fields.apiKey.trim(),
          ),
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: "openai_responses",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    default:
      throw new Error(`Unsupported custom app: ${appId}`);
  }
}

/** 从预设初始化高级选项默认值 */
export function defaultAdvancedFields(
  appId: QuickStartAppId,
  selection: QuickStartSelection,
): Partial<QuickStartFormFields> {
  if (selection.mode === "custom") {
    const customModel =
      appId === "codex"
        ? "gpt-5.6-sol"
        : appId === "gemini"
          ? "gemini-3.6-flash"
          : appId === "grokbuild"
            ? GROK_BUILD_DEFAULT_MODEL
            : "deepseek-v4-pro";
    const common = {
      customName: "",
      customBaseUrl: "",
      customModel,
    };
    switch (appId) {
      case "claude":
        return {
          ...common,
          advancedClaude: {
            apiFormat: "anthropic",
            apiKeyField: "ANTHROPIC_AUTH_TOKEN",
            haikuModel: customModel,
            haikuModelName: customModel,
            sonnetModel: customModel,
            sonnetModelName: customModel,
            sonnetSupports1m: false,
            opusModel: customModel,
            opusModelName: customModel,
            opusSupports1m: false,
            fableModel: customModel,
            fableModelName: customModel,
            fableSupports1m: false,
            subagentModel: "",
            subagentSupports1m: false,
            fallbackModel: customModel,
          },
        };
      case "claude-desktop":
        return {
          ...common,
          advancedDesktop: {
            apiFormat: "anthropic",
            sonnetModel: customModel,
            sonnetLabel: customModel,
            sonnetSupports1m: false,
            opusModel: customModel,
            opusLabel: customModel,
            opusSupports1m: false,
            haikuModel: customModel,
            haikuLabel: customModel,
            haikuSupports1m: false,
          },
        };
      case "codex":
        return {
          ...common,
          advancedCodex: {
            apiFormat: "openai_chat",
            defaultModel: customModel,
          },
        };
      case "gemini":
        return {
          ...common,
          advancedGemini: {
            apiFormat: "gemini_native",
            baseUrl: "",
            model: customModel,
          },
        };
      case "grokbuild":
        return common;
      case "opencode":
      case "openclaw":
      case "hermes":
        return common;
    }
  }
  if (selection.mode !== "preset") return {};

  const preset = resolvePresetByName(appId, selection.presetName);
  if (!preset) return {};

  switch (appId) {
    case "claude": {
      const raw = preset.raw as ProviderPreset;
      const settingsConfig = raw.settingsConfig as
        | { env?: Record<string, string> }
        | undefined;
      const env = settingsConfig?.env;
      const fallbackModel = env?.ANTHROPIC_MODEL ?? "";
      const haikuModel = env?.ANTHROPIC_DEFAULT_HAIKU_MODEL ?? fallbackModel;
      const sonnetModel = env?.ANTHROPIC_DEFAULT_SONNET_MODEL ?? fallbackModel;
      const opusModel = env?.ANTHROPIC_DEFAULT_OPUS_MODEL ?? fallbackModel;
      const fableModel = env?.ANTHROPIC_DEFAULT_FABLE_MODEL ?? opusModel;
      return {
        advancedClaude: {
          apiFormat: raw.apiFormat ?? "anthropic",
          apiKeyField: raw.apiKeyField ?? "ANTHROPIC_AUTH_TOKEN",
          haikuModel: stripOneM(haikuModel),
          haikuModelName:
            env?.ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME ?? stripOneM(haikuModel),
          sonnetModel: stripOneM(sonnetModel),
          sonnetModelName:
            env?.ANTHROPIC_DEFAULT_SONNET_MODEL_NAME ?? stripOneM(sonnetModel),
          sonnetSupports1m: /\[1m\]$/i.test(sonnetModel.trim()),
          opusModel: stripOneM(opusModel),
          opusModelName:
            env?.ANTHROPIC_DEFAULT_OPUS_MODEL_NAME ?? stripOneM(opusModel),
          opusSupports1m: /\[1m\]$/i.test(opusModel.trim()),
          fableModel: stripOneM(fableModel),
          fableModelName:
            env?.ANTHROPIC_DEFAULT_FABLE_MODEL_NAME ?? stripOneM(fableModel),
          fableSupports1m: /\[1m\]$/i.test(fableModel.trim()),
          subagentModel: stripOneM(env?.CLAUDE_CODE_SUBAGENT_MODEL ?? ""),
          subagentSupports1m: /\[1m\]$/i.test(
            (env?.CLAUDE_CODE_SUBAGENT_MODEL ?? "").trim(),
          ),
          fallbackModel: stripOneM(fallbackModel),
        },
      };
    }
    case "claude-desktop": {
      const raw = preset.raw as ClaudeDesktopProviderPreset;
      const byRouteId = new Map(
        (raw.modelRoutes ?? []).map((route) => [route.routeId, route]),
      );
      const fallbackRoute = raw.modelRoutes?.[0];
      const sonnet =
        byRouteId.get(CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet) ?? fallbackRoute;
      const opus =
        byRouteId.get(CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus) ?? fallbackRoute;
      const haiku =
        byRouteId.get(CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku) ?? fallbackRoute;
      return {
        advancedDesktop: {
          apiFormat: raw.apiFormat ?? "anthropic",
          sonnetModel: sonnet?.upstreamModel ?? "",
          sonnetLabel: sonnet?.labelOverride ?? sonnet?.upstreamModel ?? "",
          sonnetSupports1m: sonnet?.supports1m ?? false,
          opusModel: opus?.upstreamModel ?? "",
          opusLabel: opus?.labelOverride ?? opus?.upstreamModel ?? "",
          opusSupports1m: opus?.supports1m ?? false,
          haikuModel: haiku?.upstreamModel ?? "",
          haikuLabel: haiku?.labelOverride ?? haiku?.upstreamModel ?? "",
          haikuSupports1m: haiku?.supports1m ?? false,
        },
      };
    }
    case "codex": {
      const raw = preset.raw as CodexProviderPreset;
      return {
        advancedCodex: {
          apiFormat: raw.apiFormat ?? "openai_chat",
          defaultModel: raw.modelCatalog?.[0]?.model ?? "gpt-5.6-sol",
        },
      };
    }
    case "gemini": {
      const raw = preset.raw as GeminiProviderPreset;
      const settingsConfig = raw.settingsConfig as
        | { env?: Record<string, string> }
        | undefined;
      const env = settingsConfig?.env;
      return {
        advancedGemini: {
          apiFormat: "gemini_native",
          baseUrl: env?.GOOGLE_GEMINI_BASE_URL ?? raw.baseURL ?? "",
          model: env?.GEMINI_MODEL ?? raw.model ?? "",
        },
      };
    }
    case "grokbuild": {
      const raw = preset.raw as GrokBuildProviderPreset;
      const parsed = parseGrokPreset(raw);
      return {
        customBaseUrl: parsed.baseUrl,
        customModel: parsed.model,
      };
    }
    case "opencode":
    case "openclaw":
    case "hermes":
      return {};
    default:
      return {};
  }
}
