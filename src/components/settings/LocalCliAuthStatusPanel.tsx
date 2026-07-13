import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  KeyRound,
  Loader2,
  MonitorCheck,
  RefreshCw,
  Route,
  SearchCheck,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { authApi, type LocalCliAuthStatus } from "@/lib/api/auth";
import { settingsApi } from "@/lib/api";
import { cn } from "@/lib/utils";

interface ToolVersion {
  name: string;
  version: string | null;
  latest_version: string | null;
  error: string | null;
  installed_but_broken: boolean;
  env_type: "windows" | "wsl" | "macos" | "linux" | "unknown";
  wsl_distro: string | null;
}

type LayerTone = "success" | "warning" | "danger" | "muted";

interface LayerItem {
  icon: LucideIcon;
  label: string;
  value: string;
  tone: LayerTone;
}

const INSTALL_TOOL_NAMES = ["claude", "gemini"] as const;

function toneClass(tone: LayerTone): string {
  switch (tone) {
    case "success":
      return "border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300";
    case "warning":
      return "border-amber-500/25 bg-amber-500/10 text-amber-700 dark:text-amber-300";
    case "danger":
      return "border-destructive/25 bg-destructive/10 text-destructive";
    case "muted":
      return "border-border bg-muted/40 text-muted-foreground";
  }
}

function accessModeLabel(mode: LocalCliAuthStatus["accessMode"]): string {
  switch (mode) {
    case "third_party_key":
      return "第三方 Key";
    case "official_cli_login":
      return "官方登录";
    case "unknown":
      return "未识别";
  }
}

function credentialLabel(state: LocalCliAuthStatus["credentialState"]): string {
  switch (state) {
    case "present_unverified":
      return "Key 已发现";
    case "logged_in_detected":
      return "登录信号";
    case "missing":
      return "未发现凭据";
    case "unknown":
      return "无法确认";
  }
}

function routeLabel(state: LocalCliAuthStatus["routeState"]): string {
  switch (state) {
    case "applied":
      return "已应用";
    case "not_applied":
      return "未应用";
    case "not_applicable":
      return "不适用";
    case "unknown":
      return "无法确认";
  }
}

function confidenceLabel(confidence: LocalCliAuthStatus["confidence"]): string {
  switch (confidence) {
    case "high":
      return "高";
    case "medium":
      return "中";
    case "low":
      return "低";
  }
}

function installLayer(version: ToolVersion | undefined): LayerItem {
  if (!version) {
    return {
      icon: MonitorCheck,
      label: "工具",
      value: "未检测",
      tone: "muted",
    };
  }
  if (version.version) {
    return {
      icon: MonitorCheck,
      label: "工具",
      value: `已安装 ${version.version}`,
      tone: "success",
    };
  }
  if (version.installed_but_broken) {
    return {
      icon: MonitorCheck,
      label: "工具",
      value: "安装异常",
      tone: "danger",
    };
  }
  return {
    icon: MonitorCheck,
    label: "工具",
    value: "未安装",
    tone: "warning",
  };
}

function accessLayer(status: LocalCliAuthStatus): LayerItem {
  return {
    icon: ShieldCheck,
    label: "接入",
    value: accessModeLabel(status.accessMode),
    tone:
      status.accessMode === "unknown"
        ? "warning"
        : status.accessMode === "official_cli_login"
          ? "success"
          : "muted",
  };
}

function credentialLayer(status: LocalCliAuthStatus): LayerItem {
  return {
    icon: KeyRound,
    label: "凭据",
    value: credentialLabel(status.credentialState),
    tone:
      status.credentialState === "missing"
        ? "warning"
        : status.credentialState === "unknown"
          ? "muted"
          : "success",
  };
}

function routeLayer(status: LocalCliAuthStatus): LayerItem {
  return {
    icon: Route,
    label: "应用",
    value: routeLabel(status.routeState),
    tone:
      status.routeState === "applied"
        ? "success"
        : status.routeState === "not_applied"
          ? "warning"
          : "muted",
  };
}

function conclusion(
  status: LocalCliAuthStatus,
  version: ToolVersion | undefined,
): { label: string; tone: LayerTone; icon: LucideIcon } {
  if (!version?.version) {
    return {
      label: version?.installed_but_broken ? "安装异常" : "未安装",
      tone: "warning",
      icon: AlertCircle,
    };
  }
  if (
    status.accessMode === "third_party_key" &&
    status.routeState === "applied"
  ) {
    return { label: "可用", tone: "success", icon: CheckCircle2 };
  }
  if (status.accessMode === "official_cli_login") {
    return { label: "本地已登录", tone: "success", icon: CheckCircle2 };
  }
  if (status.credentialState === "missing") {
    return { label: "需处理", tone: "warning", icon: AlertCircle };
  }
  return { label: "无法确认", tone: "muted", icon: SearchCheck };
}

