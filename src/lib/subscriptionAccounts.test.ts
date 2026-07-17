import { describe, expect, it } from "vitest";
import type { SubscriptionAccountView } from "@/lib/api/subscription-accounts";
import {
  accountHealthLabel,
  canShowManagedDefault,
  groupSubscriptionAccounts,
  mostConstrainedTier,
} from "@/lib/subscriptionAccounts";

function account(
  id: string,
  provider: SubscriptionAccountView["provider"],
  health: SubscriptionAccountView["health"],
  utilization = 15,
): SubscriptionAccountView {
  return {
    id,
    provider,
    displayName: id,
    source: provider === "codex" ? "managed_oauth" : "local_cli",
    isDefault: false,
    health,
    quota: {
      tool: provider,
      credentialStatus: "valid",
      credentialMessage: null,
      success: health !== "unavailable",
      tiers:
        health === "unknown"
          ? []
          : [
              {
                name: "five_hour",
                utilization,
                resetsAt: "2030-01-01T00:00:00Z",
              },
            ],
      extraUsage: null,
      error: null,
      queriedAt: 1,
    },
  };
}

describe("subscription account overview", () => {
  it("groups accounts into a stable Claude, Codex, Gemini order", () => {
    const groups = groupSubscriptionAccounts([
      account("gemini", "gemini", "healthy"),
      account("codex-b", "codex", "healthy"),
      account("claude", "claude", "healthy"),
      account("codex-a", "codex", "constrained", 96),
    ]);

    expect(groups.map((group) => group.provider)).toEqual([
      "claude",
      "codex",
      "gemini",
    ]);
    expect(groups[1].accounts.map((item) => item.id)).toEqual([
      "codex-b",
      "codex-a",
    ]);
  });

  it("uses the highest-utilization quota tier as the account constraint", () => {
    const value = account("codex", "codex", "constrained", 15);
    value.quota.tiers.push({
      name: "seven_day",
      utilization: 82,
      resetsAt: "2030-01-02T00:00:00Z",
    });

    expect(mostConstrainedTier(value.quota)?.name).toBe("seven_day");
    expect(accountHealthLabel(value)).toBe("额度受限");
  });

  it("does not present unavailable credentials as a usable account", () => {
    const value = account("claude", "claude", "unavailable");
    expect(mostConstrainedTier(value.quota)).toBeNull();
    expect(accountHealthLabel(value)).toBe("需要重新授权");
  });

  it("only exposes a default badge for a managed provider with multiple accounts", () => {
    const localClaude = account("claude", "claude", "healthy");
    localClaude.isDefault = true;
    const onlyCodex = account("codex-a", "codex", "healthy");
    onlyCodex.isDefault = true;
    const defaultCodex = account("codex-b", "codex", "healthy");
    defaultCodex.isDefault = true;
    const backupCodex = account("codex-c", "codex", "healthy");

    expect(canShowManagedDefault(localClaude, [localClaude])).toBe(false);
    expect(canShowManagedDefault(onlyCodex, [onlyCodex])).toBe(false);
    expect(
      canShowManagedDefault(defaultCodex, [defaultCodex, backupCodex]),
    ).toBe(true);
  });
});
