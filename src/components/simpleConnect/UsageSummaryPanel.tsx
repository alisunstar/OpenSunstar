import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BarChart3, Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  simpleConnectApi,
  type SimpleConnectUsageSummary,
} from "@/lib/api/simpleConnect";

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function UsageSummaryPanel({ embedded = false }: { embedded?: boolean }) {
  const { t } = useTranslation();
  const [summary, setSummary] = useState<SimpleConnectUsageSummary | null>(
    null,
  );
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await simpleConnectApi.usageSummary();
      setSummary(data);
    } catch {
      setSummary(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const body = (
    <>
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-sm font-medium">
          <BarChart3 className="h-4 w-4 text-primary" />
          {t("simpleConnect.usage.title", { defaultValue: "用量概览（只读）" })}
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8"
          disabled={loading}
          onClick={() => void refresh()}
        >
          {loading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RefreshCw className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>

      {loading && !summary ? (
        <div className="flex justify-center py-6">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : summary ? (
        <>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <StatCard
              label={t("simpleConnect.usage.input", { defaultValue: "输入" })}
              value={formatTokens(summary.total_input_tokens)}
            />
            <StatCard
              label={t("simpleConnect.usage.output", {
                defaultValue: "输出",
              })}
              value={formatTokens(summary.total_output_tokens)}
            />
            <StatCard
              label={t("simpleConnect.usage.sessions", {
                defaultValue: "会话条数",
              })}
              value={String(summary.record_count)}
            />
            <StatCard
              label={t("simpleConnect.usage.proxyPort", {
                port: summary.proxy_port,
                defaultValue: "代理 :{{port}}",
              })}
              value={formatTokens(
                summary.proxy_session_input + summary.proxy_session_output,
              )}
              hint={t("simpleConnect.usage.proxySession", {
                defaultValue: "本次会话",
              })}
            />
          </div>

          {summary.by_tool.length > 0 && (
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground">
                {t("simpleConnect.usage.byTool", {
                  defaultValue: "按 CLI 拆分",
                })}
              </p>
              <ul className="space-y-1.5">
                {summary.by_tool.map((row) => (
                  <li
                    key={row.tool}
                    className="flex items-center justify-between gap-2 text-xs rounded-md border border-border/40 px-3 py-2"
                  >
                    <span>{row.tool}</span>
                    <span className="text-muted-foreground font-mono">
                      {formatTokens(row.input_tokens)} /{" "}
                      {formatTokens(row.output_tokens)} · {row.records}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          <p className="text-[11px] text-muted-foreground">{summary.note}</p>
        </>
      ) : (
        <p className="text-xs text-muted-foreground py-4 text-center">
          {t("simpleConnect.usage.empty", {
            defaultValue: "暂无本地会话用量数据",
          })}
        </p>
      )}
    </>
  );

  if (embedded) return <div className="space-y-4">{body}</div>;

  return (
    <div className="space-y-4 rounded-xl border border-border/60 bg-muted/10 p-5">
      {body}
    </div>
  );
}

function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="rounded-lg border border-border/40 bg-background/50 px-3 py-2.5">
      <p className="text-[11px] text-muted-foreground">{label}</p>
      <p className="text-lg font-semibold tabular-nums">{value}</p>
      {hint && (
        <Badge variant="secondary" className="mt-1 text-[10px] font-normal">
          {hint}
        </Badge>
      )}
    </div>
  );
}
