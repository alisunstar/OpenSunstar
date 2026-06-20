import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { EyeOff, FileInput } from "lucide-react";
import { Button } from "@/components/ui/button";
import { settingsApi } from "@/lib/api/settings";
import { useIgnoreActions } from "@/hooks/useIgnoreActions";
import { IgnoreListItem } from "./IgnoreListItem";
import { IgnoreFormPanel } from "./IgnoreFormPanel";
import { ConfirmDialog } from "../ConfirmDialog";

export interface IgnorePanelHandle {
  openAdd: () => void;
}

const IgnorePanel = React.forwardRef<IgnorePanelHandle, { open: boolean }>(
  ({ open }, ref) => {
    const { t } = useTranslation();
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [confirmId, setConfirmId] = useState<string | null>(null);

    const {
      rules,
      loading,
      reload,
      saveRule,
      deleteRule,
      toggleApp,
      importGitignore,
      syncRules,
    } = useIgnoreActions();

    useEffect(() => {
      if (open) void reload();
    }, [open, reload]);

    React.useImperativeHandle(ref, () => ({
      openAdd: () => {
        setEditingId(null);
        setIsFormOpen(true);
      },
    }));

    const editingRule = editingId
      ? rules.find((r) => r.id === editingId)
      : undefined;

    const handleImport = async () => {
      const path = await settingsApi.openFileDialog();
      if (path) await importGitignore(path);
    };

    return (
      <div className="flex flex-col flex-1 min-h-0 px-6">
        <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6 flex items-center justify-between gap-3 flex-wrap">
          <div className="text-sm text-muted-foreground">
            {t("ignore.count", {
              count: rules.length,
              defaultValue: "共 {{count}} 条规则",
            })}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => void handleImport()}>
              <FileInput className="w-4 h-4 mr-1" />
              {t("ignore.importGitignore", { defaultValue: "从 .gitignore 导入" })}
            </Button>
            <Button variant="outline" size="sm" onClick={() => void syncRules()}>
              {t("ignore.syncNow", { defaultValue: "同步到工具" })}
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto pb-16">
          {loading ? (
            <div className="text-center py-12 text-muted-foreground">
              {t("common.loading")}
            </div>
          ) : rules.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
                <EyeOff size={24} className="text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground mb-2">
                {t("ignore.empty", { defaultValue: "暂无忽略规则" })}
              </h3>
              <p className="text-sm text-muted-foreground max-w-md mx-auto">
                {t("ignore.emptyHint", {
                  defaultValue:
                    "添加 glob 规则后，将同步到 .claudeignore、.codexignore 等文件",
                })}
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {rules.map((rule) => (
                <IgnoreListItem
                  key={rule.id}
                  rule={rule}
                  onEdit={(id) => {
                    setEditingId(id);
                    setIsFormOpen(true);
                  }}
                  onDelete={setConfirmId}
                  onToggleApp={toggleApp}
                />
              ))}
            </div>
          )}
        </div>

        {isFormOpen && (
          <IgnoreFormPanel
            editingId={editingId || undefined}
            initialData={editingRule}
            onSave={saveRule}
            onClose={() => setIsFormOpen(false)}
          />
        )}

        {confirmId && (
          <ConfirmDialog
            isOpen
            title={t("ignore.confirm.deleteTitle", { defaultValue: "删除规则" })}
            message={t("ignore.confirm.deleteMessage", {
              defaultValue: "确定删除此忽略规则？相关 ignore 文件将同步更新。",
            })}
            onConfirm={async () => {
              await deleteRule(confirmId);
              setConfirmId(null);
            }}
            onCancel={() => setConfirmId(null)}
          />
        )}
      </div>
    );
  },
);

IgnorePanel.displayName = "IgnorePanel";
export default IgnorePanel;
