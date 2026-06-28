import { describe, expect, it } from "vitest";
import { validateCuratedPresetNames } from "@/lib/quickStart/resolvePresets";

describe("quickStartCurated", () => {
  it("all curated preset names resolve in their app libraries", () => {
    const errors = validateCuratedPresetNames();
    expect(errors).toEqual([]);
  });
});
