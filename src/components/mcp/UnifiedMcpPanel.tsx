import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Server } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import {
  useAllMcpServers,
  useToggleMcpApp,
  useDeleteMcpServer,
  useImportMcpFromApps,
} from "@/hooks/useMcp";
import type { McpServer } from "@/types";
import type { AppId } from "@/lib/api/types";
import McpFormModal from "./McpFormModal";
import { ConfirmDialog } from "../ConfirmDialog";
import { Edit3, Trash2, ExternalLink, Wifi, WifiOff, ShieldAlert, Loader2 } from "lucide-react";
import { settingsApi } from "@/lib/api";
import { mcpPresets } from "@/config/mcpPresets";
import { toast } from "sonner";
import { MCP_APP_IDS } from "@/config/appConfig";
import { AppCountBar } from "@/components/common/AppCountBar";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import { ListItemRow } from "@/components/common/ListItemRow";
import { useMcpTestConnection } from "@/hooks/useMcpDiscovery";
import { mcpRegistryApi } from "@/lib/api/mcpRegistry";
import type { McpConnectionStatus, McpConnectionTestResult } from "@/lib/api/mcpRegistry";

interface UnifiedMcpPanelProps {
  onOpenChange: (open: boolean) => void;
}

export interface UnifiedMcpPanelHandle {
  openAdd: () => void;
  openImport: () => void;
  openDiscovery: () => void;
}

const UnifiedMcpPanel = React.forwardRef<
  UnifiedMcpPanelHandle,
  UnifiedMcpPanelProps
