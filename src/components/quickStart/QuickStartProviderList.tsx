import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ProviderIcon } from "@/components/ProviderIcon";
import { useSwitchProviderMutation } from "@/lib/query/mutations";
import { useProvidersQuery } from "@/lib/query/queries";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import { cn } from "@/lib/utils";

interface QuickStartProviderListProps {
  appId: QuickStartAppId;
  onOpenManage?: () => void;
}

export function QuickStartProviderList({
  appId,
  onOpenManage,
}: QuickStartProviderListProps) {
  const { t } = useTranslation();
  const { data } = useProvidersQuery(appId);
  const switchMutation = useSwitchProviderMutation(appId);
  const { takeoverStatus, isRunning } = useProxyStatus();

  const providers = data?.providers ?? {};
  const currentId = data?.currentProviderId ?? "";
  const entries = Object.values(providers);

  const takeoverActive =
    appId === "claude"
      ? takeoverStatus?.claude
      : appId === "codex"
        ? takeoverStatus?.codex
        : appId === "gemini"
          ? takeoverStatus?.gemini
          : isRunning;

  if (entries.length === 0) {
    return null;
  }

  return (
    <div className="w-full max-w-lg space-y-2 text-left">
      <h3 className="text-sm font-medium">
        {t("quickStart.myProviders", {
          defaultValue: "我的供应商",
        })}
      </h3>
      <ul className="space-y-2">
        {entries.map((provider) => {
          const isCurrent = provider.id === currentId;
          return (
            <li
              key={provider.id}
              className={cn(
                "flex items-center gap-3 rounded-lg border border-border bg-card p-3",
                isCurrent && "border-primary/40",
              )}
            >
              <ProviderIcon
                icon={provider.icon}
                name={provider.name}
                color={provider.iconColor}
                size={32}
              />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{provider.name}</p>
                <div className="flex flex-wrap gap-1.5 pt-0.5">
                  {isCurrent && (
                    <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] text-primary">
                      {t("quickStart.badge.current", { defaultValue: "当前" })}
                    </span>
                  )}
                  {takeoverActive && (
                    <span className="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[10px] text-emerald-600 dark:text-emerald-400">
                      {t("quickStart.badge.proxy", {
                        defaultValue: "本地路由",
                      })}
                    </span>
                  )}
                </div>
              </div>
              {!isCurrent && (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={switchMutation.isPending}
                  onClick={() => switchMutation.mutate(provider.id)}
                >
                  {switchMutation.isPending ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    t("quickStart.switch", { defaultValue: "切换" })
                  )}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
      {onOpenManage && (
        <Button
          type="button"
          variant="link"
          size="sm"
          className="h-auto px-0 text-xs"
          onClick={onOpenManage}
        >
          {t("quickStart.openManage", { defaultValue: "管理全部供应商 →" })}
        </Button>
      )}
    </div>
  );
}
