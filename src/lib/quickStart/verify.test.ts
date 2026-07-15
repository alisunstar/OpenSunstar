import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import { resolvePresetByName } from "./resolvePresets";
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

import { resolveVerifyBaseUrl, verifyQuickStartKey } from "./verify";

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
});
