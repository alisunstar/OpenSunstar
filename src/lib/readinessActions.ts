import type { PageView } from "@/App";

export type ProjectAssetSection = "mcp" | "skills" | "prompts";

export type ReadinessAction =
  | { type: "projectTab"; section?: ProjectAssetSection }
  | { type: "navigate"; view: PageView };

export function getReadinessAction(
  checkName: string,
  score: number,
): ReadinessAction | null {
  if (score > 0) {
    switch (checkName) {
      case "mcp_enabled":
        return { type: "projectTab", section: "mcp" };
      case "skills_configured":
        return { type: "projectTab", section: "skills" };
      case "prompt_files":
        return { type: "projectTab", section: "prompts" };
      default:
        return null;
    }
  }

  switch (checkName) {
    case "mcp_enabled":
      return { type: "projectTab", section: "mcp" };
    case "skills_configured":
      return { type: "projectTab", section: "skills" };
    case "prompt_files":
      return { type: "projectTab", section: "prompts" };
    case "ignore_rules":
      return { type: "navigate", view: "ignore" };
    case "permissions":
      return { type: "navigate", view: "permissions" };
    case "recent_updates":
      return { type: "projectTab" };
    default:
      return null;
  }
}

export function readinessActionLabelKey(
  checkName: string,
  score: number,
): string {
  if (score > 0) {
    return "kanban.readiness.manage";
  }
  switch (checkName) {
    case "ignore_rules":
    case "permissions":
      return "kanban.readiness.configureGlobal";
    default:
      return "kanban.readiness.configure";
  }
}
