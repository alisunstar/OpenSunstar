import { describe, expect, it } from "vitest";
import {
  buildGrokBuildConfig,
  parseGrokBuildConfig,
  parseGrokProviderSettings,
} from "./grokBuildConfig";

describe("grokBuildConfig", () => {
  it("round-trips the native model profile fields", () => {
    const config = buildGrokBuildConfig({
      profile: "xai",
      model: "grok-4.5",
      baseUrl: "https://api.x.ai/v1",
      name: "xAI",
      apiKey: "xai-test-key",
      apiBackend: "responses",
      contextWindow: 500000,
    });

    expect(parseGrokBuildConfig(config)).toMatchObject({
      profile: "xai",
      model: "grok-4.5",
      baseUrl: "https://api.x.ai/v1",
      name: "xAI",
      apiKey: "xai-test-key",
      apiBackend: "responses",
      contextWindow: 500000,
    });
  });

  it("reads the config carrier used by provider records", () => {
    expect(
      parseGrokProviderSettings({
        config: buildGrokBuildConfig({
          profile: "relay",
          baseUrl: "https://relay.example/v1",
          apiKey: "relay-key",
        }),
      }),
    ).toMatchObject({ profile: "relay", baseUrl: "https://relay.example/v1" });
  });
});
