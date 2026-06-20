import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Webhook } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useHookActions } from "@/hooks/useHookActions";
import { HookListItem } from "./HookListItem";
import { HookFormPanel } from "./HookFormPanel";
import { ConfirmDialog } from "../ConfirmDialog";

export interface HooksPanelHandle {
  openAdd: () => void;
}

const HooksPanel = React.forwardRef<HooksPanelHandle, { open: boolean }>(
  ({ open }, ref) => {
    const { t } = useTranslation();
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [confirmId, setConfirmId] = useState<string | null>(null);

    const { hooks, loading, reload, saveHook, deleteHook, syncHooks } =
      useHookActions();

    useEffect(() => {
      if (open) void reload();
    }, [open, reload]);

    React.useImperativeHandle(ref, () => ({
      openAdd: () => {
        setEditingId(null);
        setIsFormOpen(true);
      },
    }));

    const editingHook = editingId
      ? hooks.find((h) => h.id === editingId)
      : undefined;

    return (
      <div className="flex flex-col flex-1 min-h-0 px-6">
        <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6 flex items-center justify-between gap-3">
          <div className="text-sm text-muted-foreground">
            {t("hooks.count", {
              count: hooks.length,
              defaultValue: "共 {{count}} 个钩子",
            })}
          </div>
          <Button variant="outline" size="sm" onClick={() => void syncHooks()}>
            {t("hooks.syncNow", { defaultValue: "同步到 Claude" })}
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto pb-16">
          {loading ? (
            <div className="text-center py-12 text-muted-foreground">
              {t("common.loading")}
            </div>
          ) : hooks.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
                <Webhook size={24} className="text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground mb-2">
                {t("hooks.empty", { defaultValue: "暂无生命周期钩子" })}
              </h3>
            </div>
          ) : (
            <div className="space-y-3">
              {hooks.map((hook) => (
                <HookListItem
                  key={hook.id}
                  hook={hook}
                  onEdit={(id) => {
                    setEditingId(id);
                    setIsFormOpen(true);
                  }}
                  onDelete={setConfirmId}
                />
              ))}
            </div>
          )}
        </div>

        {isFormOpen && (
          <HookFormPanel
            editingId={editingId || undefined}
            initialData={editingHook}
            onSave={saveHook}
            onClose={() => setIsFormOpen(false)}
          />
        )}

        {confirmId && (
          <ConfirmDialog
            isOpen
            title={t("hooks.confirm.deleteTitle", { defaultValue: "删除钩子" })}
            message={t("hooks.confirm.deleteMessage", {
              defaultValue: "确定删除此钩子？settings.json 将同步更新。",
            })}
            onConfirm={async () => {
              try {
                await deleteHook(confirmId);
                setConfirmId(null);
              } catch {
                // handled in hook
              }
            }}
            onCancel={() => setConfirmId(null)}
          />
        )}
      </div>
    );
  },
);

HooksPanel.displayName = "HooksPanel";

export default HooksPanel;
