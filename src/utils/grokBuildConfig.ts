/** Grok Build 原生配置的默认模型。 */
export const GROK_BUILD_DEFAULT_MODEL = "grok-4.5";

export interface GrokBuildConfigValues {
  profile: string;
  model: string;
  baseUrl: string;
  name: string;
  apiKey: string;
  envKey: string;
  apiBackend: "responses" | "chat";
  contextWindow: number;
}

const tomlString = (value: string) => JSON.stringify(value);

export function parseGrokBuildConfig(value: string): GrokBuildConfigValues {
  const readString = (key: string, fallback = "") =>
    value.match(new RegExp(`^${key}\\s*=\\s*["']([^"']*)["']`, "m"))?.[1] ??
    fallback;
  const profile = value.match(/^\[model\.([^\]]+)\]/m)?.[1] ?? "custom";
  const contextWindow = Number.parseInt(
    value.match(/^context_window\s*=\s*(\d+)/m)?.[1] ?? "500000",
    10,
  );
  return {
    profile,
    model: readString("model", GROK_BUILD_DEFAULT_MODEL),
    baseUrl: readString("base_url"),
    name: readString("name", profile),
    apiKey: readString("api_key"),
    envKey: readString("env_key"),
    apiBackend:
      readString("api_backend", "responses") === "chat" ? "chat" : "responses",
    contextWindow: Number.isFinite(contextWindow) ? contextWindow : 500000,
  };
}

export function buildGrokBuildConfig(
  values: Partial<GrokBuildConfigValues>,
): string {
  const profile = (values.profile ?? "custom").trim() || "custom";
  const model =
    (values.model ?? GROK_BUILD_DEFAULT_MODEL).trim() ||
    GROK_BUILD_DEFAULT_MODEL;
  const baseUrl = (values.baseUrl ?? "").trim();
  const name = (values.name ?? profile).trim() || profile;
  const apiKey = (values.apiKey ?? "").trim();
  const envKey = (values.envKey ?? "").trim();
  const apiBackend = values.apiBackend === "chat" ? "chat" : "responses";
  const contextWindow =
    values.contextWindow && values.contextWindow > 0
      ? values.contextWindow
      : 500000;
  return `[models]\ndefault = ${tomlString(profile)}\n\n[model.${profile}]\nmodel = ${tomlString(model)}\nbase_url = ${tomlString(baseUrl)}\nname = ${tomlString(name)}\n${apiKey ? `api_key = ${tomlString(apiKey)}\n` : ""}${envKey ? `env_key = ${tomlString(envKey)}\n` : ""}api_backend = ${tomlString(apiBackend)}\ncontext_window = ${contextWindow}\n`;
}

export function parseGrokProviderSettings(
  settings: unknown,
): GrokBuildConfigValues {
  if (!settings || typeof settings !== "object") {
    return parseGrokBuildConfig("");
  }
  const config = (settings as Record<string, unknown>).config;
  return parseGrokBuildConfig(typeof config === "string" ? config : "");
}
