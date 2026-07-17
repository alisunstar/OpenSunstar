import { invoke } from "@tauri-apps/api/core";
import type { SubscriptionQuota } from "@/types/subscription";

export type SubscriptionProvider = "claude" | "codex" | "gemini";
export type SubscriptionAccountHealth =
  | "healthy"
  | "constrained"
  | "unavailable"
  | "unknown";

/** A credential reference and its current subscription observation. No secret is returned. */
export interface SubscriptionAccountView {
  id: string;
  provider: SubscriptionProvider;
  displayName: string;
  source: "managed_oauth" | "local_cli";
  isDefault: boolean;
  quota: SubscriptionQuota;
  health: SubscriptionAccountHealth;
}

export const subscriptionAccountsApi = {
  list: (): Promise<SubscriptionAccountView[]> =>
    invoke("subscription_list_accounts"),
};
