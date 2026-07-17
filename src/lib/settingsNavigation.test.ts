import { describe, expect, it } from "vitest";
import { buildSubscriptionAccountsIntent } from "@/lib/settingsNavigation";

describe("subscription account navigation", () => {
  it("opens the independent authentication center instead of provider settings", () => {
    expect(buildSubscriptionAccountsIntent()).toEqual({ tab: "auth" });
  });
});
