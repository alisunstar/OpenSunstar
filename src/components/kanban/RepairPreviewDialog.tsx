import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, ChevronDown, ChevronRight, Shield } from "lucide-react";

import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { RepairPreviewItem, RepairPreviewResult } from "@/api/aiInsight";

export interface RepairPreviewDialogProps {
  open: boolean;
  loading: boolean;
  preview: RepairPreviewResult | null;
  projectName: string;
  repairing: boolean;
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onToggleItem: (checkName: string) => void;
  onConfirm: (selectedNames: string[]) => void;
  onCancel: () => void;
  selectedNames: Set<string>;
}

export function RepairPreviewDialog({
  open,
  loading,
  preview,
  projectName,
  repairing,
  onSelectAll,
  onDeselectAll,
  onToggleItem,
  onConfirm,
  onCancel,
  selectedNames,
}: RepairPreviewDialogProps) {
  const { t } = useTranslation();
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());

  const items = preview?.items ?? [];
  const allSelected = items.length > 0 && selectedNames.size === items.length;
  const noneSelected = selectedNames.size === 0;
  const safetyCount = useMemo(
    () => items.filter((i) => i.is_safety_critical).length,
    [items],
  );

  const toggleExpand = useCallback((name: string) => {
    setExpandedItems((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }, []);

  // reset expanded on open
  useEffect(() => {
    if (open) setExpandedItems(new Set());
  }, [open]);

  const confirmDisabled = noneSelected || repairing;

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onCancel()}>
      <DialogContent zIndex="top" className="max-w-xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>
            {t("repairPreview.title", {
              defaultValue: "配置修复预览",
            })}
          </DialogTitle>
          <DialogDescription>
            {t("repairPreview.description", {
              name: projectName,
              count: preview?.total_drifted ?? 0,
              defaultValue:
                "项目「{{name}}」共 {{count}} 项配置不一致，勾选需要修复的项：",
            })}
          </DialogDescription>
        </DialogHeader>

        {/* body */}
        <div className="flex-1 overflow-y-auto min-h-0 px-1">
          {loading && (
            <div className="flex items-center justify-center py-10 text-muted-foreground text-sm">
              {t("repairPreview.loading", {
                defaultValue: "正在扫描待处理项…",
              })}
            </div>
          )}

          {!loading && items.length === 0 && (
            <div className="flex items-center justify-center py-10 text-muted-foreground text-sm">
              {t("repairPreview.empty", {
                defaultValue: "未发现配置不一致，无需修复",
              })}
            </div>
          )}

          {!loading && items.length > 0 && (
            <div className="space-y-2 py-2">
              {/* toolbar */}
              <div className="flex items-center gap-3 text-xs text-muted-foreground mb-2">
                <button
                  type="button"
                  className="hover:text-foreground underline underline-offset-2"
                  onClick={allSelected ? onDeselectAll : onSelectAll}
                >
                  {allSelected
                    ? t("repairPreview.deselectAll", {
                        defaultValue: "取消全选",
                      })
                    : t("repairPreview.selectAll", {
                        defaultValue: "全选",
                      })}
                </button>
                {safetyCount > 0 && (
                  <span className="flex items-center gap-1 text-amber-600">
                    <Shield className="h-3 w-3" />
                    {t("repairPreview.safetyHint", {
                      count: safetyCount,
                      defaultValue:
                        "{{count}} 项为安全关键项（Ignore/Permissions/Hooks），修复将覆盖访问控制或命令执行配置",
                    })}
                  </span>
                )}
              </div>

              {/* items */}
              {items.map((item) => (
                <RepairPreviewItemRow
                  key={item.check_name}
                  item={item}
                  checked={selectedNames.has(item.check_name)}
                  expanded={expandedItems.has(item.check_name)}
                  onToggle={() => onToggleItem(item.check_name)}
                  onToggleExpand={() => toggleExpand(item.check_name)}
                />
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onCancel} disabled={repairing}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button
            variant="destructive"
            disabled={confirmDisabled}
            onClick={() => onConfirm(Array.from(selectedNames))}
          >
            {repairing
              ? t("repairPreview.repairing", {
                  defaultValue: "修复中…",
                })
              : t("repairPreview.confirmAction", {
                  count: selectedNames.size,
                  defaultValue: `修复选中项（${selectedNames.size}）`,
                })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/* ─── single row ──────────────────────────────────── */

function RepairPreviewItemRow({
  item,
  checked,
  expanded,
  onToggle,
  onToggleExpand,
}: {
  item: RepairPreviewItem;
  checked: boolean;
  expanded: boolean;
  onToggle: () => void;
  onToggleExpand: () => void;
}) {
  const { t } = useTranslation();
  const hasContent = item.current_content.length > 0;

  return (
    <div
      className={[
        "rounded-md border px-3 py-2 text-sm transition-colors",
        item.is_safety_critical
          ? "border-amber-400/60 bg-amber-50 dark:bg-amber-950/20"
          : "border-border-default bg-card",
      ].join(" ")}
    >
      <div className="flex items-center gap-2">
        <Checkbox
          checked={checked}
          onCheckedChange={onToggle}
          aria-label={item.label}
        />
        <span className="font-medium flex-1">{item.label}</span>
        {item.is_safety_critical && (
          <span
            className="flex items-center gap-1 text-[11px] text-amber-600 dark:text-amber-400"
            title={t("repairPreview.safetyCriticalTooltip", {
              defaultValue:
                "安全关键项：修复将覆盖访问控制或命令执行相关配置",
            })}
          >
            <AlertTriangle className="h-3 w-3" />
            {t("repairPreview.safetyBadge", { defaultValue: "安全关键" })}
          </span>
        )}
        {hasContent && (
          <button
            type="button"
            className="text-muted-foreground hover:text-foreground p-0.5"
            onClick={onToggleExpand}
            aria-label={expanded ? "收起" : "展开当前内容"}
          >
            {expanded ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
          </button>
        )}
      </div>

      {item.effective_detail && (
        <p className="mt-1 text-xs text-muted-foreground ml-6">
          {item.effective_detail}
        </p>
      )}

      {expanded && hasContent && (
        <pre className="mt-2 ml-6 p-2 rounded bg-muted text-[11px] leading-relaxed overflow-x-auto max-h-48 whitespace-pre-wrap break-all">
          {item.current_content}
        </pre>
      )}

      {item.live_path && (
        <p className="mt-1 text-[11px] text-muted-foreground/70 ml-6 truncate">
          {item.live_path}
        </p>
      )}
    </div>
  );
}
