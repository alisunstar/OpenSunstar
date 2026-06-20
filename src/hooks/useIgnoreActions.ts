import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ignoreApi, type IgnoreRule } from "@/lib/api/ignore";
import type { AppId } from "@/lib/api";

export function useIgnoreActions() {
  const { t } = useTranslation();
  const [rules, setRules] = useState<IgnoreRule[]>([]);
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const data = await ignoreApi.getAll();
      setRules(data);
    } catch {
      toast.error(t("ignore.loadFailed", { defaultValue: "加载忽略规则失败" }));
    } finally {
      setLoading(false);
    }
  }, [t]);

  const saveRule = useCallback(
    async (rule: IgnoreRule) => {
      try {
        await ignoreApi.upsert(rule);
        await reload();
        toast.success(t("ignore.saveSuccess", { defaultValue: "保存成功" }));
      } catch {
        toast.error(t("ignore.saveFailed", { defaultValue: "保存失败" }));
        throw new Error("save failed");
      }
    },
    [reload, t],
  );

  const deleteRule = useCallback(
    async (id: string) => {
      try {
        await ignoreApi.delete(id);
        await reload();
        toast.success(t("ignore.deleteSuccess", { defaultValue: "删除成功" }));
      } catch {
        toast.error(t("ignore.deleteFailed", { defaultValue: "删除失败" }));
        throw new Error("delete failed");
      }
    },
    [reload, t],
  );

  const toggleApp = useCallback(
    async (ruleId: string, app: AppId, enabled: boolean) => {
      try {
        await ignoreApi.toggleApp(ruleId, app, enabled);
        await reload();
      } catch {
        toast.error(t("ignore.toggleFailed", { defaultValue: "切换同步目标失败" }));
      }
    },
    [reload, t],
  );

  const importGitignore = useCallback(
    async (filePath: string) => {
      try {
        const count = await ignoreApi.importFromGitignore(filePath);
        await reload();
        toast.success(
          t("ignore.importSuccess", {
            count,
            defaultValue: "已导入 {{count}} 条规则",
          }),
        );
      } catch {
        toast.error(t("ignore.importFailed", { defaultValue: "导入失败" }));
      }
    },
    [reload, t],
  );

  const syncRules = useCallback(async () => {
    try {
      await ignoreApi.sync();
      toast.success(t("ignore.syncSuccess", { defaultValue: "已同步到各工具 ignore 文件" }));
    } catch {
      toast.error(t("ignore.syncFailed", { defaultValue: "同步失败" }));
    }
  }, [t]);

  return {
    rules,
    loading,
    reload,
    saveRule,
    deleteRule,
    toggleApp,
    importGitignore,
    syncRules,
  };
}
