import { describe, expect, it } from "vitest";
import { getReadinessAction } from "@/lib/readinessActions";

describe("getReadinessAction", () => {
  it("routes incomplete MCP to project assets section", () => {
    expect(getReadinessAction("mcp_enabled", 0)).toEqual({
      type: "projectTab",
      section: "mcp",
    });
  });

  it("routes ignore rules to project assets ignore section", () => {
    expect(getReadinessAction("ignore_rules", 0)).toEqual({
      type: "projectTab",
      section: "ignore",
    });
  });

  it("routes completed skills to manage in project tab", () => {
    expect(getReadinessAction("skills_configured", 12)).toEqual({
      type: "projectTab",
      section: "skill",
    });
  });

  it("routes commands to project tab", () => {
    expect(getReadinessAction("commands_configured", 0)).toEqual({
      type: "projectTab",
      section: "command",
    });
  });
});
