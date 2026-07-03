import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";

export const GOVERNANCE_CHECK_LABELS: Record<string, string> = {
  mcp_enabled: "MCP",
  skills_configured: "Skills",
  prompt_files: "Prompts",
  commands_configured: "Commands",
  hooks_configured: "Hooks",
  ignore_rules: "Ignore",
  permissions: "Permissions",
  subagents_configured: "Subagents",
};

export interface GovernancePortfolioStats {
  totalProjects: number;
  scannedProjects: number;
  driftProjects: number;
  totalDriftItems: number;
  effectiveItems: number;
  comparableItems: number;
  driftByCheck: Array<{ checkName: string; label: string; count: number }>;
}

export function aggregateGovernanceStats(
  projects: Project[],
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>,
): GovernancePortfolioStats {
  const driftByCheck = new Map<string, number>();
  let driftProjects = 0;
  let totalDriftItems = 0;
  let effectiveItems = 0;
  let comparableItems = 0;
  let scannedProjects = 0;

  for (const project of projects) {
    const entry = agentReadinessMap.get(project.id);
    if (!entry) continue;
    scannedProjects += 1;
    if (entry.driftCount > 0) driftProjects += 1;
    totalDriftItems += entry.driftCount;

    for (const item of entry.details) {
      const state = item.effective_state;
      if (!state || state === "not_applicable" || state === "unchecked") continue;
      comparableItems += 1;
      if (state === "effective") {
        effectiveItems += 1;
      } else if (state === "drifted") {
        driftByCheck.set(
          item.check_name,
          (driftByCheck.get(item.check_name) ?? 0) + 1,
        );
      }
    }
  }

  const driftByCheckList = [...driftByCheck.entries()]
    .map(([checkName, count]) => ({
      checkName,
      label: GOVERNANCE_CHECK_LABELS[checkName] ?? checkName,
      count,
    }))
    .sort((a, b) => b.count - a.count);

  return {
    totalProjects: projects.length,
    scannedProjects,
    driftProjects,
    totalDriftItems,
    effectiveItems,
    comparableItems,
    driftByCheck: driftByCheckList,
  };
}
