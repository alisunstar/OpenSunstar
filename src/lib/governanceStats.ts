import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";

/**
 * 必须与 `agent_readiness.rs` 产出的 9 项一一对应。
 *
 * 第 9 项 `recent_updates` 之前不在表里（审查报告 §5.3），于是它在任何按
 * check_name 取名字的地方都会退化成裸的 snake_case 键名。它权重 9、计进总分，
 * 只是没有磁盘生效态可比对，因此永远不会出现在 `driftByCheck` 里。
 */
export const GOVERNANCE_CHECK_LABELS: Record<string, string> = {
  mcp_enabled: "MCP",
  skills_configured: "Skills",
  prompt_files: "Prompts",
  commands_configured: "Commands",
  hooks_configured: "Hooks",
  ignore_rules: "Ignore",
  permissions: "Permissions",
  subagents_configured: "Subagents",
  recent_updates: "维护度",
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
      if (!state || state === "not_applicable" || state === "unchecked")
        continue;
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
