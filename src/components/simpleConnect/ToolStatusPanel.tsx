import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Circle, Loader2, RefreshCw, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  simpleConnectApi,
  type ToolConfigStatus,
} from "@/lib/api/simpleConnect";
import { TOOL_LABELS } from "./constants";
import { cn } from "@/lib/utils";

interface ToolStatusPanelProps {
  refreshToken?: number;
  selectedTool?: string;
  onSelectTool?: (tool: string) => void;
  embedded?: boolean;
}

export function ToolStatusPanel({
  refreshToken = 0,
  selectedTool,
  onSelectTool,
  embedded = false,
}: ToolStatusPanelProps) {
  const { t } = useTranslation();
  const [items, setItems] = useState<ToolConfigStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [clearing, setClearing] = useState<string | null>(null);

  const configuredCount = useMemo(
    () => items.filter((i) => i.configured).length,
    [items],
  );

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await simpleConnectApi.listToolStatus();
      setItems(list);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load, refreshToken]);

  const handleClear = async (tool: string) => {
    setClearing(tool);
    try {
      await simpleConnectApi.clear(tool);
      toast.success(
        t("simpleConnect.toolCleared", {
          tool: TOOL_LABELS[tool] ?? tool,
          defaultValue: "{{tool}} 已清除 Simple Connect 配置",
        }),
      );
      await load();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setClearing(null);
    }
  };

  const header = embedded ? (
    <div className="flex justify-end">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className="h-8 w-8 p-0"
        disabled={loading}
        onClick={() => void load()}
        aria-label={t("common.refresh", { defaultValue: "刷新" })}
      >
        <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
      </Button>
    </div>
  ) : (
    <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium">
            {t("simpleConnect.toolStatusTitle", {
              defaultValue: "CLI 配置状态",
            })}
          </p>
          {!loading && (
            <Badge variant="secondary" className="font-normal">
              {t("simpleConnect.toolStatusCount", {
                configured: configuredCount,
                total: items.length,
                defaultValue: "{{configured}}/{{total}} 已配置",
              })}
            </Badge>
          )}
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 gap-1.5"
          disabled={loading}
          onClick={() => void load()}
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          {t("common.refresh", { defaultValue: "刷新" })}
        </Button>
      </div>
  );

  const list = loading ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground py-4 justify-center">
          <Loader2 className="h-4 w-4 animate-spin" />
          {t("simpleConnect.toolStatusLoading", { defaultValue: "读取 CLI 状态…" })}
        </div>
  ) : (
        <ul className="grid gap-2 sm:grid-cols-2">
          {items.map((item) => {
            const selected = selectedTool === item.tool;
            return (
              <li
                key={item.tool}
                className={cn(
                  "flex items-center justify-between gap-2 rounded-lg border px-3 py-2.5 text-sm transition-colors",
                  selected
                    ? "border-primary/40 bg-primary/5"
                    : "border-border/40 bg-background/50",
                  onSelectTool && "cursor-pointer hover:bg-muted/30",
                )}
                onClick={() => onSelectTool?.(item.tool)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    onSelectTool?.(item.tool);
                  }
                }}
                role={onSelectTool ? "button" : undefined}
                tabIndex={onSelectTool ? 0 : undefined}
              >
                <div className="flex items-center gap-2 min-w-0">
                  {item.configured ? (
                    <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-500" />
                  ) : (
                    <Circle className="h-4 w-4 shrink-0 text-muted-foreground/60" />
                  )}
                  <div className="min-w-0">
                    <span className="font-medium block truncate">
                      {TOOL_LABELS[item.tool] ?? item.tool}
                    </span>
                    {item.configured && (
                      <p className="text-[11px] text-muted-foreground truncate">
                        {item.model ?? item.base_url ?? item.key_hint ?? "—"}
                      </p>
                    )}
                  </div>
                </div>
                {item.configured && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 shrink-0"
                    disabled={clearing === item.tool}
                    onClick={(e) => {
                      e.stopPropagation();
                      void handleClear(item.tool);
                    }}
                    aria-label={t("simpleConnect.clearTool", {
                      tool: TOOL_LABELS[item.tool] ?? item.tool,
                      defaultValue: "清除 {{tool}}",
                    })}
                  >
                    {clearing === item.tool ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Trash2 className="h-4 w-4" />
                    )}
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
  );

  if (embedded) {
    return (
      <div className="space-y-3">
        {header}
        {list}
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-border/60 bg-muted/10 p-4 space-y-3">
      {header}
      {list}
    </div>
  );
}
