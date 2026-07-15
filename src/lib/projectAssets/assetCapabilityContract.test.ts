import { describe, expect, it } from "vitest";

import {
  ASSET_CAPABILITY_CONTRACT,
  getAssetCapability,
  getAssetCapabilityEntries,
} from "./assetAppSupport";
import type { ProjectAssetType } from "@/types/projectAsset";

const ASSET_TYPES: ProjectAssetType[] = [
  "mcp",
  "skill",
  "prompt",
  "command",
  "hook",
  "ignore",
  "permission",
  "subagent",
];

describe("asset capability contract", () => {
  it("declares one capability for every asset and target application", () => {
    expect(ASSET_CAPABILITY_CONTRACT.schema_version).toBe(1);

    for (const assetType of ASSET_TYPES) {
      expect(getAssetCapabilityEntries(assetType)).toHaveLength(7);
      const declaration = ASSET_CAPABILITY_CONTRACT.assets[assetType];
      expect(declaration.adapter_id).toMatch(/^project-config-sync:/);
      expect(declaration.fixture_id).toMatch(/^project-config-sync:/);
      for (const [, capability] of getAssetCapabilityEntries(assetType)) {
        if (capability.support !== "unsupported") {
          expect(declaration.adapter_id).toBeTruthy();
          expect(declaration.fixture_id).toBeTruthy();
        }
      }
    }
  });

  it("makes write and verification limits explicit instead of inferring support", () => {
    const claudeMcp = getAssetCapability("mcp", "claude");
    const desktopMcp = getAssetCapability("mcp", "claude-desktop");
    const codexSubagent = getAssetCapability("subagent", "codex");

    expect(claudeMcp.support).toBe("supported");
    expect(claudeMcp.write_mode).not.toBe("none");
    expect(claudeMcp.verify_modes).toContain("config_parse");

    expect(desktopMcp.support).toBe("unsupported");
    expect(desktopMcp.write_mode).toBe("none");
    expect(desktopMcp.verify_modes).toEqual([]);

    expect(codexSubagent.support).toBe("partial");
    expect(codexSubagent.limitations).toContain("global_side_effect");
  });
});
