import { describe, expect, it } from "vitest";
import {
  ASSET_APP_SUPPORT,
  isAssetLinkable,
  summarizeAssetSupport,
} from "./assetAppSupport";
import supportContract from "./assetAppSupport.contract.json";
import type { ProjectAssetType } from "@/types/projectAsset";

const ALL_TYPES: ProjectAssetType[] = [
  "mcp",
  "skill",
  "prompt",
  "command",
  "hook",
  "ignore",
  "permission",
  "subagent",
];

describe("assetAppSupport", () => {
  it("covers all 8 asset types in the matrix", () => {
    for (const type of ALL_TYPES) {
      expect(ASSET_APP_SUPPORT[type]).toBeDefined();
      expect(Object.keys(ASSET_APP_SUPPORT[type]).length).toBeGreaterThan(0);
    }
  });

  it("marks hook as linkable because Claude supports it", () => {
    expect(isAssetLinkable("hook")).toBe(true);
    expect(summarizeAssetSupport("hook").allUnsupported).toBe(false);
  });

  it("marks types with zero supported apps as not linkable", () => {
    // If every app is unsupported, switch should stay disabled
    const fakeAllUnsupported = Object.fromEntries(
      Object.keys(ASSET_APP_SUPPORT.mcp).map((k) => [
        k,
        { status: "unsupported" as const },
      ]),
    );
    const hasSupported = Object.values(fakeAllUnsupported).some(
      (s) => s.status !== "unsupported",
    );
    expect(hasSupported).toBe(false);
  });

  it("command is supported on Codex, Claude, and Gemini", () => {
    expect(isAssetLinkable("command")).toBe(true);
    expect(ASSET_APP_SUPPORT.command.codex.status).toBe("supported");
    expect(ASSET_APP_SUPPORT.command.claude.status).toBe("supported");
  });

  it("exposes every shared-contract asset/app status to the UI", () => {
    for (const type of ALL_TYPES) {
      for (const appId of supportContract.apps) {
        const source = supportContract.assets[type] as {
          supported: string[];
          partial: string[];
        };
        const status = source.supported.includes(appId)
          ? "supported"
          : source.partial.includes(appId)
            ? "partial"
            : "unsupported";
        expect(
          ASSET_APP_SUPPORT[type][
            appId as keyof (typeof ASSET_APP_SUPPORT)[typeof type]
          ].status,
        ).toBe(status);
      }
    }
  });
});
