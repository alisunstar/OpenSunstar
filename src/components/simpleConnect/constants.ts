export const TOOL_LABELS: Record<string, string> = {
  "claude-code": "Claude Code",
  codex: "Codex",
  "gemini-cli": "Gemini CLI",
  opencode: "OpenCode",
  openclaw: "OpenClaw",
  hermes: "Hermes",
};

export const TOOL_HINTS: Record<string, string> = {
  "claude-code": "Anthropic-compatible",
  codex: "OpenAI-compatible",
  "gemini-cli": "Gemini / OpenAI bridge",
  opencode: "OpenCode provider",
  openclaw: "OpenClaw models.providers",
  hermes: "Hermes custom_providers",
};

export const SUPPLIER_ACCENTS: Record<string, string> = {
  deepseek: "from-blue-500/15 to-cyan-500/10 border-blue-500/30",
  openrouter: "from-violet-500/15 to-purple-500/10 border-violet-500/30",
  zhipu: "from-emerald-500/15 to-teal-500/10 border-emerald-500/30",
  anthropic: "from-orange-500/15 to-amber-500/10 border-orange-500/30",
  custom: "from-slate-500/15 to-zinc-500/10 border-border/60",
};
