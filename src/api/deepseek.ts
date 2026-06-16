/**
 * DeepSeek / GLM AI 提供方配置（对齐 AIControls）
 *
 * 一期使用 localStorage 存储密钥；后续 Rust 后端补充
 * `save_deepseek_settings` / `get_deepseek_settings` 等 Tauri 命令后，
 * 将 localStorage 调用替换为 invoke。
 */

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

// ── 存储层（localStorage 降级）────────────────────

const PROVIDER_KEY = "OpenSunstar-ai-provider";
const DEEPSEEK_KEY = "OpenSunstar-deepseek-key";
const GLM_KEY_PREFIX = "OpenSunstar-glm";
const CUSTOM_KEY_PREFIX = "OpenSunstar-custom";

function safeGet(key: string): string | null {
  try { return localStorage.getItem(key); } catch { return null; }
}
function safeSet(key: string, value: string): void {
  try { localStorage.setItem(key, value); } catch { /* noop */ }
}
function safeRemove(key: string): void {
  try { localStorage.removeItem(key); } catch { /* noop */ }
}

// ── AI 提供方 ────────────────────────────────────

export function getAiProvider(): AiProvider {
  return (safeGet(PROVIDER_KEY) as AiProvider) ?? "deepseek";
}

export function saveAiProvider(provider: AiProvider): void {
  safeSet(PROVIDER_KEY, provider);
}

// ── DeepSeek API ─────────────────────────────────

export function getDeepseekSettings(): DeepseekSettings {
  const key = safeGet(DEEPSEEK_KEY);
  return { apiKeyConfigured: !!key && key.trim().length > 0 };
}

export function saveDeepseekSettings(apiKey: string): boolean {
  if (apiKey.trim().length === 0) {
    safeRemove(DEEPSEEK_KEY);
  } else {
    safeSet(DEEPSEEK_KEY, apiKey.trim());
  }
  return true;
}

export async function testDeepseekConnection(): Promise<{
  ok: boolean;
  message: string;
}> {
  const key = safeGet(DEEPSEEK_KEY);
  if (!key) return { ok: false, message: "未配置 API Key" };

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

export function getGlmSettings(): GlmSettings {
  try {
    const raw = safeGet(GLM_KEY_PREFIX);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { apiKeyConfigured: false, apiUrl: "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions", model: "GLM-5.1" };
}

export function saveGlmSettings(apiKey: string, apiUrl: string, model: string): boolean {
  safeSet(GLM_KEY_PREFIX, JSON.stringify({
    apiKeyConfigured: apiKey.trim().length > 0,
    apiUrl: apiUrl.trim() || "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions",
    model: model.trim() || "GLM-5.1",
    _key: apiKey.trim() || undefined,
  }));
  return true;
}

export async function testGlmConnection(): Promise<{
  ok: boolean;
  message: string;
}> {
  const settings = getGlmSettings();
  const raw = safeGet(GLM_KEY_PREFIX);
  let key = "";
  try {
    if (raw) key = (JSON.parse(raw))._key ?? "";
  } catch { /* ignore */ }
  if (!key) return { ok: false, message: "未配置 API Key" };

  try {
    const res = await fetch(settings.apiUrl, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${key}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: settings.model,
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

export function getCustomProviderSettings(): CustomProviderSettings {
  try {
    const raw = safeGet(CUSTOM_KEY_PREFIX);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return {
    apiKeyConfigured: false,
    apiUrl: "https://api.openai.com/v1/chat/completions",
    model: "gpt-4o",
  };
}

export function saveCustomProviderSettings(
  apiKey: string,
  apiUrl: string,
  model: string,
): boolean {
  safeSet(
    CUSTOM_KEY_PREFIX,
    JSON.stringify({
      apiKeyConfigured: apiKey.trim().length > 0,
      apiUrl: apiUrl.trim() || "https://api.openai.com/v1/chat/completions",
      model: model.trim() || "gpt-4o",
      _key: apiKey.trim() || undefined,
    }),
  );
  return true;
}

export async function testCustomProviderConnection(): Promise<{
  ok: boolean;
  message: string;
}> {
  const settings = getCustomProviderSettings();
  const raw = safeGet(CUSTOM_KEY_PREFIX);
  let key = "";
  try {
    if (raw) key = (JSON.parse(raw))._key ?? "";
  } catch { /* ignore */ }
  if (!key) return { ok: false, message: "未配置 API Key" };

  try {
    const res = await fetch(settings.apiUrl, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${key}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: settings.model,
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
