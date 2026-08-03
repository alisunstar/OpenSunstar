import type { AppId } from "@/lib/api";

/** QuickStart 支持的本地 AI 编程客户端。 */
export type QuickStartAppId = Extract<
  AppId,
  | "claude"
  | "claude-desktop"
  | "codex"
  | "gemini"
  | "grokbuild"
  | "opencode"
  | "openclaw"
  | "hermes"
>;

export const QUICKSTART_APP_IDS: QuickStartAppId[] = [
  "claude",
  "claude-desktop",
  "codex",
  "gemini",
  "grokbuild",
  "opencode",
  "openclaw",
  "hermes",
];

export const QUICKSTART_CUSTOM_PRESET_ID = "__quickstart_custom__";

/**
 * `official` 仅供页面顶部的官方账号入口使用；新增供应商弹窗按其余四组展示。
 * 分类 ID 保持稳定，显示名称由 QUICKSTART_CATEGORY_LABEL_KEYS 提供。
 */
export const QUICKSTART_CATEGORY_ORDER = [
  "official",
  "ai_global",
  "ai_china",
  "relay",
  "custom",
] as const;

export type QuickStartCategoryId = (typeof QUICKSTART_CATEGORY_ORDER)[number];

export interface QuickStartPresetRef {
  /** 对应应用 preset 库中的内部名称。 */
  presetName?: string;
  /** 快速接入列表使用的产品显示名。 */
  displayName: string;
  /** 当前客户端协议尚未适配时显示为禁用卡片。 */
  unavailable?: boolean;
  /** 面向用户的具体协议约束说明。 */
  unavailableReason?: string;
  /** 协议约束说明的 i18n key。 */
  unavailableReasonKey?: string;
}

export interface QuickStartCategorySpec {
  category: QuickStartCategoryId;
  /** 顺序即展示顺序。 */
  presets: QuickStartPresetRef[];
}

const ref = (
  presetName: string,
  displayName = presetName,
): QuickStartPresetRef => ({
  presetName,
  displayName,
});

const unavailable = (
  displayName: string,
  reason: { message: string; key: string },
): QuickStartPresetRef => ({
  displayName,
  unavailable: true,
  unavailableReason: reason.message,
  unavailableReasonKey: reason.key,
});

const CODEX_OPENAI_ONLY = {
  message: "Codex 快速接入当前使用 OpenAI Responses / Chat 上游协议",
  key: "quickStart.unavailable.codexOpenAiOnly",
};
const CLAUDE_OAUTH_ONLY = {
  message: "此 OAuth 预设面向 Claude 客户端",
  key: "quickStart.unavailable.claudeOauthOnly",
};
const GEMINI_UPSTREAM_ONLY = {
  message: "Gemini CLI 需要 Gemini Native 或 Gemini 兼容网关",
  key: "quickStart.unavailable.geminiNativeOnly",
};
const GEMINI_OPENROUTER_INCOMPATIBLE = {
  message:
    "OpenRouter 提供 OpenAI Chat 上游，Gemini CLI 当前透传 Gemini Native 协议",
  key: "quickStart.unavailable.geminiOpenRouter",
};

const internationalForClaude = (): QuickStartPresetRef[] => [
  ref("OpenAI API", "OpenAI"),
  ref("Anthropic API", "Anthropic"),
  ref("Gemini Native", "Gemini API"),
  ref("OpenCode Go"),
  ref("xAI (Grok)", "xAI (Grok) API"),
  ref("GitHub Copilot"),
  ref("MiniMax en", "MiniMax(国际)"),
  ref("Zhipu GLM en", "Z.ai"),
  ref("SiliconFlow en", "SiliconFlow"),
  ref("DMXAPI"),
  ref("Agnes AI"),
];

const internationalForCodex = (): QuickStartPresetRef[] => [
  ref("OpenAI API", "OpenAI"),
  unavailable("Anthropic", CODEX_OPENAI_ONLY),
  ref("Gemini API"),
  ref("OpenCode Go"),
  ref("xAI (Grok)", "xAI (Grok) API"),
  unavailable("GitHub Copilot", CLAUDE_OAUTH_ONLY),
  ref("MiniMax en", "MiniMax(国际)"),
  ref("Zhipu GLM en", "Z.ai"),
  ref("SiliconFlow en", "SiliconFlow"),
  ref("DMXAPI"),
  ref("Agnes AI"),
];

