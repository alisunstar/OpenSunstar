import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Cloud,
  CloudOff,
  Settings,
  CheckCircle2,
  XCircle,
  HardDrive,
  Globe,
  Github,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

// ── 类型 ──────────────────────────────────────────

interface WebdavConfig {
  enabled?: boolean;
  autoSync?: boolean;
  url?: string;
  username?: string;
}

interface S3Config {
  enabled?: boolean;
  autoSync?: boolean;
  bucket?: string;
  region?: string;
  endpoint?: string;
}

interface GistConfig {
  enabled?: boolean;
  token?: string;
  gistId?: string;
}

interface SyncSettings {
  webdavSync?: WebdavConfig;
  s3Sync?: S3Config;
  gistSync?: GistConfig;
}

interface BackendCardProps {
  icon: React.ReactNode;
  name: string;
  enabled: boolean;
  autoSync?: boolean;
  detail?: string;
  t: (key: string, opts?: Record<string, unknown>) => string;
}

// ── 子组件 ────────────────────────────────────────

function BackendCard({
  icon,
  name,
  enabled,
  autoSync,
  detail,
  t,
}: BackendCardProps) {
  return (
    <div
      className={cn(
        "rounded-xl border p-4 transition-colors",
        "glass-card",
        enabled
          ? "border-green-500/30 bg-green-500/5"
          : "border-border/40 bg-background/40",
      )}
    >
      <div className="flex items-start gap-3">
        <div
          className={cn(
            "shrink-0 w-10 h-10 rounded-lg flex items-center justify-center",
            enabled
              ? "bg-green-500/10 text-green-500"
              : "bg-muted text-muted-foreground",
          )}
        >
          {icon}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-foreground">{name}</span>
            {enabled ? (
              <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
            ) : (
              <XCircle className="w-3.5 h-3.5 text-muted-foreground/60" />
            )}
          </div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            {enabled
              ? t("cloudSyncDashboard.enabled")
              : t("cloudSyncDashboard.disabled")}
            {enabled && autoSync !== undefined && (
              <span className="ml-2">
                · {t("cloudSyncDashboard.autoSync")}:{" "}
                {autoSync
                  ? t("cloudSyncDashboard.on")
                  : t("cloudSyncDashboard.off")}
              </span>
            )}
          </div>
          {detail && (
            <div className="mt-1 text-[11px] text-muted-foreground/70 truncate">
              {detail}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ── 主组件 ────────────────────────────────────────

interface CloudSyncDashboardProps {
  onNavigate: (view: "settings") => void;
  onSettingsNavIntent?: (intent: {
    tab: "advanced";
    openSections: string[];
  }) => void;
}

export function CloudSyncDashboard({
  onNavigate,
  onSettingsNavIntent,
}: CloudSyncDashboardProps) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<SyncSettings | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchSettings = useCallback(async () => {
    try {
      const s = await invoke<SyncSettings>("get_settings");
      setSettings(s);
    } catch {
      // settings not available
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  const handleGoSettings = () => {
    if (onSettingsNavIntent) {
      onSettingsNavIntent({ tab: "advanced", openSections: ["cloudSync"] });
    }
    onNavigate("settings");
  };

  // Determine overall sync status
  const hasAnyBackend =
    settings?.webdavSync?.enabled ||
    settings?.s3Sync?.enabled ||
    settings?.gistSync?.enabled;

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Cloud className="w-8 h-8 text-muted-foreground animate-pulse" />
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-6">
        {/* Header */}
        <div>
          <h1 className="text-xl font-semibold text-foreground flex items-center gap-2">
            <Cloud className="w-5 h-5 text-blue-500" />
            {t("cloudSyncDashboard.title", { defaultValue: "跨设备云同步" })}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {t("cloudSyncDashboard.subtitle", {
              defaultValue:
                "配置数据在多台设备间实时同步，随时随地保持一致的开发环境",
            })}
          </p>
        </div>

        {/* Status summary */}
        <div
          className={cn(
            "rounded-xl border p-4 flex items-center gap-3",
            hasAnyBackend
              ? "border-green-500/30 bg-green-500/5"
              : "border-amber-500/30 bg-amber-500/5",
          )}
        >
          {hasAnyBackend ? (
            <CheckCircle2 className="w-5 h-5 text-green-500 shrink-0" />
          ) : (
            <CloudOff className="w-5 h-5 text-amber-500 shrink-0" />
          )}
          <div>
            <div className="text-sm font-medium">
              {t("cloudSyncDashboard.status")}
              {hasAnyBackend
                ? `: ${t("cloudSyncDashboard.enabled")}`
                : `: ${t("cloudSyncDashboard.notConfigured")}`}
            </div>
            {!hasAnyBackend && (
              <div className="text-xs text-muted-foreground mt-0.5">
                {t("cloudSyncDashboard.notConfiguredHint")}
              </div>
            )}
          </div>
        </div>

        {/* Backend cards */}
        <div className="space-y-3">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground/60">
            {t("cloudSyncDashboard.backend")}
          </h2>

          <BackendCard
            icon={<Globe className="w-5 h-5" />}
            name={t("cloudSyncDashboard.webdav")}
            enabled={!!settings?.webdavSync?.enabled}
            autoSync={settings?.webdavSync?.autoSync}
            detail={settings?.webdavSync?.url}
            t={t}
          />

          <BackendCard
            icon={<HardDrive className="w-5 h-5" />}
            name={t("cloudSyncDashboard.s3")}
            enabled={!!settings?.s3Sync?.enabled}
            autoSync={settings?.s3Sync?.autoSync}
            detail={
              settings?.s3Sync?.bucket
                ? `${settings.s3Sync.bucket}${settings.s3Sync.region ? ` (${settings.s3Sync.region})` : ""}`
                : settings?.s3Sync?.endpoint
            }
            t={t}
          />

          <BackendCard
            icon={<Github className="w-5 h-5" />}
            name={t("cloudSyncDashboard.gist")}
            enabled={!!settings?.gistSync?.enabled}
            detail={
              settings?.gistSync?.gistId
                ? `Gist ID: ${settings.gistSync.gistId}`
                : undefined
            }
            t={t}
          />
        </div>

        {/* Go to settings */}
        <div className="flex justify-center pt-2">
          <Button
            variant="outline"
            onClick={handleGoSettings}
            className="gap-2"
          >
            <Settings className="w-4 h-4" />
            {t("cloudSyncDashboard.goSettings", {
              defaultValue: "前往同步设置",
            })}
          </Button>
        </div>

        {/* Scope note */}
        <p className="text-[11px] text-muted-foreground/60 text-center leading-relaxed">
          {t("cloudSyncDashboard.description")}
        </p>
      </div>
    </div>
  );
}
