import type { AppId } from "@/lib/api";

export const AGENT_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

export const AGENT_DISABLED_APP_KEYS = {
  hermes: "agents.syncDisabled.hermes",
} as const satisfies Partial<Record<AppId, string>>;
