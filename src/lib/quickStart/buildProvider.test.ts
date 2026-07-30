import { describe, expect, it } from "vitest";
import {
  buildQuickStartProviderInput,
  defaultAdvancedFields,
} from "./buildProvider";
import type { QuickStartSelection } from "./types";

describe("QuickStart advanced configuration", () => {
  it("initializes a populated collapsed-panel model for every client", () => {
    for (const appId of [
      "claude",
      "claude-desktop",
      "codex",
      "gemini",
    ] as const) {
      const selection: QuickStartSelection = { mode: "custom", appId };
      const fields = defaultAdvancedFields(appId, selection);
      expect(
        fields.advancedClaude ??
          fields.advancedDesktop ??
          fields.advancedCodex ??
          fields.advancedGemini,
      ).toBeTruthy();
    }
  });

  it("persists Claude API format, credential field, and per-role model mapping", () => {
    const selection: QuickStartSelection = {
      mode: "custom",
      appId: "claude",
    };
    const provider = buildQuickStartProviderInput(
      "claude",
      selection,
      {
        apiKey: "sk-openai",
        customName: "OpenAI gateway",
        customBaseUrl: "https://gateway.example/v1/",
        customModel: "fallback-model",
        advancedClaude: {
          apiFormat: "openai_responses",
          apiKeyField: "ANTHROPIC_API_KEY",
          haikuModel: "fast-model",
          haikuModelName: "Fast",
          sonnetModel: "main-model",
          sonnetModelName: "Main",
          sonnetSupports1m: true,
          opusModel: "strong-model",
          opusModelName: "Strong",
          opusSupports1m: true,
          fableModel: "balanced-model",
          fableModelName: "Balanced",
          fableSupports1m: false,
          subagentModel: "agent-model",
          subagentSupports1m: true,
          fallbackModel: "fallback-model",
        },
      },
      "Custom",
    );
    const env = provider.settingsConfig.env as Record<string, string>;

    expect(provider.meta).toMatchObject({
      apiFormat: "openai_responses",
      apiKeyField: "ANTHROPIC_API_KEY",
    });
    expect(env).toMatchObject({
      ANTHROPIC_BASE_URL: "https://gateway.example/v1",
      ANTHROPIC_API_KEY: "sk-openai",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "fast-model",
      ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME: "Fast",
      ANTHROPIC_DEFAULT_SONNET_MODEL: "main-model[1M]",
      ANTHROPIC_DEFAULT_SONNET_MODEL_NAME: "Main",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "strong-model[1M]",
      ANTHROPIC_DEFAULT_OPUS_MODEL_NAME: "Strong",
      ANTHROPIC_DEFAULT_FABLE_MODEL: "balanced-model",
      ANTHROPIC_DEFAULT_FABLE_MODEL_NAME: "Balanced",
      CLAUDE_CODE_SUBAGENT_MODEL: "agent-model[1M]",
      ANTHROPIC_MODEL: "fallback-model",
    });
    expect(env).not.toHaveProperty("ANTHROPIC_AUTH_TOKEN");
  });

  it("keeps the current Anthropic role tiers when building the reviewed preset", () => {
    const selection: QuickStartSelection = {
      mode: "preset",
      appId: "claude",
      presetName: "Anthropic API",
      isOfficial: false,
    };
    const advanced = defaultAdvancedFields("claude", selection);
    expect(advanced.advancedClaude).toMatchObject({
      sonnetModel: "claude-sonnet-5",
      sonnetSupports1m: true,
      opusModel: "claude-opus-5",
      opusSupports1m: true,
      fableModel: "claude-fable-5",
      fableSupports1m: true,
      haikuModel: "claude-haiku-4-5",
    });

    const provider = buildQuickStartProviderInput(
      "claude",
      selection,
      {
        apiKey: "sk-ant",
        customName: "",
        customBaseUrl: "",
        customModel: "",
        ...advanced,
      },
      "Anthropic",
    );
    const env = provider.settingsConfig.env as Record<string, string>;
    expect(env).toMatchObject({
      ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-sonnet-5[1M]",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-opus-5[1M]",
      ANTHROPIC_DEFAULT_FABLE_MODEL: "claude-fable-5[1M]",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "claude-haiku-4-5",
    });
  });

  it("persists Codex upstream format and advanced default model", () => {
    const selection: QuickStartSelection = {
      mode: "custom",
      appId: "codex",
    };
    const provider = buildQuickStartProviderInput(
      "codex",
      selection,
      {
        apiKey: "sk-chat",
        customName: "Chat gateway",
        customBaseUrl: "https://chat.example/v1",
        customModel: "fallback-model",
        advancedCodex: {
          apiFormat: "openai_chat",
          defaultModel: "agnes-2.0-flash",
        },
      },
      "Custom",
    );

    expect(provider.meta?.apiFormat).toBe("openai_chat");
    expect(String(provider.settingsConfig.config)).toContain(
      'model = "agnes-2.0-flash"',
    );
  });

  it("persists Gemini protocol, endpoint override, and advanced model", () => {
    const selection: QuickStartSelection = {
      mode: "custom",
      appId: "gemini",
    };
    const provider = buildQuickStartProviderInput(
      "gemini",
      selection,
      {
        apiKey: "AIza-test",
        customName: "Gemini gateway",
        customBaseUrl: "https://basic.example",
        customModel: "fallback-model",
        advancedGemini: {
          apiFormat: "gemini_native",
          baseUrl: "https://native.example/v1beta/",
          model: "gemini-custom",
        },
      },
      "Custom",
    );
    const env = provider.settingsConfig.env as Record<string, string>;

    expect(provider.meta?.apiFormat).toBe("gemini_native");
    expect(env).toMatchObject({
      GOOGLE_GEMINI_BASE_URL: "https://native.example/v1beta/",
      GEMINI_API_KEY: "AIza-test",
      GEMINI_MODEL: "gemini-custom",
    });
  });

  it("persists distinct Claude Desktop role routes instead of flattening them", () => {
    const selection: QuickStartSelection = {
      mode: "custom",
      appId: "claude-desktop",
    };
    const provider = buildQuickStartProviderInput(
      "claude-desktop",
      selection,
      {
        apiKey: "desktop-key",
        customName: "Tiered gateway",
        customBaseUrl: "https://desktop.example",
        customModel: "main",
        advancedDesktop: {
          apiFormat: "openai_responses",
          sonnetModel: "gpt-5.6-terra",
          sonnetLabel: "Terra",
          sonnetSupports1m: false,
          opusModel: "gpt-5.6-sol",
          opusLabel: "Sol",
          opusSupports1m: true,
          haikuModel: "gpt-5.6-luna",
          haikuLabel: "Luna",
          haikuSupports1m: false,
        },
      },
      "Custom",
    );

    expect(provider.meta?.claudeDesktopModelRoutes).toMatchObject({
      "claude-sonnet-4-6": { model: "gpt-5.6-terra", labelOverride: "Terra" },
      "claude-opus-4-8": {
        model: "gpt-5.6-sol",
        labelOverride: "Sol",
        supports1m: true,
      },
      "claude-haiku-4-5": { model: "gpt-5.6-luna", labelOverride: "Luna" },
    });
  });
});