const internationalForGemini = (): QuickStartPresetRef[] => [
  unavailable("OpenAI", GEMINI_UPSTREAM_ONLY),
  unavailable("Anthropic", GEMINI_UPSTREAM_ONLY),
  ref("Gemini API"),
  unavailable("OpenCode Go", GEMINI_UPSTREAM_ONLY),
  unavailable("xAI (Grok) API", GEMINI_UPSTREAM_ONLY),
  unavailable("GitHub Copilot", GEMINI_UPSTREAM_ONLY),
  unavailable("MiniMax(国际)", GEMINI_UPSTREAM_ONLY),
  unavailable("Z.ai", GEMINI_UPSTREAM_ONLY),
  unavailable("SiliconFlow", GEMINI_UPSTREAM_ONLY),
  unavailable("DMXAPI", GEMINI_UPSTREAM_ONLY),
  unavailable("Agnes AI", GEMINI_UPSTREAM_ONLY),
];

const chinaPresets = (): QuickStartPresetRef[] => [
  ref("DeepSeek"),
  ref("Kimi"),
  ref("Zhipu GLM"),
  ref("MiniMax"),
  ref("Xiaomi MiMo"),
  ref("StepFun"),
  ref("Longcat"),
  ref("BaiLing"),
];

const unavailableChinaPresets = (): QuickStartPresetRef[] =>
  chinaPresets().map(({ displayName }) =>
    unavailable(displayName, GEMINI_UPSTREAM_ONLY),
  );

const relayPresets = (): QuickStartPresetRef[] => [
  ref("SiliconFlow", "硅基流动中国"),
  ref("OpenRouter"),
];

const agentInternationalPresets = (): QuickStartPresetRef[] => [
  ref("Shengsuanyun"),
  ref("火山Agentplan"),
  ref("BytePlus"),
  ref("DouBaoSeed"),
  ref("CCSub"),
  ref("Unity2.ai"),
];

const agentChinaPresets = (): QuickStartPresetRef[] => [
  ref("DeepSeek"),
  ref("Zhipu GLM"),
  ref("MiniMax"),
  ref("Xiaomi MiMo"),
  ref("StepFun"),
  ref("Longcat"),
  ref("BaiLing"),
];

const agentRelayPresets = (): QuickStartPresetRef[] => [ref("OpenRouter")];

const agentApiKeyGroups = (): QuickStartCategorySpec[] => [
  { category: "ai_global", presets: agentInternationalPresets() },
  { category: "ai_china", presets: agentChinaPresets() },
  { category: "relay", presets: agentRelayPresets() },
  { category: "custom", presets: [] },
];

export const QUICKSTART_CURATED: Record<
  QuickStartAppId,
  QuickStartCategorySpec[]
> = {
  claude: [
    { category: "official", presets: [ref("Claude Official")] },
    { category: "ai_global", presets: internationalForClaude() },
    { category: "ai_china", presets: chinaPresets() },
    { category: "relay", presets: relayPresets() },
    { category: "custom", presets: [] },
  ],
  "claude-desktop": [
    { category: "official", presets: [ref("Claude Desktop Official")] },
    { category: "ai_global", presets: internationalForClaude() },
    { category: "ai_china", presets: chinaPresets() },
    { category: "relay", presets: relayPresets() },
    { category: "custom", presets: [] },
  ],
  codex: [
    { category: "official", presets: [ref("OpenAI Official")] },
    { category: "ai_global", presets: internationalForCodex() },
    { category: "ai_china", presets: chinaPresets() },
    { category: "relay", presets: relayPresets() },
    { category: "custom", presets: [] },
  ],
  gemini: [
    { category: "official", presets: [ref("Google Official")] },
    { category: "ai_global", presets: internationalForGemini() },
    { category: "ai_china", presets: unavailableChinaPresets() },
    {
      category: "relay",
      presets: [
        unavailable("硅基流动中国", GEMINI_UPSTREAM_ONLY),
        unavailable("OpenRouter", GEMINI_OPENROUTER_INCOMPATIBLE),
      ],
    },
    { category: "custom", presets: [] },
  ],
  grokbuild: [
    { category: "official", presets: [ref("Grok Official")] },
    {
      category: "ai_global",
      presets: [
        ref("xAI (Grok)"),
        ref("OpenRouter"),
        ref("AiHubMix"),
        ref("Amux"),
        ref("Shengsuanyun"),
      ],
    },
    {
      category: "ai_china",
      presets: [
        ref("PackyCode"),
        ref("ZetaAPI"),
        ref("APINebula"),
        ref("PatewayAI"),
        ref("CCSub"),
      ],
    },
    {
      category: "relay",
      presets: [ref("SubRouter"), ref("TheRouter"), ref("AICodeMirror")],
    },
    { category: "custom", presets: [] },
  ],
  opencode: agentApiKeyGroups(),
  openclaw: agentApiKeyGroups(),
  hermes: agentApiKeyGroups(),
};

export const QUICKSTART_CATEGORY_LABEL_KEYS: Record<
  QuickStartCategoryId,
  string
> = {
  official: "providerPreset.category.official",
  ai_global: "quickStart.category.aiGlobal",
  ai_china: "quickStart.category.aiChina",
  relay: "quickStart.category.relay",
  custom: "quickStart.category.customModel",
};
