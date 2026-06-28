import { describe, expect, it } from "vitest";
import {
  AGENT_READINESS_MAX,
  isReadinessOk,
  readinessMaxScore,
  readinessScoreTone,
} from "./readinessConstants";

describe("readinessConstants", () => {
  it("uses 100 as default max", () => {
    expect(readinessMaxScore()).toBe(AGENT_READINESS_MAX);
    expect(readinessMaxScore(undefined)).toBe(100);
  });

  it("ok threshold at 75 on 100-point scale", () => {
    expect(isReadinessOk(75)).toBe(true);
    expect(isReadinessOk(74)).toBe(false);
  });

  it("tone reflects score bands", () => {
    expect(readinessScoreTone(80)).toContain("emerald");
    expect(readinessScoreTone(60)).toContain("amber");
    expect(readinessScoreTone(30)).toContain("zinc");
  });
});
