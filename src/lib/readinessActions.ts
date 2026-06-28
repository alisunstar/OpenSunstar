import type { PageView } from "@/App";

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

