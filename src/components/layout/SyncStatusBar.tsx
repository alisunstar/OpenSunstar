import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Cloud, CloudOff, Loader2, AlertCircle, CheckCircle2 } from "lucide-react";
import { cn } from "@/lib/utils";

type SyncState = "idle" | "syncing" | "success" | "error" | "disabled";

interface SyncInfo {
  state: SyncState;
  lastSyncAt: number | null;
  lastError: string | null;
  backend: "webdav" | "s3" | "none";
}

function formatRelativeTime(timestamp: number): string {
  const now = Math.floor(Date.now() / 1000);
  const diff = now - timestamp;

  if (diff < 60) return "刚刚";
  if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`;
  return `${Math.floor(diff / 86400)}天前`;
}

export function SyncStatusBar({ collapsed }: { collapsed: boolean }) {
  const [syncInfo, setSyncInfo] = useState<SyncInfo>({
    state: "idle",
    lastSyncAt: null,
    lastError: null,
    backend: "none",
  });

  const fetchStatus = useCallback(async () => {
    try {
      const settings = await invoke<any>("get_settings");

      const webdav = settings?.webdavSync;
      const s3 = settings?.s3Sync;

      if (webdav?.enabled && webdav?.autoSync) {
        setSyncInfo((prev) => ({
          ...prev,
          backend: "webdav",
          lastSyncAt: webdav.status?.lastSyncAt ?? null,
          lastError: webdav.status?.lastError ?? null,
          state: webdav.status?.lastError ? "error" : prev.state === "syncing" ? "syncing" : "idle",
        }));
      } else if (s3?.enabled && s3?.autoSync) {
        setSyncInfo((prev) => ({
          ...prev,
          backend: "s3",
          lastSyncAt: s3.status?.lastSyncAt ?? null,
          lastError: s3.status?.lastError ?? null,
          state: s3.status?.lastError ? "error" : prev.state === "syncing" ? "syncing" : "idle",
        }));
      } else {
        setSyncInfo({ state: "disabled", lastSyncAt: null, lastError: null, backend: "none" });
      }
    } catch {
      // Settings not available yet
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 30000);
    return () => clearInterval(interval);
  }, [fetchStatus]);

  // Listen for sync status events
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      listen<{ source: string; status: string; error?: string }>("webdav-sync-status-updated", (event) => {
        const { status, error } = event.payload;
        setSyncInfo((prev) => {
          if (prev.backend !== "webdav" && prev.backend !== "none") return prev;
          if (status === "success") {
            return { ...prev, state: "success", lastSyncAt: Math.floor(Date.now() / 1000), lastError: null, backend: "webdav" };
          }
          if (status === "error") {
            return { ...prev, state: "error", lastError: error ?? "Unknown error", backend: "webdav" };
          }
          return { ...prev, state: "syncing", backend: "webdav" };
        });
      })
    );

    unlisteners.push(
      listen<{ source: string; status: string; error?: string }>("s3-sync-status-updated", (event) => {
        const { status, error } = event.payload;
        setSyncInfo((prev) => {
          if (prev.backend !== "s3" && prev.backend !== "none") return prev;
          if (status === "success") {
            return { ...prev, state: "success", lastSyncAt: Math.floor(Date.now() / 1000), lastError: null, backend: "s3" };
          }
          if (status === "error") {
            return { ...prev, state: "error", lastError: error ?? "Unknown error", backend: "s3" };
          }
          return { ...prev, state: "syncing", backend: "s3" };
        });
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  // Auto-clear "success" state after 5s → back to idle
  useEffect(() => {
    if (syncInfo.state === "success") {
      const timer = setTimeout(() => setSyncInfo((prev) => ({ ...prev, state: "idle" })), 5000);
      return () => clearTimeout(timer);
    }
  }, [syncInfo.state]);

  if (syncInfo.state === "disabled") return null;

  const stateConfig: Record<SyncState, { icon: React.ReactNode; color: string; label: string }> = {
    idle: {
      icon: <Cloud className="w-3 h-3" />,
      color: "text-muted-foreground",
      label: "等待变更",
    },
    syncing: {
      icon: <Loader2 className="w-3 h-3 animate-spin" />,
      color: "text-blue-500",
      label: "同步中...",
    },
    success: {
      icon: <CheckCircle2 className="w-3 h-3" />,
      color: "text-green-500",
      label: "已同步",
    },
    error: {
      icon: <AlertCircle className="w-3 h-3" />,
      color: "text-red-500",
      label: "同步失败",
    },
    disabled: {
      icon: <CloudOff className="w-3 h-3" />,
      color: "text-muted-foreground",
      label: "未启用",
    },
  };

  const config = stateConfig[syncInfo.state];

  if (collapsed) {
    return (
      <div
        className={cn("flex justify-center py-1.5", config.color)}
        title={`${config.label}${syncInfo.lastSyncAt ? ` · ${formatRelativeTime(syncInfo.lastSyncAt)}` : ""}`}
      >
        {config.icon}
      </div>
    );
  }

  return (
    <div className={cn("flex items-center gap-1.5 px-3 py-1.5 text-[11px]", config.color)}>
      {config.icon}
      <span className="truncate">
        {syncInfo.backend === "webdav" ? "WebDAV" : syncInfo.backend === "s3" ? "S3" : ""}
        {" · "}
        {config.label}
      </span>
      {syncInfo.lastSyncAt && syncInfo.state !== "syncing" && (
        <span className="ml-auto text-muted-foreground/60 shrink-0">
          {formatRelativeTime(syncInfo.lastSyncAt)}
        </span>
      )}
    </div>
  );
}
