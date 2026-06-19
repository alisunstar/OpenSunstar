export const filenameMap: Record<string, string> = {
  claude: "CLAUDE.md",
  "claude-desktop": "CLAUDE.md",
  codex: "AGENTS.md",
  opencode: "AGENTS.md",
  openclaw: "AGENTS.md",
  hermes: "AGENTS.md",
  gemini: "GEMINI.md",
};

export function getPromptFilename(appId: string): string {
  return filenameMap[appId] || "AGENTS.md";
}
