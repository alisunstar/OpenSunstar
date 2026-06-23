import { describe, expect, it } from "vitest";
import { getReadinessAction } from "@/lib/readinessActions";

describe("getReadinessAction", () => {
  it("routes incomplete MCP to project assets section", () => {
    expect(getReadinessAction("mcp_enabled", 0)).toEqual({
      type: "projectTab",
      section: "mcp",
    });
  });

  it("routes ignore rules to global ignore page", () => {
    expect(getReadinessAction("ignore_rules", 0)).toEqual({
      type: "navigate",
      view: "ignore",
    });
  });

  it("routes completed skills to manage in project tab", () => {
    expect(getReadinessAction("skills_configured", 15)).toEqual({
      type: "projectTab",
      section: "skills",
    });
  });
});
