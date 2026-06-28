import type { AppId } from "@/lib/api";

/** QuickStart 支持的四个应用（与产品规格一致） */
export type QuickStartAppId = Extract<
  AppId,
  "claude" | "claude-desktop" | "codex" | "gemini"
>;

export const QUICKSTART_APP_IDS: QuickStartAppId[] = [
  "claude",
  "claude-desktop",
  "codex",
  "gemini",
];

export const QUICKSTART_CUSTOM_PRESET_ID = "__quickstart_custom__";

/** Step ① 分类展示顺序（精选向导，非全量 preset 库） */
export const QUICKSTART_CATEGORY_ORDER = [
  "official",
  "cn_official",
  "aggregator",
  "custom",
] as const;

export type QuickStartCategoryId = (typeof QUICKSTART_CATEGORY_ORDER)[number];

export interface QuickStartCategorySpec {
  category: QuickStartCategoryId;
  /** 对应各 app preset 数组中的 `name` 字段，顺序即展示顺序 */
  presetNames: string[];
  /** i18n key；无 preset 时仍展示分类 + 说明（Gemini cn 策略 B） */
  emptyHintKey?: string;
}

export const QUICKSTART_CURATED: Record<
  QuickStartAppId,
  QuickStartCategorySpec[]
> = {
  claude: [
    { category: "official", presetNames: ["Claude Official"] },
    {
      category: "cn_official",
      presetNames: [
        "DeepSeek",
        "Zhipu GLM",
        "Kimi",
        "MiniMax",
        "Xiaomi MiMo",
        "StepFun",
      ],
    },
    { category: "aggregator", presetNames: ["OpenRouter"] },
    { category: "custom", presetNames: [] },
  ],
  "claude-desktop": [
    { category: "official", presetNames: ["Claude Desktop Official"] },
    {
      category: "cn_official",
      presetNames: [
        "DeepSeek",
        "Zhipu GLM",
        "Kimi",
        "MiniMax",
        "Xiaomi MiMo",
        "StepFun",
      ],
    },
    { category: "aggregator", presetNames: ["OpenRouter"] },
    { category: "custom", presetNames: [] },
  ],
  codex: [
    { category: "official", presetNames: ["OpenAI Official"] },
    {
      category: "cn_official",
      presetNames: [
        "DeepSeek",
        "Zhipu GLM",
        "Kimi",
        "MiniMax",
        "Xiaomi MiMo",
        "StepFun",
      ],
    },
    { category: "aggregator", presetNames: ["OpenRouter"] },
    { category: "custom", presetNames: [] },
  ],
  gemini: [
    { category: "official", presetNames: ["Google Official"] },
    {
      category: "cn_official",
      presetNames: [],
      emptyHintKey: "quickStart.gemini.cnOfficialHint",
    },
    { category: "aggregator", presetNames: ["OpenRouter"] },
    { category: "custom", presetNames: ["自定义"] },
  ],
};

export const QUICKSTART_CATEGORY_LABEL_KEYS: Record<
  QuickStartCategoryId,
  string
> = {
  official: "providerPreset.category.official",
  cn_official: "providerPreset.category.cnOfficial",
  aggregator: "providerPreset.category.aggregator",
  custom: "quickStart.category.custom",
};
