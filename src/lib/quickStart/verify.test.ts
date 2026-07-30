import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import { resolvePresetByName } from "./resolvePresets";
import { defaultAdvancedFields } from "./buildProvider";
import type { QuickStartFormFields, QuickStartSelection } from "./types";

const { fetchModelsForConfigMock, verifyProviderKeyMock } = vi.hoisted(() => ({
  fetchModelsForConfigMock: vi.fn(),
  verifyProviderKeyMock: vi.fn(),
}));

vi.mock("@/lib/api/model-fetch", () => ({
  fetchModelsForConfig: fetchModelsForConfigMock,
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    verifyProviderKey: verifyProviderKeyMock,
  },
}));

import {
  inferVerifyProtocol,
  resolveVerifyBaseUrl,
  verifyQuickStartKey,
} from "./verify";

const codexSelection: QuickStartSelection = {
  mode: "preset",
  appId: "codex",
  presetName: "DeepSeek",
  isOfficial: false,
};

const fields: QuickStartFormFields = {
  apiKey: " sk-test ",
  customName: "",
  customBaseUrl: "",
  customModel: "",
};

const t = (_key: string, options?: Record<string, unknown>) =>
  String(options?.defaultValue ?? _key);

describe("QuickStart verification contract", () => {
  beforeEach(() => {
    fetchModelsForConfigMock.mockReset();
    verifyProviderKeyMock.mockReset();
  });

  it("resolves the Codex preset base URL from its settings contract", () => {
    const preset = resolvePresetByName("codex", "DeepSeek");
    const raw = preset?.raw as CodexProviderPreset;
    const expected = raw.endpointCandidates?.[0];

    expect(expected).toBeTruthy();
    expect(resolveVerifyBaseUrl("codex", codexSelection, fields)).toBe(
      expected,
    );
  });

  it("passes positional baseUrl and apiKey arguments to model discovery", async () => {
    const preset = resolvePresetByName("codex", "DeepSeek");
    const expectedBaseUrl = (preset?.raw as CodexProviderPreset)
      .endpointCandidates?.[0];

    verifyProviderKeyMock.mockResolvedValue({
      ok: true,
      modelCount: 1,
      error: null,
    });
    fetchModelsForConfigMock.mockResolvedValue([
      { id: "deepseek-chat", ownedBy: "deepseek" },
    ]);

    const outcome = await verifyQuickStartKey(
      "codex",
      codexSelection,
      fields,
      t,
    );

    expect(verifyProviderKeyMock).toHaveBeenCalledWith(
      expectedBaseUrl,
      "sk-test",
      "openai",
    );
    expect(fetchModelsForConfigMock.mock.calls[0]?.[0]).toBe(expectedBaseUrl);
    expect(fetchModelsForConfigMock.mock.calls[0]?.[1]).toBe("sk-test");
    expect(outcome).toMatchObject({
      ok: true,
      models: [{ id: "deepseek-chat" }],
    });
  });

  it("uses custom advanced API format and Gemini endpoint overrides during verification", () => {
    expect(
      inferVerifyProtocol(
        "claude",
        { mode: "custom", appId: "claude" },
        {
          ...fields,
          customBaseUrl: "https://anthropic.example",
          advancedClaude: {
            apiFormat: "openai_responses",
            apiKeyField: "ANTHROPIC_API_KEY",
            haikuModel: "fast",
            haikuModelName: "Fast",
            sonnetModel: "main",
            sonnetModelName: "Main",
            sonnetSupports1m: false,
            opusModel: "strong",
            opusModelName: "Strong",
            opusSupports1m: false,
            fableModel: "balanced",
            fableModelName: "Balanced",
            fableSupports1m: false,
            subagentModel: "agent",
            subagentSupports1m: false,
            fallbackModel: "main",
          },
        },
      ),
    ).toBe("openai");

    expect(
      resolveVerifyBaseUrl(
        "gemini",
        { mode: "custom", appId: "gemini" },
        {
          ...fields,
          customBaseUrl: "https://basic.example",
          advancedGemini: {
            apiFormat: "gemini_native",
            baseUrl: "https://advanced.example/",
            model: "gemini-custom",
          },
        },
      ),
    ).toBe("https://advanced.example");

    expect(
      inferVerifyProtocol(
        "gemini",
        { mode: "custom", appId: "gemini" },
        {
          ...fields,
          advancedGemini: {
            apiFormat: "gemini_native",
            baseUrl: "https://advanced.example/",
            model: "gemini-custom",
          },
        },
      ),
    ).toBe("gemini");
  });

  it("selects native Gemini verification for the Gemini API profile", () => {
    expect(
      inferVerifyProtocol(
        "gemini",
        {
          mode: "preset",
          appId: "gemini",
          presetName: "Gemini API",
          isOfficial: false,
        },
        fields,
      ),
    ).toBe("gemini");

    const claudeGeminiFields = {
      ...fields,
      ...defaultAdvancedFields("claude", {
        mode: "preset",
        appId: "claude",
        presetName: "Gemini Native",
        isOfficial: false,
      }),
    };
    expect(
      inferVerifyProtocol(
        "claude",
        {
          mode: "preset",
          appId: "claude",
          presetName: "Gemini Native",
          isOfficial: false,
        },
        claudeGeminiFields,
      ),
    ).toBe("gemini");
  });
});
