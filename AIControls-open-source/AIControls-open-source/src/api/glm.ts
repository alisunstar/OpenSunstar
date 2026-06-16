import { invoke } from "@tauri-apps/api/core";

export type GlmSettings = {
  apiKeyConfigured: boolean;
  apiUrl: string;
  model: string;
};

export async function getAiProvider(): Promise<string | null> {
  try {
    return await invoke<string>("get_ai_provider");
  } catch {
    return null;
  }
}

export async function saveAiProvider(provider: string): Promise<boolean> {
  try {
    await invoke("save_ai_provider", { provider });
    return true;
  } catch {
    return false;
  }
}

export async function getGlmSettings(): Promise<GlmSettings | null> {
  try {
    return await invoke<GlmSettings>("get_glm_settings");
  } catch {
    return null;
  }
}

export async function saveGlmSettings(
  apiKey: string,
  apiUrl: string,
  model: string,
): Promise<boolean> {
  try {
    await invoke("save_glm_settings", { apiKey, apiUrl, model });
    return true;
  } catch {
    return false;
  }
}

export async function testGlmConnection(): Promise<{
  ok: boolean;
  message: string;
}> {
  try {
    const msg = await invoke<string>("test_glm_connection");
    return { ok: true, message: msg };
  } catch (e) {
    return { ok: false, message: e instanceof Error ? e.message : String(e) };
  }
}
