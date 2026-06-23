import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Settings2, Sparkles } from "lucide-react";
import { AppSwitcher } from "@/components/AppSwitcher";
import { ProviderList } from "@/components/providers/ProviderList";
import { AddProviderDialog } from "@/components/providers/AddProviderDialog";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import { useProviderActions } from "@/hooks/useProviderActions";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import { useProvidersQuery } from "@/lib/query/queries";
import { Button } from "@/components/ui/button";
import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";

interface ExpertProviderPanelProps {
  onSwitchToSimple?: () => void;
  onOpenSettings?: () => void;
}

export function ExpertProviderPanel({
  onSwitchToSimple,
  onOpenSettings,
}: ExpertProviderPanelProps) {
  const { t } = useTranslation();
  const [activeApp, setActiveApp] = useState<AppId>("claude");
  const [addOpen, setAddOpen] = useState(false);
  const [editProvider, setEditProvider] = useState<Provider | null>(null);

  const { isRunning, takeoverStatus } = useProxyStatus();
  const isProxyTakeover = Boolean(
    takeoverStatus?.[activeApp as keyof typeof takeoverStatus],
  );

  const { data, isPending, isFetching } = useProvidersQuery(activeApp, {
    isProxyRunning: isRunning,
  });

  const {
    addProvider,
    updateProvider,
    switchProvider,
    deleteProvider,
    setAsDefaultModel,
  } = useProviderActions(activeApp, isRunning, isProxyTakeover);

  const providers = data?.providers ?? {};
  const currentProviderId = data?.currentProviderId ?? "";

  return (
    <div className="space-y-4 max-w-4xl">
      <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-4 text-sm space-y-3">
        <p className="font-medium text-amber-800 dark:text-amber-200">
          {t("simpleConnect.expertNoticeTitle", {
            defaultValue: "Expert Provider（精简保留）",
          })}
        </p>
        <p className="text-muted-foreground">
          {t("simpleConnect.expertNoticeBody", {
            defaultValue:
              "故障队列、代理接管、Universal 等高级能力仍在此管理。默认推荐先用「快速接入」。",
          })}
        </p>
        <div className="flex flex-wrap gap-2 pt-1">
          {onSwitchToSimple && (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="gap-1.5"
              onClick={onSwitchToSimple}
            >
              <Sparkles className="h-3.5 w-3.5" />
              {t("simpleConnect.backToSimple", {
                defaultValue: "返回快速接入",
              })}
            </Button>
          )}
          {onOpenSettings && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="gap-1.5"
              onClick={onOpenSettings}
            >
              <Settings2 className="h-3.5 w-3.5" />
              {t("simpleConnect.openProxySettings", {
                defaultValue: "代理与故障队列设置",
              })}
            </Button>
          )}
        </div>
      </div>

      <div className="flex items-center justify-between gap-3 flex-wrap">
        <AppSwitcher activeApp={activeApp} onSwitch={setActiveApp} compact />
        {isRunning && (
          <span className="text-xs text-emerald-600 dark:text-emerald-400">
            {t("simpleConnect.proxyRunning", {
              defaultValue: "主代理运行中",
            })}
          </span>
        )}
      </div>

      <ProviderList
        providers={providers}
        currentProviderId={currentProviderId}
        appId={activeApp}
        isLoading={isPending && !data}
        isRefreshing={isFetching && Boolean(data)}
        isProxyRunning={isRunning}
        isProxyTakeover={isProxyTakeover}
        onSwitch={switchProvider}
        onEdit={setEditProvider}
        onDelete={(p) => void deleteProvider(p.id)}
        onDuplicate={(_p) => undefined}
        onOpenWebsite={(url) => window.open(url, "_blank")}
        onCreate={() => setAddOpen(true)}
        onSetAsDefault={setAsDefaultModel}
        compactExpert
      />

      <AddProviderDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        appId={activeApp}
        onSubmit={addProvider}
      />

      {editProvider && (
        <EditProviderDialog
          open={Boolean(editProvider)}
          onOpenChange={(open) => !open && setEditProvider(null)}
          provider={editProvider}
          appId={activeApp}
          onSubmit={async (payload) => {
            await updateProvider(payload.provider, payload.originalId);
          }}
        />
      )}
    </div>
  );
}
