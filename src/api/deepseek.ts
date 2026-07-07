/**
 * DeepSeek / GLM AI 提供方配置
 *
 * 密钥存储于 OS Keychain（Rust 后端）；非敏感元数据存 SQLite settings。
 */

import { invoke } from "@tauri-apps/api/core";

// ── 类型 ──────────────────────────────────────────

export type AiProvider = "deepseek" | "glm" | "custom";

export interface DeepseekSettings {
  apiKeyConfigured: boolean;
}

export interface GlmSettings {
  apiKeyConfigured: boolean;
  apiUrl: string;
  model: string;
}

export interface CustomProviderSettings {
  apiKeyConfigured: boolean;
  apiUrl: string;
  model: string;
}

export interface AiInsightProviderSettingsView {
  provider: string;
  deepseek_configured: boolean;
  glm_api_url: string;
  glm_model: string;
  glm_configured: boolean;
  custom_api_url: string;
  custom_model: string;
  custom_configured: boolean;
}

// ── localStorage 迁移（一次性）────────────────────

const PROVIDER_KEY = "OpenSunstar-ai-provider";
const DEEPSEEK_KEY = "OpenSunstar-deepseek-key";
const GLM_KEY = "OpenSunstar-glm";
const CUSTOM_KEY = "OpenSunstar-custom";
const MIGRATED_KEY = "OpenSunstar-ai-insight-keychain-v1";

function safeGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeRemove(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    /* noop */
  }
}

async function migrateLegacyAiSettingsIfNeeded(): Promise<void> {
  if (safeGet(MIGRATED_KEY)) return;

  const provider = (safeGet(PROVIDER_KEY) as AiProvider) ?? "deepseek";
  await invoke("save_ai_insight_provider_choice", { provider });

  const deepseekKey = safeGet(DEEPSEEK_KEY)?.trim();
  if (deepseekKey) {
    await invoke("save_ai_insight_deepseek_key", { apiKey: deepseekKey });
  }

  try {
    const glmRaw = safeGet(GLM_KEY);
    if (glmRaw) {
      const parsed = JSON.parse(glmRaw) as {
        _key?: string;
        apiUrl?: string;
        model?: string;
      };
      if (parsed._key?.trim()) {
        await invoke("save_ai_insight_glm_settings", {
          apiKey: parsed._key,
          apiUrl: parsed.apiUrl ?? "",
          model: parsed.model ?? "",
        });
      }
    }
  } catch {
    /* ignore */
  }

  try {
    const customRaw = safeGet(CUSTOM_KEY);
    if (customRaw) {
      const parsed = JSON.parse(customRaw) as {
        _key?: string;
        apiUrl?: string;
        model?: string;
      };
      if (parsed._key?.trim()) {
        await invoke("save_ai_insight_custom_settings", {
          apiKey: parsed._key,
          apiUrl: parsed.apiUrl ?? "",
          model: parsed.model ?? "",
        });
      }
    }
  } catch {
    /* ignore */
  }

  [PROVIDER_KEY, DEEPSEEK_KEY, GLM_KEY, CUSTOM_KEY].forEach(safeRemove);
  localStorage.setItem(MIGRATED_KEY, "1");
}

async function loadSettingsView(): Promise<AiInsightProviderSettingsView> {
  await migrateLegacyAiSettingsIfNeeded();
  return invoke<AiInsightProviderSettingsView>(
    "get_ai_insight_provider_settings",
  );
}

// ── AI 提供方 ────────────────────────────────────

export async function getAiProvider(): Promise<AiProvider> {
  const view = await loadSettingsView();
  return (view.provider as AiProvider) ?? "deepseek";
}

export async function saveAiProvider(provider: AiProvider): Promise<void> {
  await invoke("save_ai_insight_provider_choice", { provider });
}

// ── DeepSeek API ─────────────────────────────────

export async function getDeepseekSettings(): Promise<DeepseekSettings> {
  const view = await loadSettingsView();
  return { apiKeyConfigured: view.deepseek_configured };
}

export async function saveDeepseekSettings(apiKey: string): Promise<boolean> {
  return invoke<boolean>("save_ai_insight_deepseek_key", { apiKey });
}

