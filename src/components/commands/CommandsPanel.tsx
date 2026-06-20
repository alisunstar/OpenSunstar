import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "lucide-react";
import { useCommandActions } from "@/hooks/useCommandActions";
import { CommandListItem } from "./CommandListItem";
import { CommandFormPanel } from "./CommandFormPanel";
import { ConfirmDialog } from "../ConfirmDialog";

export interface CommandsPanelHandle {
  openAdd: () => void;
}

const CommandsPanel = React.forwardRef<CommandsPanelHandle, { open: boolean }>(
  ({ open }, ref) => {
    const { t } = useTranslation();
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [confirmDialog, setConfirmDialog] = useState<{
      isOpen: boolean;
      id: string;
      name: string;
    } | null>(null);

    const {
      commands,
      loading,
      reload,
      saveCommand,
      deleteCommand,
      toggleApp,
    } = useCommandActions();

    useEffect(() => {
      if (open) void reload();
    }, [open, reload]);

    React.useImperativeHandle(ref, () => ({
      openAdd: () => {
        setEditingId(null);
        setIsFormOpen(true);
      },
    }));

    const entries = useMemo(() => Object.values(commands), [commands]);

    return (
      <div className="flex flex-col flex-1 min-h-0 px-6">
        <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6">
          <div className="text-sm text-muted-foreground">
            {t("commands.count", {
              count: entries.length,
              defaultValue: "共 {{count}} 个命令",
            })}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto pb-16">
          {loading ? (
            <div className="text-center py-12 text-muted-foreground">
              {t("common.loading")}
            </div>
          ) : entries.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
                <Terminal size={24} className="text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground mb-2">
                {t("commands.empty", { defaultValue: "暂无自定义命令" })}
              </h3>
            </div>
          ) : (
            <div className="space-y-3">
              {entries.map((command) => (
                <CommandListItem
                  key={command.id}
                  command={command}
                  onEdit={(id) => {
                    setEditingId(id);
                    setIsFormOpen(true);
                  }}
                  onDelete={(id) =>
                    setConfirmDialog({
                      isOpen: true,
                      id,
                      name: command.name,
                    })
                  }
                  onToggleApp={toggleApp}
                />
              ))}
            </div>
          )}
        </div>

        {isFormOpen && (
          <CommandFormPanel
            editingId={editingId || undefined}
            initialData={editingId ? commands[editingId] : undefined}
            onSave={saveCommand}
            onClose={() => setIsFormOpen(false)}
          />
        )}

        {confirmDialog && (
          <ConfirmDialog
            isOpen={confirmDialog.isOpen}
            title={t("commands.confirm.deleteTitle", {
              defaultValue: "删除命令",
            })}
            message={t("commands.confirm.deleteMessage", {
              name: confirmDialog.name,
              defaultValue:
                '确定删除命令 "{{name}}"？已同步的文件将被清理。',
            })}
            onConfirm={async () => {
              try {
                await deleteCommand(confirmDialog.id);
                setConfirmDialog(null);
              } catch {
                // handled in hook
              }
            }}
            onCancel={() => setConfirmDialog(null)}
          />
        )}
      </div>
    );
  },
);

CommandsPanel.displayName = "CommandsPanel";

export default CommandsPanel;
