import type { PageView } from "@/app/navigation";

import type { ProjectAssetSection } from "@/types/projectAsset";

export type { ProjectAssetSection };

export type ReadinessAction =
  | { type: "projectTab"; section?: ProjectAssetSection }
  | { type: "navigate"; view: PageView };

const CHECK_TO_SECTION: Record<string, ProjectAssetSection> = {
  mcp_enabled: "mcp",

  skills_configured: "skill",

  prompt_files: "prompt",

  commands_configured: "command",

  hooks_configured: "hook",

  ignore_rules: "ignore",

  permissions: "permission",

  subagents_configured: "subagent",
};

/**
 * readiness check_name → 「项目资产配置」资产区（工作区重构 2026-07-30）。
 * 资产矩阵缺口格与「今日」告警卡的深链都靠它定位，返回 undefined 表示
 * 该检查项没有对应资产区（如 `recent_updates` 维护度指标）。
 */
export function checkNameToAssetSection(
  checkName: string,
): ProjectAssetSection | undefined {
  return CHECK_TO_SECTION[checkName];
}

export function getReadinessAction(
  checkName: string,

  score: number,
): ReadinessAction | null {
  const section = CHECK_TO_SECTION[checkName];

  if (score > 0) {
    if (section) return { type: "projectTab", section };

    if (checkName === "recent_updates") return { type: "projectTab" };

    return null;
  }

  if (section) return { type: "projectTab", section };

  if (checkName === "recent_updates") return { type: "projectTab" };

  return null;
}

export function readinessActionLabelKey(
  _checkName: string,
  score: number,
): string {
  if (score > 0) return "kanban.readiness.manage";

  return "kanban.readiness.configure";
}
