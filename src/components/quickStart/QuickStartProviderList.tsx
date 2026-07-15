import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import { ProviderList } from "@/components/providers/ProviderList";
import { useProviderActions } from "@/hooks/useProviderActions";
import { useProvidersQuery } from "@/lib/query/queries";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type { Provider, UsageScript } from "@/types";
import { QuickStartUsageDialog } from "./QuickStartUsageDialog";

interface QuickStartProviderListProps {
  appId: QuickStartAppId;
  onAddProvider?: () => void;
}

export function QuickStartProviderList({
  appId,
  onAddProvider,
}: QuickStartProviderListProps) {
  const { t } = useTranslation();
  const { data } = useProvidersQuery(appId);
  const { takeoverStatus, isRunning } = useProxyStatus();
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [deletingProvider, setDeletingProvider] = useState<Provider | null>(
    null,
  );
  const [usageProvider, setUsageProvider] = useState<Provider | null>(null);
  const [isSavingUsage, setIsSavingUsage] = useState(false);

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

  const {
    addProvider,
    updateProvider,
    switchProvider,
    deleteProvider,
    saveUsageScript,
  } = useProviderActions(appId, isRunning, takeoverActive);

  const handleDuplicate = async (provider: Provider) => {
    const {
      id: _id,
      createdAt: _createdAt,
      sortIndex: _sortIndex,
      ...copy
    } = provider;
    try {
      await addProvider({
        ...copy,
        name: t("provider.copyName", {
          defaultValue: "{{name}} 副本",
          name: provider.name,
        }),
        meta: provider.meta ? { ...provider.meta } : undefined,
      });
    } catch {
      // The provider mutation surfaces the actionable error to the user.
    }
  };

  const handleDeleteRequest = (provider: Provider) => {
    if (provider.id === currentId) {
      toast.error(
        t("quickStart.cannotDeleteCurrent", {
          defaultValue: "当前使用中的供应商不能删除，请先切换到其他供应商。",
        }),
      );
      return;
    }
    setDeletingProvider(provider);
  };

  const handleSaveUsage = async (script: UsageScript) => {
    if (!usageProvider) return;
    setIsSavingUsage(true);
    try {
      await saveUsageScript(usageProvider, script);
      setUsageProvider(null);
    } finally {
      setIsSavingUsage(false);
    }
  };

  return (
    <div className="w-full max-w-4xl space-y-3 text-left">
      <div className="flex items-end justify-between gap-4">
        <div>
          <h3 className="text-sm font-medium">
            {t("quickStart.myProviders", { defaultValue: "我的供应商" })}
            {entries.length > 0 ? `（${entries.length}）` : ""}
          </h3>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("quickStart.myProvidersHint", {
              defaultValue:
                "拖动左侧手柄可排序；悬停供应商可进行编辑、复制、检测、用量配置或删除。",
            })}
          </p>
        </div>
        {takeoverActive && (
          <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-700 dark:text-emerald-300">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse" />
            {t("quickStart.badge.proxy", { defaultValue: "本地路由" })}
          </span>
        )}
      </div>

      <ProviderList
        providers={providers}
        currentProviderId={currentId}
        appId={appId}
        onSwitch={(provider) => void switchProvider(provider)}
        onEdit={setEditingProvider}
        onDelete={handleDeleteRequest}
        onDuplicate={(provider) => void handleDuplicate(provider)}
        onConfigureUsage={setUsageProvider}
        onOpenWebsite={(url) =>
          window.open(url, "_blank", "noopener,noreferrer")
        }
        onCreate={onAddProvider}
        isProxyRunning={isRunning}
        isProxyTakeover={takeoverActive}
      />

      <EditProviderDialog
        open={editingProvider !== null}
        provider={editingProvider}
        onOpenChange={(open) => {
          if (!open) setEditingProvider(null);
        }}
        onSubmit={({ provider, originalId }) =>
          updateProvider(provider, originalId)
        }
        appId={appId}
        isProxyTakeover={takeoverActive}
      />

      <ConfirmDialog
        isOpen={deletingProvider !== null}
        title={t("provider.deleteProvider", { defaultValue: "删除供应商" })}
        message={t("quickStart.deleteProviderMessage", {
          defaultValue:
            "将删除 {{name}} 的已保存配置。此操作不会删除供应商平台上的账户或 API Key。",
          name: deletingProvider?.name ?? "",
        })}
        confirmText={t("common.delete", { defaultValue: "删除" })}
        onCancel={() => setDeletingProvider(null)}
        onConfirm={() => {
          if (!deletingProvider) return;
          void deleteProvider(deletingProvider.id)
            .then(() => setDeletingProvider(null))
            .catch(() => undefined);
        }}
      />

      <QuickStartUsageDialog
        open={usageProvider !== null}
        provider={usageProvider}
        isSaving={isSavingUsage}
        onOpenChange={(open) => {
          if (!open) setUsageProvider(null);
        }}
        onSave={handleSaveUsage}
      />
    </div>
  );
}