export async function testDeepseekConnection(apiKeyOverride?: string): Promise<{
  ok: boolean;
  message: string;
}> {
  const key = apiKeyOverride?.trim();
  if (!key) {
    const cfg = await invoke<{
      api_key: string;
    } | null>("build_ai_insight_provider_config");
    if (!cfg?.api_key) return { ok: false, message: "未配置 API Key" };
    return testDeepseekConnection(cfg.api_key);
  }

  try {
    const res = await fetch("https://api.deepseek.com/v1/models", {
      headers: { Authorization: `Bearer ${key}` },
    });
    if (res.ok) {
      return { ok: true, message: "连接成功" };
    }
    const text = await res.text();
    return { ok: false, message: `HTTP ${res.status}: ${text.slice(0, 200)}` };
  } catch (e) {
    return { ok: false, message: String(e) };
  }
}

// ── GLM API ──────────────────────────────────────

export async function getGlmSettings(): Promise<GlmSettings> {
  const view = await loadSettingsView();
  return {
    apiKeyConfigured: view.glm_configured,
    apiUrl: view.glm_api_url,
    model: view.glm_model,
  };
}

export async function saveGlmSettings(
  apiKey: string,
  apiUrl: string,
  model: string,
): Promise<boolean> {
  return invoke<boolean>("save_ai_insight_glm_settings", {
    apiKey,
    apiUrl,
    model,
  });
}

export async function testGlmConnection(
  apiKeyOverride?: string,
  apiUrlOverride?: string,
  modelOverride?: string,
): Promise<{ ok: boolean; message: string }> {
  const view = await loadSettingsView();
  let key = apiKeyOverride?.trim() ?? "";
  if (!key) {
    const cfg = await invoke<{ api_key: string } | null>(
      "build_ai_insight_provider_config",
    );
    key = cfg?.api_key ?? "";
  }
  if (!key) return { ok: false, message: "未配置 API Key" };

  const url = apiUrlOverride?.trim() || view.glm_api_url;
  const model = modelOverride?.trim() || view.glm_model;

  try {
    const res = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${key}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model,
        messages: [{ role: "user", content: "ping" }],
        max_tokens: 1,
      }),
    });
    if (res.ok) return { ok: true, message: "连接成功" };
    const text = await res.text();
    return { ok: false, message: `HTTP ${res.status}: ${text.slice(0, 200)}` };
  } catch (e) {
    return { ok: false, message: String(e) };
  }
}

// ── 自定义提供方 API ────────────────────────────

export async function getCustomProviderSettings(): Promise<CustomProviderSettings> {
  const view = await loadSettingsView();
  return {
    apiKeyConfigured: view.custom_configured,
    apiUrl: view.custom_api_url,
    model: view.custom_model,
  };
}

export async function saveCustomProviderSettings(
  apiKey: string,
  apiUrl: string,
  model: string,
): Promise<boolean> {
  return invoke<boolean>("save_ai_insight_custom_settings", {
    apiKey,
    apiUrl,
    model,
  });
}

export async function testCustomProviderConnection(
  apiKeyOverride?: string,
  apiUrlOverride?: string,
  modelOverride?: string,
): Promise<{ ok: boolean; message: string }> {
  const view = await loadSettingsView();
  let key = apiKeyOverride?.trim() ?? "";
  if (!key) {
    const cfg = await invoke<{ api_key: string } | null>(
      "build_ai_insight_provider_config",
    );
    key = cfg?.api_key ?? "";
  }
  if (!key) return { ok: false, message: "未配置 API Key" };

  const url = apiUrlOverride?.trim() || view.custom_api_url;
  const model = modelOverride?.trim() || view.custom_model;

  try {
    const res = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${key}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model,
        messages: [{ role: "user", content: "ping" }],
        max_tokens: 1,
      }),
    });
    if (res.ok) return { ok: true, message: "连接成功" };
    const text = await res.text();
    return { ok: false, message: `HTTP ${res.status}: ${text.slice(0, 200)}` };
  } catch (e) {
    return { ok: false, message: String(e) };
  }
}
