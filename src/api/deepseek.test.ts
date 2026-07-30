import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  getCustomProviderSettings,
  getDeepseekSettings,
  getGlmSettings,
} from "@/api/deepseek";

const invoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("AI 提供方设置 IPC 字段映射", () => {
  beforeEach(() => {
    localStorage.setItem("OpenSunstar-ai-insight-keychain-v1", "1");
    invoke.mockReset();
    invoke.mockResolvedValue({
      provider: "deepseek",
      deepseekConfigured: true,
      glmApiUrl: "https://glm.example/v1/chat/completions",
      glmModel: "glm-test",
      glmConfigured: true,
      customApiUrl: "https://custom.example/v1/chat/completions",
      customModel: "custom-test",
      customConfigured: true,
    });
  });

  it("按 Rust camelCase 响应识别 DeepSeek Key 状态", async () => {
    await expect(getDeepseekSettings()).resolves.toEqual({
      apiKeyConfigured: true,
    });
  });

  it("按 Rust camelCase 响应读取 GLM 与自定义 Provider 元数据", async () => {
    await expect(getGlmSettings()).resolves.toEqual({
      apiKeyConfigured: true,
      apiUrl: "https://glm.example/v1/chat/completions",
      model: "glm-test",
    });
    await expect(getCustomProviderSettings()).resolves.toEqual({
      apiKeyConfigured: true,
      apiUrl: "https://custom.example/v1/chat/completions",
      model: "custom-test",
    });
  });
});
