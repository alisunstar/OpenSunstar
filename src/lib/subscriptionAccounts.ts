import type {
  SubscriptionAccountHealth,
  SubscriptionAccountView,
  SubscriptionProvider,
} from "@/lib/api/subscription-accounts";
import type { QuotaTier, SubscriptionQuota } from "@/types/subscription";

const PROVIDER_ORDER: SubscriptionProvider[] = ["claude", "codex", "gemini"];

export interface SubscriptionAccountGroup {
  provider: SubscriptionProvider;
  accounts: SubscriptionAccountView[];
}

export function groupSubscriptionAccounts(
  accounts: SubscriptionAccountView[],
): SubscriptionAccountGroup[] {
  return PROVIDER_ORDER.map((provider) => ({
    provider,
    accounts: accounts.filter((account) => account.provider === provider),
  }));
}

/**
 * A default is meaningful only when OpenSunstar manages a choice between two
 * or more identities for the same provider. Local CLI state is observational:
 * it must never be presented as a selectable global default.
 */
export function canShowManagedDefault(
  account: SubscriptionAccountView,
  providerAccounts: SubscriptionAccountView[],
): boolean {
  return (
    account.isDefault &&
    account.source === "managed_oauth" &&
    providerAccounts.filter((item) => item.source === "managed_oauth").length >=
      2
  );
}

export function mostConstrainedTier(
  quota: SubscriptionQuota,
): QuotaTier | null {
  if (!quota.success || quota.tiers.length === 0) return null;
  return quota.tiers.reduce((current, tier) =>
    tier.utilization > current.utilization ? tier : current,
  );
}

export function accountHealthLabel(account: SubscriptionAccountView): string {
  const labels: Record<SubscriptionAccountHealth, string> = {
    healthy: "可用",
    constrained: "额度受限",
    unavailable: "需要重新授权",
    unknown: "额度待确认",
  };
  return labels[account.health];
}
