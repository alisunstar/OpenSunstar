/**
 * Raw user input for /cp-* /cps-* command segments shown in Prompt Library modals.
 * ASCII letters, digits, hyphen-separated words only (no CJK or other scripts).
 */
export const AGENT_COMMAND_SEGMENT_INPUT_RE =
  /^[a-zA-Z0-9]+(?:-[a-zA-Z0-9]+)*$/;

export function isValidAgentCommandSegmentInput(raw: string): boolean {
  const t = raw.trim();
  return t.length > 0 && AGENT_COMMAND_SEGMENT_INPUT_RE.test(t);
}

/** Lowercase slug after normalization; aligned with prompt_library Rust `is_valid_command_name`. */
export const AGENT_COMMAND_SEGMENT_SLUG_RE =
  /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function agentCommandSegmentInvalidMessage(locale: "zh" | "en"): string {
  return locale === "zh"
    ? "仅允许英文字母、数字与连字符（例如 prd-review），不能使用中文或其它符号。"
    : "Use only English letters, numbers, and hyphens (e.g. prd-review). No Chinese or other characters.";
}

/** Strip `/cp-`, `prompts:` prefixes from paths used when applying a Prompt to an Agent. */
export function normalizePromptApplyCommandSegment(raw: string): string {
  let s = raw.trim();
  if (!s) return "";
  s = s.replace(/^\/?prompts:/i, "");
  s = s.replace(/^\/?cp-/i, "");
  return s.trim();
}
