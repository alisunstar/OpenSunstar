import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ExternalLink, Download, Check, Loader2, Globe, Server } from "lucide-react";
import type { RegistryServer } from "@/lib/api/mcpRegistry";
import type { McpApps } from "@/types";
import { useAllMcpServers } from "@/hooks/useMcp";
import { useInstallFromRegistry } from "@/hooks/useMcpDiscovery";
import { settingsApi } from "@/lib/api";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import { MCP_APP_IDS } from "@/config/appConfig";
import type { AppId } from "@/lib/api/types";

interface McpDiscoveryCardProps {
  server: RegistryServer;
  meta?: Record<string, unknown>;
}

const DEFAULT_APPS: McpApps = {
  claude: true,
  codex: true,
  gemini: true,
  opencode: false,
  hermes: false,
  openclaw: false,
};

export const McpDiscoveryCard: React.FC<McpDiscoveryCardProps> = ({
  server,
  meta: _meta,
}) => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [selectedApps, setSelectedApps] = useState<McpApps>({ ...DEFAULT_APPS });

  const { data: existingServers } = useAllMcpServers();
  const installMutation = useInstallFromRegistry();

  const installedId = server.name.replace(/[/@.]/g, "-").replace(/^-+/, "").replace(/-+$/, "");
  const isInstalled = existingServers && installedId in existingServers;

  const handleInstall = async () => {
    setLoading(true);
    try {
      await installMutation.mutateAsync({
        name: server.name,
        enabledApps: selectedApps,
      });
    } finally {
      setLoading(false);
    }
  };

  const handleOpenWebsite = async () => {
    if (server.websiteUrl) {
      try {
        await settingsApi.openExternal(server.websiteUrl);
      } catch { /* ignore */ }
    }
  };

  const handleOpenRepo = async () => {
    if (server.repository?.url) {
      try {
        await settingsApi.openExternal(server.repository.url);
      } catch { /* ignore */ }
    }
  };

  const displayName = server.title || server.name;
  const description = server.description || "";
  const version = server.version;
  const remoteTypes = server.remotes?.map((r) => r.type).join(", ") || "";
  const isOfficial =
    _meta &&
    "io.modelcontextprotocol.registry/official" in (_meta || {});

  return (
    <Card className="glass-card flex flex-col h-full transition-all duration-300 hover:shadow-lg group relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />

      <CardHeader className="pb-2">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5">
              <Server size={14} className="text-muted-foreground flex-shrink-0" />
              <CardTitle className="text-sm font-semibold truncate">
                {displayName}
              </CardTitle>
            </div>
            <div className="flex items-center gap-1.5 mt-1.5 flex-wrap">
              <span className="text-[10px] text-muted-foreground/60 font-mono truncate">
                {server.name}
              </span>
              {version && (
                <Badge variant="outline" className="shrink-0 text-[10px] px-1.5 py-0 h-4 border-border-default">
                  v{version}
                </Badge>
              )}
              {remoteTypes && (
                <Badge variant="secondary" className="shrink-0 text-[10px] px-1.5 py-0 h-4">
                  <Globe className="h-2.5 w-2.5 mr-0.5" />
                  {remoteTypes}
                </Badge>
              )}
            </div>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {isOfficial && (
              <Badge
                variant="default"
                className="bg-emerald-600/90 hover:bg-emerald-600 dark:bg-emerald-700/90 dark:hover:bg-emerald-700 text-white border-0"
              >
                {t("mcp.discovery.official", { defaultValue: "官方" })}
              </Badge>
            )}
            {isInstalled && (
              <Badge
                variant="default"
                className="bg-green-600/90 hover:bg-green-600 dark:bg-green-700/90 dark:hover:bg-green-700 text-white border-0"
              >
                <Check className="h-2.5 w-2.5 mr-0.5" />
                {t("mcp.discovery.installed", { defaultValue: "已安装" })}
              </Badge>
            )}
          </div>
        </div>
      </CardHeader>

      {description ? (
        <CardContent className="flex-1 pt-0">
          <p className="text-xs text-muted-foreground/90 line-clamp-3 leading-relaxed">
            {description}
          </p>
          {!isInstalled && (
            <div className="mt-3 pt-3 border-t border-border/50">
              <label className="text-[10px] font-medium text-foreground block mb-2">
                {t("mcp.form.enabledApps", { defaultValue: "启用到应用" })}
              </label>
              <AppToggleGroup
                apps={selectedApps}
                onToggle={(app: AppId, enabled: boolean) =>
                  setSelectedApps((prev) => ({ ...prev, [app]: enabled }))
                }
                appIds={MCP_APP_IDS}
              />
            </div>
          )}
        </CardContent>
      ) : (
        <div className="flex-1">
          {!isInstalled && (
            <div className="px-6 pb-2">
              <label className="text-[10px] font-medium text-foreground block mb-2">
                {t("mcp.form.enabledApps", { defaultValue: "启用到应用" })}
              </label>
              <AppToggleGroup
                apps={selectedApps}
                onToggle={(app: AppId, enabled: boolean) =>
                  setSelectedApps((prev) => ({ ...prev, [app]: enabled }))
                }
                appIds={MCP_APP_IDS}
              />
            </div>
          )}
        </div>
      )}

      <CardFooter className="flex gap-2 pt-3 border-t border-border/50 relative z-10">
        <div className="flex items-center gap-1 flex-1">
          {server.websiteUrl && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleOpenWebsite}
              className="h-7 text-xs"
            >
              <ExternalLink className="h-3 w-3 mr-1" />
              {t("mcp.discovery.website", { defaultValue: "网站" })}
            </Button>
          )}
          {server.repository?.url && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleOpenRepo}
              className="h-7 text-xs"
            >
              <ExternalLink className="h-3 w-3 mr-1" />
              {t("mcp.discovery.repository", { defaultValue: "仓库" })}
            </Button>
          )}
        </div>
        {isInstalled ? (
          <span className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-medium bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 whitespace-nowrap">
            <Check size={12} />
            {t("mcp.discovery.installed", { defaultValue: "已安装" })}
          </span>
        ) : (
          <Button
            variant="mcp"
            size="sm"
            onClick={handleInstall}
            disabled={loading || installMutation.isPending}
            className="whitespace-nowrap"
          >
            {loading ? (
              <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
            ) : (
              <Download className="h-3.5 w-3.5 mr-1.5" />
            )}
            {loading
              ? t("skills.installing", { defaultValue: "安装中..." })
              : t("mcp.discovery.install", { defaultValue: "安装" })}
          </Button>
        )}
      </CardFooter>
    </Card>
  );
};
