import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, Clock, Loader2, Server } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  simpleConnectApi,
  type SimpleConnectRuntimeStats,
} from "@/lib/api/simpleConnect";

interface PoolHealthPanelProps {
  enabled?: boolean;
  pollMs?: number;
  embedded?: boolean;
}

export function PoolHealthPanel({
  enabled = true,
  pollMs = 3000,
  embedded = false,
}: PoolHealthPanelProps) {
  const { t } = useTranslation();
  const [stats, setStats] = useState<SimpleConnectRuntimeStats | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const next = await simpleConnectApi.poolStats();
      setStats(next);
    } catch {
      /* optional telemetry */
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;
    void refresh();
    const id = window.setInterval(() => void refresh(), pollMs);
    return () => window.clearInterval(id);
  }, [enabled, pollMs, refresh]);

  if (!enabled) return null;

  const keys = stats?.pool_keys ?? [];

  const body = (
    <>
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-sm font-medium">
          <Activity className="h-4 w-4 text-primary" />
          {t("simpleConnect.poolHealth.title", {
            defaultValue: "密钥池运行时",
          })}
        </div>
        {loading && !stats ? (
          <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        ) : (
          <Badge variant={stats?.running ? "default" : "secondary"}>
            {stats?.running
              ? t("simpleConnect.poolHealth.running", {
                  defaultValue: "代理运行中",
                })
              : t("simpleConnect.poolHealth.stopped", {
                  defaultValue: "代理未启动",
                })}
          </Badge>
        )}
      </div>

      {stats?.running && (
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1">
            <Server className="h-3 w-3" />
            {stats.local_base ?? `:${stats.port}`}
          </span>
          {stats.upstream && (
            <span className="font-mono truncate max-w-[240px]">
              → {stats.upstream}
            </span>
          )}
        </div>
      )}

      {!stats?.running && !loading && (
        <p className="text-xs text-muted-foreground">
          {t("simpleConnect.poolHealth.idleHint", {
            defaultValue: "应用配置后将启动本地代理，此处显示 Key 成功/失败与冷却状态",
          })}
        </p>
      )}

      {keys.length > 0 && (
        <ul className="space-y-2">
          {keys.map((k) => (
            <li
              key={k.id}
              className="flex flex-wrap items-center gap-2 rounded-md border border-border/40 px-3 py-2 text-xs"
            >
              <span className="font-medium min-w-0 truncate">{k.label}</span>
              <Badge
                variant={k.available ? "outline" : "secondary"}
                className="text-[10px]"
              >
                {k.available
                  ? t("simpleConnect.poolHealth.available", {
                      defaultValue: "可用",
                    })
                  : t("simpleConnect.poolHealth.cooling", {
                      defaultValue: "冷却中",
                    })}
              </Badge>
              <span className="text-muted-foreground">
                ✓{k.success} / ✗{k.failure}
              </span>
              {k.weight > 1 && (
                <span className="text-muted-foreground">w{k.weight}</span>
              )}
              {k.cooling_remaining_secs != null && k.cooling_remaining_secs > 0 && (
                <span className="inline-flex items-center gap-1 text-amber-600 dark:text-amber-400">
                  <Clock className="h-3 w-3" />
                  {t("simpleConnect.poolHealth.cooldownSecs", {
                    secs: k.cooling_remaining_secs,
                    defaultValue: "{{secs}}s",
                  })}
                </span>
              )}
              {k.last_status != null && (
                <span className="font-mono text-muted-foreground">
                  HTTP {k.last_status}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </>
  );

  if (embedded) return <div className="space-y-3">{body}</div>;

  return (
    <div className="space-y-3 rounded-lg border border-border/50 bg-background/40 p-4">
      {body}
    </div>
  );
}
