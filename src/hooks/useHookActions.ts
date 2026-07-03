import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { hooksApi, type Hook } from "@/lib/api/hooks";
import type { AppId } from "@/lib/api";

export function useHookActions() {
  const { t } = useTranslation();
  const [hooks, setHooks] = useState<Hook[]>([]);
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const data = await hooksApi.getAll();
      setHooks(data);
    } catch {
      toast.error(t("hooks.loadFailed", { defaultValue: "加载钩子失败" }));
    } finally {
      setLoading(false);
    }
  }, [t]);

  const saveHook = useCallback(
    async (hook: Hook) => {
      try {
        await hooksApi.upsert(hook);
        await reload();
        toast.success(t("hooks.saveSuccess", { defaultValue: "保存成功" }));
      } catch {
        toast.error(t("hooks.saveFailed", { defaultValue: "保存失败" }));
        throw new Error("save failed");
      }
    },
    [reload, t],
  );

  const deleteHook = useCallback(
    async (id: string) => {
      try {
        await hooksApi.delete(id);
        await reload();
        toast.success(t("hooks.deleteSuccess", { defaultValue: "删除成功" }));
      } catch {
        toast.error(t("hooks.deleteFailed", { defaultValue: "删除失败" }));
        throw new Error("delete failed");
      }
    },
    [reload, t],
  );

  const toggleApp = useCallback(
    async (hookId: string, app: AppId, enabled: boolean) => {
      try {
        await hooksApi.toggleApp(hookId, app, enabled);
        await reload();
      } catch {
        toast.error(t("hooks.toggleFailed", { defaultValue: "切换同步目标失败" }));
      }
    },
    [reload, t],
  );

  const syncHooks = useCallback(async () => {
    try {
      await hooksApi.sync();
      toast.success(
        t("hooks.syncSuccess", { defaultValue: "已同步到各 CLI 配置文件" }),
      );
    } catch {
      toast.error(t("hooks.syncFailed", { defaultValue: "同步失败" }));
    }
  }, [t]);

  return { hooks, loading, reload, saveHook, deleteHook, toggleApp, syncHooks };
}
