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
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type {
  ClaudeDesktopModelRoute,
  Provider,
  ProviderCategory,
  ProviderMeta,
} from "@/types";
import type { QuickStartFormFields, QuickStartSelection } from "./types";
import { resolvePresetByName } from "./resolvePresets";

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
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

export function buildQuickStartProviderInput(
  appId: QuickStartAppId,
  selection: QuickStartSelection,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> & {
  ensureClaudeDesktopOfficialSeed?: boolean;
} {
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
    default:
      throw new Error(`Unsupported app: ${appId}`);
  }
}

function buildClaudeFromPreset(
  preset: ProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<string, any>;
  const keyField =
    fields.advancedClaude?.apiKeyField ??
    preset.apiKeyField ??
    "ANTHROPIC_AUTH_TOKEN";
  const env = (settingsConfig.env ?? {}) as Record<string, string>;
  env[keyField] = fields.apiKey.trim();

  if (fields.advancedClaude) {
    env.ANTHROPIC_DEFAULT_HAIKU_MODEL = fields.advancedClaude.haikuModel;
    env.ANTHROPIC_DEFAULT_SONNET_MODEL = fields.advancedClaude.sonnetModel;
    env.ANTHROPIC_DEFAULT_OPUS_MODEL = fields.advancedClaude.opusModel;
    env.ANTHROPIC_MODEL = fields.advancedClaude.sonnetModel;
  }
  settingsConfig.env = env;

  const apiFormat = fields.advancedClaude?.apiFormat ?? preset.apiFormat ?? "anthropic";
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
  const upstream =
    fields.advancedDesktop?.upstreamModel ??
    preset.modelRoutes?.[0]?.upstreamModel ??
    "deepseek-v4-pro";

  const routeMap: Record<string, ClaudeDesktopModelRoute> = {};
  if (preset.modelRoutes?.length) {
    for (const route of preset.modelRoutes) {
      routeMap[route.routeId] = {
        model:
          preset.mode === "direct"
            ? route.upstreamModel
            : fields.advancedDesktop?.upstreamModel || route.upstreamModel,
        labelOverride: route.labelOverride,
        supports1m: route.supports1m || undefined,
      };
    }
  } else {
    routeMap[CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet] = {
      model: upstream,
    };
  }

  const settingsConfig = {
    env: {
      ANTHROPIC_BASE_URL: baseUrl,
      [keyField]: fields.apiKey.trim(),
    },
  };

  const apiFormat =
    fields.advancedDesktop?.apiFormat ??
    preset.apiFormat ??
    "anthropic";

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
    "gpt-5.5";

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
  const fromUtil = getCodexBaseUrl({ auth: preset.auth, config: preset.config });
  if (fromUtil) return fromUtil;
  return preset.endpointCandidates?.[0] ?? "https://api.openai.com/v1";
}

function buildGeminiFromPreset(
  preset: GeminiProviderPreset,
  fields: QuickStartFormFields,
  displayName: string,
): Omit<Provider, "id"> {
  const settingsConfig = cloneJson(preset.settingsConfig) as Record<string, any>;
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
    meta: buildMetaCustomEndpoints([
      baseUrl,
      ...(preset.endpointCandidates ?? []),
    ]),
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
    case "claude":
      return {
        name,
        settingsConfig: {
          env: {
            ANTHROPIC_BASE_URL: baseUrl,
            ANTHROPIC_AUTH_TOKEN: fields.apiKey.trim(),
            ANTHROPIC_MODEL: model,
            ANTHROPIC_DEFAULT_HAIKU_MODEL: model,
            ANTHROPIC_DEFAULT_SONNET_MODEL: model,
            ANTHROPIC_DEFAULT_OPUS_MODEL: model,
          },
        },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: "anthropic",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    case "claude-desktop": {
      const routeId = CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet;
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
          apiFormat: "anthropic",
          claudeDesktopModelRoutes: {
            [routeId]: { model },
          },
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    }
    case "codex": {
      const auth = generateThirdPartyAuth(fields.apiKey.trim());
      const config = generateThirdPartyConfig("custom", baseUrl, model);
      return {
        name,
        settingsConfig: { auth, config },
        category: "custom",
        createdAt: Date.now(),
        meta: {
          apiFormat: "openai_chat",
          ...buildMetaCustomEndpoints([baseUrl]),
        },
      };
    }
    case "gemini":
      return {
        name,
        settingsConfig: {
          env: {
            GOOGLE_GEMINI_BASE_URL: baseUrl,
            GEMINI_API_KEY: fields.apiKey.trim(),
            GEMINI_MODEL: model,
          },
        },
        category: "custom",
        createdAt: Date.now(),
        meta: buildMetaCustomEndpoints([baseUrl]),
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
    return {
      customName: "",
      customBaseUrl: "",
      customModel:
        appId === "codex"
          ? "gpt-5.5"
          : appId === "gemini"
            ? "gemini-3.5-flash"
            : "deepseek-v4-pro",
    };
  }
  if (selection.mode !== "preset") return {};

  const preset = resolvePresetByName(appId, selection.presetName);
  if (!preset) return {};

  switch (appId) {
    case "claude": {
      const raw = preset.raw as ProviderPreset;
      const env = raw.settingsConfig?.env as Record<string, string> | undefined;
      return {
        advancedClaude: {
          apiFormat: raw.apiFormat ?? "anthropic",
          apiKeyField: raw.apiKeyField ?? "ANTHROPIC_AUTH_TOKEN",
          haikuModel: env?.ANTHROPIC_DEFAULT_HAIKU_MODEL ?? "",
          sonnetModel: env?.ANTHROPIC_DEFAULT_SONNET_MODEL ?? "",
          opusModel: env?.ANTHROPIC_DEFAULT_OPUS_MODEL ?? "",
        },
      };
    }
    case "claude-desktop": {
      const raw = preset.raw as ClaudeDesktopProviderPreset;
      return {
        advancedDesktop: {
          apiFormat: raw.apiFormat ?? "anthropic",
          upstreamModel: raw.modelRoutes?.[0]?.upstreamModel ?? "",
        },
      };
    }
    case "codex": {
      const raw = preset.raw as CodexProviderPreset;
      return {
        advancedCodex: {
          apiFormat: raw.apiFormat ?? "openai_chat",
          defaultModel: raw.modelCatalog?.[0]?.model ?? "gpt-5.5",
        },
      };
    }
    case "gemini": {
      const raw = preset.raw as GeminiProviderPreset;
      const env = raw.settingsConfig?.env as Record<string, string> | undefined;
      return {
        advancedGemini: {
          baseUrl: env?.GOOGLE_GEMINI_BASE_URL ?? raw.baseURL ?? "",
          model: env?.GEMINI_MODEL ?? raw.model ?? "",
        },
      };
    }
    default:
      return {};
  }
}