export function LocalCliAuthStatusPanel() {
  const { t } = useTranslation();
  const [statuses, setStatuses] = useState<LocalCliAuthStatus[]>([]);
  const [versions, setVersions] = useState<ToolVersion[]>([]);
  const [loading, setLoading] = useState(true);

  const versionByTool = useMemo(() => {
    return new Map(versions.map((version) => [version.name, version]));
  }, [versions]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [nextStatuses, nextVersions] = await Promise.all([
        authApi.getLocalCliAuthStatus(),
        settingsApi.getToolVersions([...INSTALL_TOOL_NAMES]),
      ]);
      setStatuses(nextStatuses);
      setVersions(nextVersions);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="rounded-xl border border-border/60 bg-card/60 p-6">
      <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <MonitorCheck className="h-5 w-5 text-primary" />
            <h4 className="font-medium">
              {t("settings.authCenter.localCliTitle", {
                defaultValue: "本地 CLI 登录状态",
              })}
            </h4>
            <Badge variant="outline" className="font-normal">
              {t("settings.authCenter.readonlyProbe", {
                defaultValue: "只读探测",
              })}
            </Badge>
          </div>
          <p className="text-sm text-muted-foreground">
            {t("settings.authCenter.localCliDescription", {
              defaultValue:
                "区分第三方 Key 与官方订阅登录，只披露本地状态信号，不接管 Claude 或 Google 账号凭据。",
            })}
          </p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 gap-1.5 self-start"
          disabled={loading}
          onClick={() => void load()}
        >
          {loading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RefreshCw className="h-3.5 w-3.5" />
          )}
          {t("common.refresh", { defaultValue: "刷新" })}
        </Button>
      </div>

      {loading ? (
        <div
          className={cn(
            "flex items-center justify-center gap-2 rounded-lg border",
            "border-dashed border-border/60 py-8 text-sm text-muted-foreground",
          )}
        >
          <Loader2 className="h-4 w-4 animate-spin" />
          {t("settings.authCenter.localCliLoading", {
            defaultValue: "正在读取本地 CLI 授权状态…",
          })}
        </div>
      ) : (
        <div className="grid gap-3 lg:grid-cols-2">
          {statuses.map((status) => {
            const version = versionByTool.get(status.toolKey);
            const result = conclusion(status, version);
            const ResultIcon = result.icon;
            const layers = [
              installLayer(version),
              accessLayer(status),
              credentialLayer(status),
              routeLayer(status),
            ];

            return (
              <article
                key={status.toolKey}
                className="rounded-lg border border-border/50 bg-background/50 p-4"
              >
                <div className="mb-3 flex items-start justify-between gap-3">
                  <div>
                    <h5 className="font-medium">{status.displayName}</h5>
                    <p className="mt-0.5 break-all text-xs text-muted-foreground">
                      {status.configPath}
                    </p>
                  </div>
                  <Badge
                    variant="outline"
                    className={cn("shrink-0 gap-1.5", toneClass(result.tone))}
                  >
                    <ResultIcon className="h-3.5 w-3.5" />
                    {result.label}
                  </Badge>
                </div>

                <div className="grid gap-2 sm:grid-cols-2">
                  {layers.map((layer) => {
                    const Icon = layer.icon;
                    return (
                      <div
                        key={layer.label}
                        className={cn(
                          "flex items-center gap-2 rounded-md border px-2.5 py-2 text-xs",
                          toneClass(layer.tone),
                        )}
                      >
                        <Icon className="h-3.5 w-3.5 shrink-0" />
                        <span className="text-muted-foreground">
                          {layer.label}
                        </span>
                        <span className="min-w-0 truncate font-medium">
                          {layer.value}
                        </span>
                      </div>
                    );
                  })}
                </div>

                <dl className="mt-3 grid gap-2 text-xs text-muted-foreground sm:grid-cols-2">
                  <div>
                    <dt className="font-medium text-foreground">探测置信度</dt>
                    <dd>{confidenceLabel(status.confidence)}</dd>
                  </div>
                  <div>
                    <dt className="font-medium text-foreground">当前模型</dt>
                    <dd>{status.simpleConnectModel ?? "未识别"}</dd>
                  </div>
                  <div>
                    <dt className="font-medium text-foreground">Base URL</dt>
                    <dd className="truncate">
                      {status.simpleConnectBaseUrl ?? "未识别"}
                    </dd>
                  </div>
                  <div>
                    <dt className="font-medium text-foreground">Key 提示</dt>
                    <dd>{status.keyHint ?? "不显示或未发现"}</dd>
                  </div>
                </dl>

                {status.evidence.length > 0 ? (
                  <ul className="mt-3 space-y-1 border-t border-border/50 pt-3 text-xs text-muted-foreground">
                    {status.evidence.slice(0, 4).map((item) => (
                      <li key={item} className="flex gap-2">
                        <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-muted-foreground/50" />
                        <span>{item}</span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="mt-3 border-t border-border/50 pt-3 text-xs text-muted-foreground">
                    未发现可披露的本地授权信号。请在 CLI 内完成官方登录，或在快速接入中保存第三方 Key。
                  </p>
                )}
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
