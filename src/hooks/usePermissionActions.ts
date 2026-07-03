import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  permissionsApi,
  type ToolPermission,
  type PermissionPreset,
} from "@/lib/api/permissions";
import type { AppId } from "@/lib/api";

export function usePermissionActions() {
  const { t } = useTranslation();
  const [permissions, setPermissions] = useState<ToolPermission[]>([]);
  const [presets, setPresets] = useState<PermissionPreset[]>([]);
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const [data, presetData] = await Promise.all([
        permissionsApi.getAll(),
        permissionsApi.getPresets(),
      ]);
      setPermissions(data);
      setPresets(presetData);
    } catch {
      toast.error(t("permissions.loadFailed", { defaultValue: "加载权限失败" }));
    } finally {
      setLoading(false);
    }
  }, [t]);

  const savePermission = useCallback(
    async (permission: ToolPermission) => {
      try {
        await permissionsApi.upsert(permission);
        await reload();
        toast.success(t("permissions.saveSuccess", { defaultValue: "保存成功" }));
      } catch {
        toast.error(t("permissions.saveFailed", { defaultValue: "保存失败" }));
        throw new Error("save failed");
      }
    },
    [reload, t],
  );

  const deletePermission = useCallback(
    async (id: string) => {
      try {
        await permissionsApi.delete(id);
        await reload();
        toast.success(t("permissions.deleteSuccess", { defaultValue: "删除成功" }));
      } catch {
        toast.error(t("permissions.deleteFailed", { defaultValue: "删除失败" }));
        throw new Error("delete failed");
      }
    },
    [reload, t],
  );

  const toggleApp = useCallback(
    async (permId: string, app: AppId, enabled: boolean) => {
      try {
        await permissionsApi.toggleApp(permId, app, enabled);
        await reload();
      } catch {
        toast.error(
          t("permissions.toggleFailed", { defaultValue: "切换同步目标失败" }),
        );
      }
    },
    [reload, t],
  );

  const syncPermissions = useCallback(async () => {
    try {
      await permissionsApi.sync();
      toast.success(
        t("permissions.syncSuccess", {
          defaultValue: "已同步到各 CLI 配置文件",
        }),
      );
    } catch {
      toast.error(t("permissions.syncFailed", { defaultValue: "同步失败" }));
    }
  }, [t]);

  const applyPreset = useCallback(
    async (presetId: string) => {
      try {
        await permissionsApi.applyPreset(presetId);
        await reload();
        toast.success(t("permissions.presetApplied", { defaultValue: "预设已应用" }));
      } catch {
        toast.error(t("permissions.presetFailed", { defaultValue: "应用预设失败" }));
      }
    },
    [reload, t],
  );

  return {
    permissions,
    presets,
    loading,
    reload,
    savePermission,
    deletePermission,
    toggleApp,
    syncPermissions,
    applyPreset,
  };
}
