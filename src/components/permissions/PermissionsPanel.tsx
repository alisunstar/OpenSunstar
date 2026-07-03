import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Shield } from "lucide-react";
import { Button } from "@/components/ui/button";
import { usePermissionActions } from "@/hooks/usePermissionActions";
import { PermissionListItem } from "./PermissionListItem";
import { PermissionFormPanel } from "./PermissionFormPanel";
import { ConfirmDialog } from "../ConfirmDialog";

export interface PermissionsPanelHandle {
  openAdd: () => void;
}

const PermissionsPanel = React.forwardRef<PermissionsPanelHandle, { open: boolean }>(
  ({ open }, ref) => {
    const { t } = useTranslation();
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [confirmId, setConfirmId] = useState<string | null>(null);

    const {
      permissions,
      presets,
      loading,
      reload,
      savePermission,
      deletePermission,
      toggleApp,
      syncPermissions,
      applyPreset,
    } = usePermissionActions();

    useEffect(() => {
      if (open) void reload();
    }, [open, reload]);

    React.useImperativeHandle(ref, () => ({
      openAdd: () => {
        setEditingId(null);
        setIsFormOpen(true);
      },
    }));

    const editingPermission = editingId
      ? permissions.find((p) => p.id === editingId)
      : undefined;

    return (
      <div className="flex flex-col flex-1 min-h-0 px-6">
        <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6 space-y-3">
          <div className="flex items-center justify-between gap-3 flex-wrap">
            <div className="text-sm text-muted-foreground">
              {t("permissions.count", {
                count: permissions.length,
                defaultValue: "共 {{count}} 条权限",
              })}
            </div>
            <Button variant="outline" size="sm" onClick={() => void syncPermissions()}>
              {t("permissions.syncNow", { defaultValue: "同步到各 CLI" })}
            </Button>
          </div>
          {presets.length > 0 && (
            <div className="flex flex-wrap gap-2">
              <span className="text-xs text-muted-foreground self-center">
                {t("permissions.presets", { defaultValue: "预设模板：" })}
              </span>
              {presets.map((preset) => (
                <Button
                  key={preset.id}
                  variant="secondary"
                  size="sm"
                  onClick={() => void applyPreset(preset.id)}
                  title={preset.description}
                >
                  {preset.label}
                </Button>
              ))}
            </div>
          )}
        </div>

        <div className="flex-1 overflow-y-auto pb-16">
          {loading ? (
            <div className="text-center py-12 text-muted-foreground">
              {t("common.loading")}
            </div>
          ) : permissions.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
                <Shield size={24} className="text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground mb-2">
                {t("permissions.empty", { defaultValue: "暂无工具权限规则" })}
              </h3>
              <p className="text-sm text-muted-foreground max-w-md mx-auto">
                {t("permissions.emptyHint", {
                  defaultValue:
                    "管理各 CLI 的工具 allow/deny 规则；通过图标切换同步目标",
                })}
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {permissions.map((permission) => (
                <PermissionListItem
                  key={permission.id}
                  permission={permission}
                  onEdit={(id) => {
                    setEditingId(id);
                    setIsFormOpen(true);
                  }}
                  onDelete={setConfirmId}
                  onToggleApp={(id, app, enabled) =>
                    void toggleApp(id, app, enabled)
                  }
                />
              ))}
            </div>
          )}
        </div>

        {isFormOpen && (
          <PermissionFormPanel
            editingId={editingId || undefined}
            initialData={editingPermission}
            onSave={savePermission}
            onClose={() => setIsFormOpen(false)}
          />
        )}

        {confirmId && (
          <ConfirmDialog
            isOpen
            title={t("permissions.confirm.deleteTitle", { defaultValue: "删除权限" })}
            message={t("permissions.confirm.deleteMessage", {
              defaultValue: "确定删除此权限规则？settings.json 将同步更新。",
            })}
            onConfirm={async () => {
              await deletePermission(confirmId);
              setConfirmId(null);
            }}
            onCancel={() => setConfirmId(null)}
          />
        )}
      </div>
    );
  },
);

PermissionsPanel.displayName = "PermissionsPanel";
export default PermissionsPanel;
