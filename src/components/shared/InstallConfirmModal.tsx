import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  FilePlus,
  FileX,
  ShieldAlert,
  ShieldCheck,
  XCircle,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type {
  InstallFileEntry,
  InstallAuditSummary,
} from "@/lib/api/designContract";
import { cn } from "@/lib/utils";

interface InstallConfirmModalProps {
  open: boolean;
  files: InstallFileEntry[];
  audit: InstallAuditSummary;
  title?: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

const SEVERITY_COLORS: Record<string, string> = {
  CRITICAL: "bg-red-500/20 text-red-600 dark:text-red-400 border-red-500/30",
  HIGH: "bg-orange-500/20 text-orange-600 dark:text-orange-400 border-orange-500/30",
  MEDIUM: "bg-amber-500/20 text-amber-600 dark:text-amber-400 border-amber-500/30",
  LOW: "bg-blue-500/20 text-blue-600 dark:text-blue-400 border-blue-500/30",
  INFO: "bg-muted text-muted-foreground border-border/50",
};

export function InstallConfirmModal({
  open,
  files,
  audit,
  title,
  confirmLabel,
  onConfirm,
  onCancel,
}: InstallConfirmModalProps) {
  const { t } = useTranslation();
  const [expandedFile, setExpandedFile] = useState<string | null>(null);

  if (!open) return null;

  const willCreate = files.filter((f) => f.status === "create");
  const willOverwrite = files.filter((f) => f.status === "overwrite");
  const willSkip = files.filter((f) => f.status === "skip");
  const hasBlocked = audit.blocked;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border border-border rounded-xl shadow-2xl w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="px-5 py-4 border-b border-border/60 flex items-center justify-between shrink-0">
          <h3 className="text-sm font-semibold text-foreground">
            {title || t("installConfirm.title", { defaultValue: "确认安装到项目" })}
          </h3>
          <Button variant="ghost" size="sm" onClick={onCancel} className="h-7 px-2">
            <XCircle className="w-4 h-4" />
          </Button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
          {/* Audit Summary */}
          <div
            className={cn(
              "rounded-lg border p-3 space-y-2",
              hasBlocked
                ? "border-red-500/40 bg-red-500/5"
                : audit.totalFindings > 0
                  ? "border-amber-500/30 bg-amber-500/5"
                  : "border-emerald-500/20 bg-emerald-500/5",
            )}
          >
            <div className="flex items-center gap-2">
              {hasBlocked ? (
                <ShieldAlert className="w-4 h-4 text-red-500" />
              ) : audit.totalFindings > 0 ? (
                <AlertTriangle className="w-4 h-4 text-amber-500" />
              ) : (
                <ShieldCheck className="w-4 h-4 text-emerald-500" />
              )}
              <span className="text-xs font-medium text-foreground">
                {t("installConfirm.auditScan", { defaultValue: "安全审计扫描" })}
              </span>
              <span className="text-[10px] text-muted-foreground">
                ({audit.filesScanned}{" "}
                {t("installConfirm.filesScanned", { defaultValue: "文件已扫描" })})
              </span>
            </div>

            {audit.totalFindings === 0 ? (
              <p className="text-[11px] text-emerald-600 dark:text-emerald-400">
                {t("installConfirm.noFindings", {
                  defaultValue: "未发现安全问题。",
                })}
              </p>
            ) : (
              <div className="space-y-1">
                <div className="flex items-center gap-2 flex-wrap">
                  {audit.critical > 0 && (
                    <span
                      className={cn(
                        "text-[10px] px-1.5 py-0.5 rounded border",
                        SEVERITY_COLORS.CRITICAL,
                      )}
                    >
                      CRITICAL: {audit.critical}
                    </span>
                  )}
                  {audit.high > 0 && (
                    <span
                      className={cn(
                        "text-[10px] px-1.5 py-0.5 rounded border",
                        SEVERITY_COLORS.HIGH,
                      )}
                    >
                      HIGH: {audit.high}
                    </span>
                  )}
                  {audit.medium > 0 && (
                    <span
                      className={cn(
                        "text-[10px] px-1.5 py-0.5 rounded border",
                        SEVERITY_COLORS.MEDIUM,
                      )}
                    >
                      MEDIUM: {audit.medium}
                    </span>
                  )}
                  {audit.low > 0 && (
                    <span
                      className={cn(
                        "text-[10px] px-1.5 py-0.5 rounded border",
                        SEVERITY_COLORS.LOW,
                      )}
                    >
                      LOW: {audit.low}
                    </span>
                  )}
                </div>
                {audit.findings.slice(0, 5).map((f, i) => (
                  <div
                    key={i}
                    className="text-[10px] text-muted-foreground flex items-start gap-1"
                  >
                    <span
                      className={cn(
                        "shrink-0 px-1 rounded text-[9px] font-medium",
                        SEVERITY_COLORS[f.severity] || SEVERITY_COLORS.INFO,
                      )}
                    >
                      {f.severity}
                    </span>
                    <span className="truncate">
                      {f.file}:{f.ruleId} — {f.message}
                    </span>
                  </div>
                ))}
                {audit.findings.length > 5 && (
                  <p className="text-[10px] text-muted-foreground italic">
                    ... +{audit.findings.length - 5} more
                  </p>
                )}
              </div>
            )}

            {hasBlocked && (
              <p className="text-[11px] text-red-600 dark:text-red-400 font-medium">
                {t("installConfirm.auditBlocked", {
                  defaultValue:
                    "安全审计发现 CRITICAL 级别问题，安装已被阻断。请修复后重试。",
                })}
              </p>
            )}
          </div>

          {/* File List */}
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-foreground">
                {t("installConfirm.fileList", { defaultValue: "文件清单" })}
              </span>
              <span className="text-[10px] text-muted-foreground">
                ({willCreate.length}{" "}
                {t("installConfirm.toCreate", { defaultValue: "将创建" })},{" "}
                {willOverwrite.length}{" "}
                {t("installConfirm.toOverwrite", { defaultValue: "将覆盖" })},{" "}
                {willSkip.length}{" "}
                {t("installConfirm.toSkip", { defaultValue: "已存在跳过" })})
              </span>
            </div>

            <div className="space-y-1">
              {files.map((f) => (
                <div key={f.path} className="rounded border border-border/60">
                  <button
                    className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-muted/30 transition-colors"
                    onClick={() =>
                      setExpandedFile(expandedFile === f.path ? null : f.path)
                    }
                  >
                    {f.status === "create" ? (
                      <FilePlus className="w-3.5 h-3.5 text-emerald-500 shrink-0" />
                    ) : f.status === "overwrite" ? (
                      <AlertTriangle className="w-3.5 h-3.5 text-amber-500 shrink-0" />
                    ) : (
                      <FileX className="w-3.5 h-3.5 text-muted-foreground/50 shrink-0" />
                    )}
                    <span className="text-xs font-mono text-foreground truncate flex-1">
                      {f.path}
                    </span>
                    <span
                      className={cn(
                        "text-[10px] px-1.5 py-0.5 rounded shrink-0",
                        f.status === "create"
                          ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                          : f.status === "overwrite"
                            ? "bg-amber-500/10 text-amber-700 dark:text-amber-400"
                          : "bg-muted text-muted-foreground",
                      )}
                    >
                      {f.status === "create"
                        ? t("installConfirm.create", { defaultValue: "创建" })
                        : f.status === "overwrite"
                          ? t("installConfirm.overwrite", { defaultValue: "覆盖" })
                        : t("installConfirm.skip", { defaultValue: "跳过" })}
                    </span>
                    {f.newContent && (
                      expandedFile === f.path ? (
                        <ChevronDown className="w-3 h-3 text-muted-foreground shrink-0" />
                      ) : (
                        <ChevronRight className="w-3 h-3 text-muted-foreground shrink-0" />
                      )
                    )}
                  </button>
                  {expandedFile === f.path && f.newContent && (
                    <div className="border-t border-border/40 px-3 py-2">
                      {f.existingContent && (
                        <div className="mb-2">
                          <span className="text-[9px] uppercase font-medium text-muted-foreground">
                            {t("installConfirm.existing", { defaultValue: "现有内容" })}
                          </span>
                          <pre className="text-[10px] font-mono text-muted-foreground bg-muted/30 rounded p-2 mt-1 max-h-32 overflow-auto whitespace-pre-wrap">
                            {f.existingContent.slice(0, 500)}
                            {f.existingContent.length > 500 && "..."}
                          </pre>
                        </div>
                      )}
                      <div>
                          <span className="text-[9px] uppercase font-medium text-muted-foreground">
                          {f.status === "overwrite"
                            ? t("installConfirm.willOverwrite", {
                                defaultValue: "将覆盖为",
                              })
                            : f.existingContent
                            ? t("installConfirm.wouldOverwrite", {
                                defaultValue: "将覆盖为 (当前跳过)",
                              })
                            : t("installConfirm.newContent", {
                                defaultValue: "新内容",
                              })}
                        </span>
                        <pre className="text-[10px] font-mono text-foreground bg-muted/20 rounded p-2 mt-1 max-h-48 overflow-auto whitespace-pre-wrap">
                          {f.newContent.slice(0, 1000)}
                          {f.newContent.length > 1000 && "..."}
                        </pre>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-border/60 flex items-center justify-end gap-2 shrink-0">
          <Button variant="outline" size="sm" onClick={onCancel}>
            {t("installConfirm.cancel", { defaultValue: "取消" })}
          </Button>
          <Button
            size="sm"
            onClick={onConfirm}
            disabled={hasBlocked}
            className={cn(
              "flex items-center gap-1.5",
              hasBlocked && "opacity-50 cursor-not-allowed",
            )}
          >
            <CheckCircle2 className="w-3.5 h-3.5" />
            {confirmLabel ??
              t("installConfirm.confirmInstall", {
                defaultValue: "确认安装",
              })}
          </Button>
        </div>
      </div>
    </div>
  );
}
