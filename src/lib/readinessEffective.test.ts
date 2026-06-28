import { describe, expect, it } from "vitest";
import type { AgentReadinessItem } from "@/api/aiInsight";
import {
  effectiveBadgeTone,
  hasEffectiveScan,
  resolveConfiguredState,
} from "@/lib/readinessEffective";

function item(partial: Partial<AgentReadinessItem>): AgentReadinessItem {
  return {
    check_name: "mcp_enabled",
    label: "MCP",
    weight: 15,
    score: 0,
    detail: "",
    ...partial,
  };
}

describe("readinessEffective", () => {
  it("resolveConfiguredState prefers explicit field", () => {
    expect(
      resolveConfiguredState(
        item({ configured_state: "configured", score: 0 }),
      ),
    ).toBe("configured");
  });

  it("resolveConfiguredState falls back to score", () => {
    expect(resolveConfiguredState(item({ score: 15 }))).toBe("configured");
    expect(resolveConfiguredState(item({ score: 0 }))).toBe("unconfigured");
  });

  it("hasEffectiveScan requires state and timestamp", () => {
    expect(
      hasEffectiveScan(
        item({ effective_state: "effective", effective_scanned_at: 1 }),
      ),
    ).toBe(true);
    expect(hasEffectiveScan(item({ effective_state: "effective" }))).toBe(
      false,
    );
  });

  it("effectiveBadgeTone maps states", () => {
    expect(effectiveBadgeTone("effective")).toBe("success");
    expect(effectiveBadgeTone("drifted")).toBe("warning");
    expect(effectiveBadgeTone("unchecked")).toBe("muted");
  });
});
