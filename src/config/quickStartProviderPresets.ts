import type { ProviderPreset } from "./claudeProviderPresets";
import {
  CLAUDE_DESKTOP_ROLE_ROUTE_IDS,
  type ClaudeDesktopProviderPreset,
} from "./claudeDesktopProviderPresets";
import {
  generateThirdPartyAuth,
  generateThirdPartyConfig,
  type CodexProviderPreset,
} from "./codexProviderPresets";
import type { GeminiProviderPreset } from "./geminiProviderPresets";

const claudeModels = (
  baseUrl: string,
  model: string,
  apiFormat: NonNullable<ProviderPreset["apiFormat"]>,
  apiKeyField: NonNullable<
    ProviderPreset["apiKeyField"]
  > = "ANTHROPIC_AUTH_TOKEN",
  roles?: {
    sonnet: string;
    opus: string;
    haiku: string;
    fable?: string;
    names?: Partial<Record<"sonnet" | "opus" | "haiku" | "fable", string>>;
    oneM?: Array<"sonnet" | "opus" | "fable">;
  },
): Pick<
  ProviderPreset,
  "settingsConfig" | "apiFormat" | "apiKeyField" | "endpointCandidates"
> => {
  const haiku = roles?.haiku ?? model;
  const sonnet = roles?.sonnet ?? model;
  const opus = roles?.opus ?? model;
  const fable = roles?.fable ?? opus;
  const oneM = new Set(roles?.oneM ?? []);
  const marked = (role: "sonnet" | "opus" | "fable", value: string) =>
    oneM.has(role) ? `${value}[1M]` : value;

  return {
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: baseUrl,
        [apiKeyField]: "",
        ANTHROPIC_MODEL: model,
        ANTHROPIC_DEFAULT_HAIKU_MODEL: haiku,
        ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME: roles?.names?.haiku ?? haiku,
        ANTHROPIC_DEFAULT_SONNET_MODEL: marked("sonnet", sonnet),
        ANTHROPIC_DEFAULT_SONNET_MODEL_NAME: roles?.names?.sonnet ?? sonnet,
        ANTHROPIC_DEFAULT_OPUS_MODEL: marked("opus", opus),
        ANTHROPIC_DEFAULT_OPUS_MODEL_NAME: roles?.names?.opus ?? opus,
        ANTHROPIC_DEFAULT_FABLE_MODEL: marked("fable", fable),
        ANTHROPIC_DEFAULT_FABLE_MODEL_NAME: roles?.names?.fable ?? fable,
      },
    },
    apiFormat,
    apiKeyField,
    endpointCandidates: [baseUrl],
  };
};

export const quickStartClaudePresets: ProviderPreset[] = [
  {
    name: "OpenAI API",
    websiteUrl: "https://platform.openai.com",
    apiKeyUrl: "https://platform.openai.com/api-keys",
    category: "third_party",
    icon: "openai",
    iconColor: "#000000",
    ...claudeModels(
      "https://api.openai.com/v1",
      "gpt-5.6-sol",
      "openai_responses",
      "ANTHROPIC_API_KEY",
      {
        sonnet: "gpt-5.6-terra",
        opus: "gpt-5.6-sol",
        haiku: "gpt-5.6-luna",
        fable: "gpt-5.6-sol",
      },
    ),
    modelsUrl: "https://api.openai.com/v1/models",
  },
  {
    name: "Anthropic API",
    websiteUrl: "https://console.anthropic.com",
    apiKeyUrl: "https://console.anthropic.com/settings/keys",
    category: "third_party",
    icon: "anthropic",
    iconColor: "#D4915D",
    ...claudeModels(
      "https://api.anthropic.com",
      "claude-sonnet-5",
      "anthropic",
      "ANTHROPIC_API_KEY",
      {
        sonnet: "claude-sonnet-5",
        opus: "claude-opus-5",
        haiku: "claude-haiku-4-5",
        fable: "claude-fable-5",
        names: {
          sonnet: "Claude Sonnet 5",
          opus: "Claude Opus 5",
          haiku: "Claude Haiku 4.5",
          fable: "Claude Fable 5",
        },
        oneM: ["sonnet", "opus", "fable"],
      },
    ),
  },
  {
    name: "xAI (Grok)",
    websiteUrl: "https://x.ai/api",
    apiKeyUrl: "https://console.x.ai",
    category: "third_party",
    icon: "xai",
    iconColor: "#000000",
    ...claudeModels("https://api.x.ai/v1", "grok-4.5", "openai_responses"),
  },
  {
    name: "Agnes AI",
    websiteUrl: "https://www.agnes-ai.com",
    apiKeyUrl: "https://platform.agnes-ai.com",
    category: "third_party",
    icon: "generic",
    iconColor: "#6D5EF7",
    ...claudeModels(
      "https://apihub.agnes-ai.com/v1",
      "agnes-2.0-flash",
      "openai_chat",
    ),
  },
];

