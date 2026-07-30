import { describe, expect, it } from "vitest";
import { QUICKSTART_CURATED } from "@/config/quickStartCurated";
import {
  getCuratedPresetGroups,
  resolvePresetByName,
  validateCuratedPresetNames,
} from "@/lib/quickStart/resolvePresets";

const expectedInternational = [
  "OpenAI",
  "Anthropic",
  "Gemini API",
  "OpenCode Go",
  "xAI (Grok) API",
  "GitHub Copilot",
  "MiniMax(国际)",
  "Z.ai",
  "SiliconFlow",
  "DMXAPI",
  "Agnes AI",
];

const expectedChina = [
  "DeepSeek",
  "Kimi",
  "Zhipu GLM",
  "MiniMax",
  "Xiaomi MiMo",
  "StepFun",
  "Longcat",
  "BaiLing",
];

describe("quickStartCurated", () => {
  it("all curated preset names resolve in their app libraries", () => {
    const errors = validateCuratedPresetNames();
    expect(errors).toEqual([]);
  });

  it.each(Object.keys(QUICKSTART_CURATED))(
    "%s keeps the required Chinese group order and provider order",
    (appId) => {
      const groups = getCuratedPresetGroups(
        appId as keyof typeof QUICKSTART_CURATED,
        "",
      ).filter((group) => group.category !== "official");

      expect(groups.map((group) => group.category)).toEqual([
        "ai_global",
        "ai_china",
        "relay",
        "custom",
      ]);
      expect(groups[0]?.presets.map((preset) => preset.displayName)).toEqual(
        expectedInternational,
      );
      expect(groups[1]?.presets.map((preset) => preset.displayName)).toEqual(
        expectedChina,
      );
      expect(groups[2]?.presets.map((preset) => preset.displayName)).toEqual([
        "硅基流动中国",
        "OpenRouter",
      ]);
    },
  );

  it("exposes usable supplemental profiles with API format and endpoint metadata", () => {
    const groups = getCuratedPresetGroups("claude", "");
    const international = groups.find(
      (group) => group.category === "ai_global",
    );
    const agnes = international?.presets.find(
      (preset) => preset.displayName === "Agnes AI",
    );
    const xai = international?.presets.find(
      (preset) => preset.displayName === "xAI (Grok) API",
    );

    expect(agnes).toMatchObject({ authMode: "api_key" });
    expect(xai).toMatchObject({ authMode: "api_key" });
    expect(agnes?.unavailable).toBeFalsy();
    expect(xai?.unavailable).toBeFalsy();
    expect(JSON.stringify(agnes?.raw)).toContain(
      "https://apihub.agnes-ai.com/v1",
    );
    expect(JSON.stringify(xai?.raw)).toContain("https://api.x.ai/v1");
  });

  it("documents every intentionally disabled card with a concrete protocol reason", () => {
    const codexDisabled = getCuratedPresetGroups("codex", "")
      .flatMap((group) => group.presets)
      .filter((preset) => preset.unavailable);
    expect(codexDisabled.map((preset) => preset.displayName)).toEqual([
      "Anthropic",
      "GitHub Copilot",
    ]);

    const geminiDisabled = getCuratedPresetGroups("gemini", "")
      .flatMap((group) => group.presets)
      .filter((preset) => preset.unavailable);
    expect(geminiDisabled).toHaveLength(20);
    expect(geminiDisabled.at(-1)).toMatchObject({
      displayName: "OpenRouter",
      unavailableReason:
        "OpenRouter 提供 OpenAI Chat 上游，Gemini CLI 当前透传 Gemini Native 协议",
    });
    for (const preset of [...codexDisabled, ...geminiDisabled]) {
      expect(preset.unavailableReason).toBeTruthy();
    }
  });

  it("uses reviewed current defaults for volatile provider models and protocols", () => {
    const anthropic = resolvePresetByName("claude", "Anthropic API")?.raw as {
      settingsConfig?: { env?: Record<string, string> };
    };
    expect(anthropic.settingsConfig?.env).toMatchObject({
      ANTHROPIC_MODEL: "claude-sonnet-5",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "claude-haiku-4-5",
      ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-sonnet-5[1M]",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-opus-5[1M]",
      ANTHROPIC_DEFAULT_FABLE_MODEL: "claude-fable-5[1M]",
    });
    expect(
      JSON.stringify(resolvePresetByName("claude", "Kimi")?.raw),
    ).toContain("kimi-k2.7-code");
    expect(
      JSON.stringify(resolvePresetByName("claude", "Longcat")?.raw),
    ).toContain("LongCat-2.0");
    expect(resolvePresetByName("codex", "MiniMax")?.raw).toMatchObject({
      apiFormat: "openai_responses",
    });
    expect(
      JSON.stringify(resolvePresetByName("codex", "Zhipu GLM")?.raw),
    ).toContain("glm-5.2");
    expect(
      JSON.stringify(resolvePresetByName("gemini", "Gemini API")?.raw),
    ).toContain("gemini-3.6-flash");
  });
});
