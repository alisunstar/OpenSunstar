import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Bot } from "lucide-react";
import { useAgentActions } from "@/hooks/useAgentActions";
import { AgentListItem } from "./AgentListItem";
import { AgentFormPanel } from "./AgentFormPanel";
import { ConfirmDialog } from "../ConfirmDialog";

export interface AgentsPanelHandle {
  openAdd: () => void;
}

const AgentsPanel = React.forwardRef<AgentsPanelHandle, { open: boolean }>(
  ({ open }, ref) => {
    const { t } = useTranslation();
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [confirmDialog, setConfirmDialog] = useState<{
      isOpen: boolean;
      id: string;
      name: string;
    } | null>(null);

    const { agents, loading, reload, saveAgent, deleteAgent, toggleApp } =
      useAgentActions();

    useEffect(() => {
      if (open) void reload();
    }, [open, reload]);

    React.useImperativeHandle(ref, () => ({
      openAdd: () => {
        setEditingId(null);
        setIsFormOpen(true);
      },
    }));

    const entries = useMemo(() => Object.values(agents), [agents]);

    return (
      <div className="flex flex-col flex-1 min-h-0 px-6">
        <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6">
          <div className="text-sm text-muted-foreground">
            {t("agents.count", {
              count: entries.length,
              defaultValue: "共 {{count}} 个 Subagent",
            })}
          </div>
          <p className="text-xs text-muted-foreground mt-2">
            {t("agents.subtitle", {
              defaultValue:
                "同步到 Claude / Gemini / OpenCode 的 agents 目录（Markdown）",
            })}
          </p>
        </div>

        <div className="flex-1 overflow-y-auto pb-16">
          {loading ? (
            <div className="text-center py-12 text-muted-foreground">
              {t("common.loading")}
            </div>
          ) : entries.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
                <Bot size={24} className="text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground mb-2">
                {t("agents.empty", { defaultValue: "暂无 Subagent" })}
              </h3>
              <p className="text-sm text-muted-foreground max-w-md mx-auto">
                {t("agents.emptyHint", {
                  defaultValue:
                    "创建自定义 Subagent 并同步到 ~/.claude/agents/ 等目录",
                })}
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {entries.map((agent) => (
                <AgentListItem
                  key={agent.id}
                  agent={agent}
                  onEdit={(id) => {
                    setEditingId(id);
                    setIsFormOpen(true);
                  }}
                  onDelete={(id) =>
                    setConfirmDialog({
                      isOpen: true,
                      id,
                      name: agent.name,
                    })
                  }
                  onToggleApp={toggleApp}
                />
              ))}
            </div>
          )}
        </div>

        {isFormOpen && (
          <AgentFormPanel
            editingId={editingId || undefined}
            initialData={editingId ? agents[editingId] : undefined}
            onSave={saveAgent}
            onClose={() => setIsFormOpen(false)}
          />
        )}

        {confirmDialog && (
          <ConfirmDialog
            isOpen={confirmDialog.isOpen}
            title={t("agents.confirm.deleteTitle", {
              defaultValue: "删除 Subagent",
            })}
            message={t("agents.confirm.deleteMessage", {
              name: confirmDialog.name,
              defaultValue:
                '确定删除 Subagent "{{name}}"？已同步的文件将被清理。',
            })}
            onConfirm={async () => {
              try {
                await deleteAgent(confirmDialog.id);
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

AgentsPanel.displayName = "AgentsPanel";

export default AgentsPanel;
