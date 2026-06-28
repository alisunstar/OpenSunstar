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
import {
  ExternalLink,
  Download,
  Check,
  Loader2,
  Server,
  TrendingUp,
  ShieldCheck,
} from "lucide-react";
import type { SmitheryServer } from "@/lib/api/smitheryRegistry";
import type { McpApps } from "@/types";
import { useAllMcpServers } from "@/hooks/useMcp";
import { useInstallFromSmithery } from "@/hooks/useMcpDiscovery";
import { settingsApi } from "@/lib/api";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import { MCP_APP_IDS } from "@/config/appConfig";
import type { AppId } from "@/lib/api/types";

interface McpSmitheryCardProps {
  server: SmitheryServer;
  /** 是否显示安装量（安装热榜 Tab 显示） */
  showUseCount?: boolean;
}

const DEFAULT_APPS: McpApps = {
  claude: true,
  codex: true,
  gemini: true,
  opencode: false,
  hermes: false,
  openclaw: false,
};

/** 格式化数字：39907 → 39.9k */
function formatCount(n: number): string {
  if (n >= 10000) return `${(n / 1000).toFixed(1)}k`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

export const McpSmitheryCard: React.FC<McpSmitheryCardProps> = ({
  server,
  showUseCount = false,
}) => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [selectedApps, setSelectedApps] = useState<McpApps>({ ...DEFAULT_APPS });

  const { data: existingServers } = useAllMcpServers();
  const installMutation = useInstallFromSmithery();

  const installedId = server.qualifiedName
    .replace(/[/@.]/g, "-")
    .replace(/^-+/, "")
    .replace(/-+$/, "");
  const isInstalled = existingServers && installedId in existingServers;

  const handleInstall = async () => {
    setLoading(true);
    try {
      await installMutation.mutateAsync({
        qualifiedName: server.qualifiedName,
        enabledApps: selectedApps,
      });
    } finally {
      setLoading(false);
    }
  };

  const handleOpenWebsite = async () => {
    const url = server.homepage || `https://smithery.ai/server/${server.qualifiedName}`;
    try {
      await settingsApi.openExternal(url);
    } catch {
      /* ignore */
    }
  };

  const displayName = server.displayName || server.qualifiedName;
  const description = server.description || "";

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
                {server.qualifiedName}
              </span>
              <Badge
                variant="secondary"
                className="shrink-0 text-[10px] px-1.5 py-0 h-4"
              >
                {server.remote ? "Remote" : "Stdio"}
              </Badge>
              {showUseCount && server.useCount > 0 && (
                <Badge
                  variant="outline"
                  className="shrink-0 text-[10px] px-1.5 py-0 h-4 border-border-default flex items-center gap-0.5"
                >
                  <TrendingUp className="h-2.5 w-2.5" />
                  {formatCount(server.useCount)}
                </Badge>
              )}
            </div>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {server.verified && (
              <Badge
                variant="default"
                className="bg-emerald-600/90 hover:bg-emerald-600 dark:bg-emerald-700/90 dark:hover:bg-emerald-700 text-white border-0 flex items-center gap-0.5"
              >
                <ShieldCheck className="h-2.5 w-2.5" />
                {t("mcp.discovery.verified", { defaultValue: "认证" })}
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
          <Button
            variant="ghost"
            size="sm"
            onClick={handleOpenWebsite}
            className="h-7 text-xs"
          >
            <ExternalLink className="h-3 w-3 mr-1" />
            {t("mcp.discovery.website", { defaultValue: "网站" })}
          </Button>
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
