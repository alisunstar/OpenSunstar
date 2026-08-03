import { describe, expect, it } from "vitest";
import {
  buildCodexOauthIntent,
  buildLocalCliAuthIntent,
  buildSubscriptionAccountsIntent,
  buildXaiOauthIntent,
} from "@/lib/settingsNavigation";

describe("subscription account navigation", () => {
  it("opens the independent authentication center instead of provider settings", () => {
    expect(buildSubscriptionAccountsIntent()).toEqual({
      tab: "auth",
      targetId: "subscription-accounts",
    });
  });

  it("targets the exact local CLI and Codex account sections", () => {
    expect(buildLocalCliAuthIntent()).toEqual({
      tab: "auth",
      targetId: "local-cli-auth",
    });
    expect(buildCodexOauthIntent()).toEqual({
      tab: "auth",
      targetId: "codex-oauth",
    });
    expect(buildXaiOauthIntent()).toEqual({
      tab: "auth",
      targetId: "xai-oauth",
    });
  });
});