const desktopPreset = (
  name: string,
  websiteUrl: string,
  apiKeyUrl: string,
  baseUrl: string,
  models: string | { sonnet: string; opus: string; haiku: string },
  apiFormat: NonNullable<ClaudeDesktopProviderPreset["apiFormat"]>,
  icon: string,
  apiKeyField: ClaudeDesktopProviderPreset["apiKeyField"] = "ANTHROPIC_AUTH_TOKEN",
  oneM: Partial<Record<"sonnet" | "opus" | "haiku", boolean>> = {},
): ClaudeDesktopProviderPreset => ({
  name,
  websiteUrl,
  apiKeyUrl,
  category: "third_party",
  baseUrl,
  mode: "proxy",
  apiFormat,
  apiKeyField,
  modelRoutes: [
    {
      routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
      upstreamModel: typeof models === "string" ? models : models.sonnet,
      supports1m: oneM.sonnet ?? false,
    },
    {
      routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
      upstreamModel: typeof models === "string" ? models : models.opus,
      supports1m: oneM.opus ?? false,
    },
    {
      routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
      upstreamModel: typeof models === "string" ? models : models.haiku,
      supports1m: oneM.haiku ?? false,
    },
  ],
  endpointCandidates: [baseUrl],
  icon,
});

export const quickStartDesktopPresets: ClaudeDesktopProviderPreset[] = [
  desktopPreset(
    "OpenAI API",
    "https://platform.openai.com",
    "https://platform.openai.com/api-keys",
    "https://api.openai.com/v1",
    {
      sonnet: "gpt-5.6-terra",
      opus: "gpt-5.6-sol",
      haiku: "gpt-5.6-luna",
    },
    "openai_responses",
    "openai",
    "ANTHROPIC_API_KEY",
  ),
  desktopPreset(
    "Anthropic API",
    "https://console.anthropic.com",
    "https://console.anthropic.com/settings/keys",
    "https://api.anthropic.com",
    {
      sonnet: "claude-sonnet-5",
      opus: "claude-opus-5",
      haiku: "claude-haiku-4-5",
    },
    "anthropic",
    "anthropic",
    "ANTHROPIC_API_KEY",
    { sonnet: true, opus: true },
  ),
  desktopPreset(
    "xAI (Grok)",
    "https://x.ai/api",
    "https://console.x.ai",
    "https://api.x.ai/v1",
    "grok-4.5",
    "openai_responses",
    "xai",
  ),
  desktopPreset(
    "Agnes AI",
    "https://www.agnes-ai.com",
    "https://platform.agnes-ai.com",
    "https://apihub.agnes-ai.com/v1",
    "agnes-2.0-flash",
    "openai_chat",
    "generic",
  ),
];

const codexPreset = (
  name: string,
  websiteUrl: string,
  apiKeyUrl: string,
  baseUrl: string,
  model: string,
  apiFormat: NonNullable<CodexProviderPreset["apiFormat"]>,
  icon: string,
): CodexProviderPreset => ({
  name,
  websiteUrl,
  apiKeyUrl,
  auth: generateThirdPartyAuth(""),
  config: generateThirdPartyConfig(
    name.toLowerCase().replace(/[^a-z0-9]+/g, "_"),
    baseUrl,
    model,
  ),
  category: "third_party",
  endpointCandidates: [baseUrl],
  apiFormat,
  modelCatalog: [{ model }],
  icon,
});

export const quickStartCodexPresets: CodexProviderPreset[] = [
  codexPreset(
    "OpenAI API",
    "https://platform.openai.com",
    "https://platform.openai.com/api-keys",
    "https://api.openai.com/v1",
    "gpt-5.6-sol",
    "openai_responses",
    "openai",
  ),
  codexPreset(
    "Gemini API",
    "https://ai.google.dev/gemini-api",
    "https://aistudio.google.com/apikey",
    "https://generativelanguage.googleapis.com/v1beta/openai",
    "gemini-3.6-flash",
    "openai_chat",
    "gemini",
  ),
  codexPreset(
    "OpenCode Go",
    "https://opencode.ai/go",
    "https://opencode.ai/go",
    "https://opencode.ai/zen/go/v1",
    "glm-5.2",
    "openai_chat",
    "opencode",
  ),
  codexPreset(
    "xAI (Grok)",
    "https://x.ai/api",
    "https://console.x.ai",
    "https://api.x.ai/v1",
    "grok-4.5",
    "openai_responses",
    "xai",
  ),
  codexPreset(
    "Agnes AI",
    "https://www.agnes-ai.com",
    "https://platform.agnes-ai.com",
    "https://apihub.agnes-ai.com/v1",
    "agnes-2.0-flash",
    "openai_chat",
    "generic",
  ),
];

export const quickStartGeminiPresets: GeminiProviderPreset[] = [
  {
    name: "Gemini API",
    websiteUrl: "https://ai.google.dev/gemini-api",
    apiKeyUrl: "https://aistudio.google.com/apikey",
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://generativelanguage.googleapis.com",
        GEMINI_API_KEY: "",
        GEMINI_MODEL: "gemini-3.6-flash",
      },
    },
    baseURL: "https://generativelanguage.googleapis.com",
    model: "gemini-3.6-flash",
    apiFormat: "gemini_native",
    category: "third_party",
    icon: "gemini",
    iconColor: "#4285F4",
  },
];