>(({ onOpenChange: _onOpenChange }, ref) => {
  const { t } = useTranslation();
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
  } | null>(null);

  // 批量测试链接状态
  const [batchTestRunning, setBatchTestRunning] = useState(false);

  const { data: serversMap, isLoading } = useAllMcpServers();
  const toggleAppMutation = useToggleMcpApp();
  const deleteServerMutation = useDeleteMcpServer();
  const importMutation = useImportMcpFromApps();

  const serverEntries = useMemo((): Array<[string, McpServer]> => {
    if (!serversMap) return [];
    return Object.entries(serversMap);
  }, [serversMap]);

  const enabledCounts = useMemo(() => {
    const counts = {
      claude: 0,
      "claude-desktop": 0,
      codex: 0,
      gemini: 0,
      opencode: 0,
      openclaw: 0,
      hermes: 0,
    };
    serverEntries.forEach(([_, server]) => {
      for (const app of MCP_APP_IDS) {
        if (server.apps[app]) counts[app]++;
      }
    });
    return counts;
  }, [serverEntries]);

  const handleToggleApp = async (
    serverId: string,
    app: AppId,
    enabled: boolean,
  ) => {
    try {
      await toggleAppMutation.mutateAsync({ serverId, app, enabled });
    } catch (error) {
      toast.error(t("common.error"), { description: String(error) });
    }
  };

  const handleEdit = (id: string) => {
    setEditingId(id);
    setIsFormOpen(true);
  };

  const handleAdd = () => {
    setEditingId(null);
    setIsFormOpen(true);
  };

  const handleImport = async () => {
    try {
      const count = await importMutation.mutateAsync();
      if (count === 0) {
        toast.success(t("mcp.unifiedPanel.noImportFound"), {
          closeButton: true,
        });
      } else {
        toast.success(t("mcp.unifiedPanel.importSuccess", { count }), {
          closeButton: true,
        });
      }
    } catch (error) {
      toast.error(t("common.error"), { description: String(error) });
    }
  };

  // 批量测试所有 MCP 服务器链接
  const handleBatchTestConnections = async () => {
    if (serverEntries.length === 0) {
      toast.error(t("mcp.test.noServersToTest", { defaultValue: "没有可测试的服务器" }));
      return;
    }
    setBatchTestRunning(true);

    const results = new Map<string, McpConnectionTestResult>();
    let completed = 0;

    for (const [id, server] of serverEntries) {
      try {
        const result = await mcpRegistryApi.testConnection(
          server.server as Record<string, unknown>,
        );
        results.set(id, result);
      } catch (e: any) {
        results.set(id, {
          status: "unreachable" as McpConnectionStatus,
          message: String(e),
        });
      }
      completed++;
      if (completed < serverEntries.length) {
        toast.loading(
          t("mcp.test.batchProgress", {
            defaultValue: `测试中... (${completed}/${serverEntries.length})`,
            completed,
            total: serverEntries.length,
          }),
          { id: "batch-test-progress", duration: Infinity },
        );
      }
    }

    setBatchTestRunning(false);
    toast.dismiss("batch-test-progress");

    const connected = Array.from(results.values()).filter(
      (r) => r.status === "connected",
    ).length;
    const total = results.size;
    if (connected === total) {
      toast.success(
        t("mcp.test.allConnected", {
          defaultValue: `全部 ${total} 个服务器连接成功`,
          count: total,
        }),
        { closeButton: true },
      );
    } else {
      const failed = total - connected;
      toast.warning(
        t("mcp.test.batchSummary", {
          defaultValue: `${connected} 个成功，${failed} 个失败（共 ${total} 个）`,
          connected,
          failed,
          total,
        }),
        { closeButton: true, duration: 6000 },
      );
    }
  };

  React.useImperativeHandle(ref, () => ({
    openAdd: handleAdd,
    openImport: handleImport,
    openDiscovery: () => {},
  }));

  const handleDelete = (id: string) => {
    setConfirmDialog({
      isOpen: true,
      title: t("mcp.unifiedPanel.deleteServer"),
      message: t("mcp.unifiedPanel.deleteConfirm", { id }),
      onConfirm: async () => {
        try {
          await deleteServerMutation.mutateAsync(id);
          setConfirmDialog(null);
          toast.success(t("common.success"), { closeButton: true });
        } catch (error) {
          toast.error(t("common.error"), { description: String(error) });
        }
      },
    });
  };

  const handleCloseForm = () => {
    setIsFormOpen(false);
    setEditingId(null);
  };

  return (
    <div className="px-6 flex flex-col flex-1 min-h-0 overflow-hidden">
      <div className="flex-shrink-0 flex items-center gap-3">
        <div className="flex-1">
          <AppCountBar
            totalLabel={t("mcp.serverCount", { count: serverEntries.length })}
            counts={enabledCounts}
            appIds={MCP_APP_IDS}
          />
        </div>
        {serverEntries.length > 0 && (
          <Button
            variant="outline"
            size="sm"
            onClick={handleBatchTestConnections}
            disabled={batchTestRunning}
            className="h-9 px-3 flex-shrink-0"
            title={t("mcp.test.batchTestConnection", { defaultValue: "批量测试所有服务器链接" })}
          >
            {batchTestRunning ? (
              <Loader2 size={14} className="animate-spin mr-1" />
            ) : (
              <Wifi size={14} className="mr-1" />
            )}
            {t("mcp.test.testConnection", { defaultValue: "测试链接" })}
          </Button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto overflow-x-hidden pb-24">
        {isLoading ? (
          <div className="text-center py-12 text-muted-foreground">
            {t("mcp.loading")}
          </div>
        ) : serverEntries.length === 0 ? (
          <div className="text-center py-12">
            <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
              <Server size={24} className="text-muted-foreground" />
            </div>
            <h3 className="text-lg font-medium text-foreground mb-2">
              {t("mcp.unifiedPanel.noServers")}
            </h3>
            <p className="text-muted-foreground text-sm">
              {t("mcp.emptyDescription")}
            </p>
          </div>
        ) : (
          <TooltipProvider delayDuration={300}>
            <div className="rounded-xl border border-border-default overflow-hidden">
              {serverEntries.map(([id, server], index) => (
                <UnifiedMcpListItem
                  key={id}
                  id={id}
                  server={server}
                  onToggleApp={handleToggleApp}
                  onEdit={handleEdit}
                  onDelete={handleDelete}
                  isLast={index === serverEntries.length - 1}
                />
              ))}
            </div>
          </TooltipProvider>
        )}
      </div>

      {isFormOpen && (
        <McpFormModal
          editingId={editingId || undefined}
          initialData={
            editingId && serversMap ? serversMap[editingId] : undefined
          }
          existingIds={serversMap ? Object.keys(serversMap) : []}
          defaultFormat="json"
          onSave={async () => {
            setIsFormOpen(false);
            setEditingId(null);
          }}
          onClose={handleCloseForm}
        />
      )}

      {confirmDialog && (
        <ConfirmDialog
          isOpen={confirmDialog.isOpen}
          title={confirmDialog.title}
          message={confirmDialog.message}
          onConfirm={confirmDialog.onConfirm}
          onCancel={() => setConfirmDialog(null)}
        />
      )}
    </div>
  );
});

UnifiedMcpPanel.displayName = "UnifiedMcpPanel";

interface UnifiedMcpListItemProps {
  id: string;
  server: McpServer;
  onToggleApp: (serverId: string, app: AppId, enabled: boolean) => void;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
  isLast?: boolean;
}

const getTestStatusIcon = (status: McpConnectionStatus | null) => {
  switch (status) {
    case "connected":
      return <Wifi size={12} className="text-emerald-500" />;
    case "auth_required":
      return <ShieldAlert size={12} className="text-amber-500" />;
    case "unreachable":
    case "timeout":
    case "command_failed":
      return <WifiOff size={12} className="text-red-500" />;
    default:
      return null;
  }
};

const getTestStatusLabel = (status: McpConnectionStatus | null, t: (k: string, options?: Record<string, unknown>) => string) => {
  switch (status) {
    case "connected":
      return t("mcp.test.connected", { defaultValue: "已连接" });
    case "auth_required":
      return t("mcp.test.authRequired", { defaultValue: "需验证" });
    case "unreachable":
      return t("mcp.test.unreachable", { defaultValue: "无法连接" });
    case "timeout":
      return t("mcp.test.timeout", { defaultValue: "超时" });
    case "command_failed":
      return t("mcp.test.commandFailed", { defaultValue: "启动失败" });
    case "unexpected_response":
      return t("mcp.test.unexpected", { defaultValue: "异常" });
    default:
      return "";
  }
};

const UnifiedMcpListItem: React.FC<UnifiedMcpListItemProps> = ({
  id,
  server,
  onToggleApp,
  onEdit,
  onDelete,
  isLast,
}) => {
  const { t } = useTranslation();
  const name = server.name || id;
  const description = server.description || "";

  const meta = mcpPresets.find((p) => p.id === id);
  const docsUrl = server.docs || meta?.docs;
  const homepageUrl = server.homepage || meta?.homepage;
  const tags = server.tags || meta?.tags;

  const testMutation = useMcpTestConnection();
  const [testStatus, setTestStatus] = useState<McpConnectionStatus | null>(null);
  const [testMessage, setTestMessage] = useState<string>("");

  const openDocs = async () => {
    const url = docsUrl || homepageUrl;
    if (!url) return;
    try {
      await settingsApi.openExternal(url);
    } catch {
      // ignore
    }
  };

  const handleTestConnection = async () => {
    setTestStatus(null);
    setTestMessage("");
    try {
      const result = await testMutation.mutateAsync(server.server as Record<string, unknown>);
      setTestStatus(result.status);
      setTestMessage(result.message);
      if (result.server_info?.name) {
        setTestMessage(`${result.message} (${result.server_info.name}${result.server_info.version ? " v" + result.server_info.version : ""})`);
      }
    } catch (e: any) {
      setTestStatus("unreachable");
      setTestMessage(String(e));
    }
  };

  const isTesting = testMutation.isPending;

  return (
    <ListItemRow isLast={isLast}>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5">
          <span className="font-medium text-sm text-foreground truncate">
            {name}
          </span>
          {docsUrl && (
            <button
              type="button"
              onClick={openDocs}
              className="text-muted-foreground/60 hover:text-foreground flex-shrink-0"
              title={t("mcp.presets.docs")}
            >
              <ExternalLink size={12} />
            </button>
          )}
          {/* 连接测试状态指示 */}
          {testStatus && (
            <span
              className="flex items-center gap-0.5 text-[10px] flex-shrink-0"
              title={testMessage}
            >
              {getTestStatusIcon(testStatus)}
              <span className="text-muted-foreground/80">{getTestStatusLabel(testStatus, t)}</span>
            </span>
          )}
        </div>
        {description && (
          <p
            className="text-xs text-muted-foreground truncate"
            title={description}
          >
            {description}
          </p>
        )}
        {!description && tags && tags.length > 0 && (
          <p className="text-xs text-muted-foreground/60 truncate">
            {tags.join(", ")}
          </p>
        )}
        {/* 测试结果消息 */}
        {testMessage && testStatus && testStatus !== "connected" && (
          <p className="text-[10px] text-muted-foreground/70 truncate mt-0.5" title={testMessage}>
            {testMessage}
          </p>
        )}
      </div>

      <AppToggleGroup
        apps={server.apps}
        onToggle={(app, enabled) => onToggleApp(id, app, enabled)}
        appIds={MCP_APP_IDS}
      />

      <div className="flex items-center gap-0.5 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
        {/* 测试链接按钮 */}
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={handleTestConnection}
          disabled={isTesting}
          title={t("mcp.test.testConnection", { defaultValue: "测试链接" })}
        >
          {isTesting ? (
            <Loader2 size={13} className="animate-spin" />
          ) : (
            getTestStatusIcon(testStatus) || <Wifi size={13} className="text-muted-foreground/50" />
          )}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => onEdit(id)}
          title={t("common.edit")}
        >
          <Edit3 size={14} />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 hover:text-red-500 hover:bg-red-100 dark:hover:text-red-400 dark:hover:bg-red-500/10"
          onClick={() => onDelete(id)}
          title={t("common.delete")}
        >
          <Trash2 size={14} />
        </Button>
      </div>
    </ListItemRow>
  );
};

export default UnifiedMcpPanel;
