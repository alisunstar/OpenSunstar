import { useQuery } from "@tanstack/react-query";
import { Loader2, RefreshCw, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ClaudeIcon, CodexIcon, GeminiIcon } from "@/components/BrandIcons";
import { subscriptionAccountsApi } from "@/lib/api/subscription-accounts";
import {
  accountHealthLabel,
  canShowManagedDefault,
  groupSubscriptionAccounts,
  mostConstrainedTier,
} from "@/lib/subscriptionAccounts";
import {
  countdownStr,
  TIER_I18N_KEYS,
} from "@/components/SubscriptionQuotaFooter";

const HEALTH_CLASS = {
  healthy:
    "border-emerald-500/25 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  constrained:
    "border-amber-500/25 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  unavailable: "border-destructive/25 bg-destructive/10 text-destructive",
  unknown: "border-border bg-muted/40 text-muted-foreground",
} as const;

function ProviderIcon({
  provider,
}: {
  provider: "claude" | "codex" | "gemini";
}) {
  switch (provider) {
    case "claude":
      return <ClaudeIcon size={20} />;
    case "codex":
      return <CodexIcon size={20} />;
    case "gemini":
      return <GeminiIcon size={20} />;
  }
}

function providerLabel(provider: "claude" | "codex" | "gemini"): string {
  return provider === "codex"
    ? "ChatGPT / Codex"
    : provider === "claude"
      ? "Claude"
      : "Gemini";
}

/**
 * Subscription identity control plane. This intentionally has no provider or
 * project selector: it monitors identities and exposes a default only when a
 * provider has multiple OpenSunstar-managed accounts.
 */
export function SubscriptionAccountsPanel() {
  const { t } = useTranslation();
  const query = useQuery({
    queryKey: ["subscription-accounts"],
    queryFn: subscriptionAccountsApi.list,
    staleTime: 5 * 60 * 1000,
    retry: 1,
  });
  const groups = groupSubscriptionAccounts(query.data ?? []);

  return (
    <section
      id="subscription-accounts"
      className="rounded-xl border border-border/60 bg-card/60 p-6"
    >
      <div className="mb-5 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-5 w-5 text-primary" />
            <h4 className="font-medium">
              {t("settings.authCenter.subscriptionAccountsTitle", {
                defaultValue: "订阅账号",
              })}
            </h4>
          </div>
          <p className="text-sm text-muted-foreground">
            {t("settings.authCenter.subscriptionAccountsDescription", {
              defaultValue:
                "查看官方订阅的额度、重置时间与健康状态。仅托管的多账号支持默认标记；这里不绑定项目或第三方 API Key。",
            })}
          </p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 gap-1.5 self-start"
          disabled={query.isFetching}
          onClick={() => void query.refetch()}
        >
          {query.isFetching ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RefreshCw className="h-3.5 w-3.5" />
          )}
          {t("common.refresh", { defaultValue: "刷新" })}
        </Button>
      </div>

      {query.isLoading ? (
        <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border/60 py-8 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          {t("settings.authCenter.subscriptionAccountsLoading", {
            defaultValue: "正在并发读取订阅账号状态…",
          })}
        </div>
      ) : query.isError ? (
        <div className="rounded-lg border border-destructive/25 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {t("settings.authCenter.subscriptionAccountsFailed", {
            defaultValue: "无法读取订阅账号状态，请刷新后重试。",
          })}
        </div>
      ) : (
        <div className="grid gap-3 xl:grid-cols-3">
          {groups.map((group) => (
            <article
              key={group.provider}
              className="min-w-0 rounded-lg border border-border/50 bg-background/50 p-4"
            >
              <div className="mb-3 flex items-center gap-2">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted">
                  <ProviderIcon provider={group.provider} />
                </div>
                <div>
                  <h5 className="text-sm font-medium">
                    {providerLabel(group.provider)}
                  </h5>
                  <p className="text-xs text-muted-foreground">
                    {group.provider === "codex"
                      ? t("settings.authCenter.managedOauth", {
                          defaultValue: "OpenSunstar 托管 OAuth",
                        })
                      : t("settings.authCenter.localCliProfile", {
                          defaultValue: "本机 CLI 当前会话",
                        })}
                  </p>
                </div>
              </div>

              {group.accounts.length === 0 ? (
                <p className="rounded-md bg-muted/50 px-3 py-2 text-xs text-muted-foreground">
                  {t("settings.authCenter.noSubscriptionAccount", {
                    defaultValue: "尚未发现可用订阅账号",
                  })}
                </p>
              ) : (
                <div className="space-y-2">
                  {group.accounts.map((account) => {
                    const tier = mostConstrainedTier(account.quota);
                    const reset = tier ? countdownStr(tier.resetsAt) : null;
                    const source =
                      account.source === "managed_oauth"
                        ? t("settings.authCenter.sourceManaged", {
                            defaultValue: "托管 OAuth",
                          })
                        : t("settings.authCenter.sourceLocal", {
                            defaultValue: "本机 CLI 当前会话",
                          });
                    return (
                      <div
                        key={account.id}
                        className="rounded-md border border-border/50 bg-muted/20 px-3 py-2.5"
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-1.5">
                              <span className="truncate text-sm font-medium">
                                {account.displayName}
                              </span>
                              {canShowManagedDefault(
                                account,
                                group.accounts,
                              ) && (
                                <Badge
                                  variant="secondary"
                                  className="h-5 px-1.5 text-[10px]"
                                >
                                  {t("settings.authCenter.defaultAccount", {
                                    defaultValue: "默认",
                                  })}
                                </Badge>
                              )}
                            </div>
                            <p className="mt-0.5 text-xs text-muted-foreground">
                              {source}
                            </p>
                          </div>
                          <Badge
                            variant="outline"
                            className={`shrink-0 text-[10px] ${HEALTH_CLASS[account.health]}`}
                          >
                            {accountHealthLabel(account)}
                          </Badge>
                        </div>
                        {tier && (
                          <p className="mt-2 text-xs text-muted-foreground">
                            {t(TIER_I18N_KEYS[tier.name] ?? tier.name, {
                              defaultValue: tier.name,
                            })}
                            <span className="mx-1.5 text-foreground/40">·</span>
                            <span className="font-medium text-foreground">
                              {Math.round(tier.utilization)}%
                            </span>
                            {reset && (
                              <>
                                <span className="mx-1.5 text-foreground/40">
                                  ·
                                </span>
                                {t("subscription.resetsIn", { time: reset })}
                              </>
                            )}
                          </p>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
