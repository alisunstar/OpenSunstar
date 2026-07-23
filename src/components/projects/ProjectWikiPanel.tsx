/**
 * Project Wiki Baseline 面板
 *
 * 作为项目治理资产区块，展示在 ProjectAssetPanel 中。
 * 与 8 类 AI 资产 section 平级但独立，语义为"项目知识基线"。
 */

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  BookOpen,
  CheckCircle2,
  AlertCircle,
  Clock,
  FileText,
  Loader2,
  RefreshCw,
  PlusCircle,
  ShieldAlert,
  Bug,
  ToggleLeft,
  ToggleRight,
  GitCompare,
  Snowflake,
  FileQuestion,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  useProjectWikiScan,
  useProjectWikiInit,
  useProjectWikiLint,
  useProjectWikiChangedFiles,
} from "@/hooks/useProjectWiki";
import type { WikiScanResult, WikiLintResult, WikiChangedFilesResult } from "@/types/projectWiki";

interface ProjectWikiPanelProps {
  projectId: string;
  onConfigChanged?: () => void;
}

const STATUS_META: Record<
  string,
  { icon: typeof BookOpen; label: string; className: string }
> = {
  missing: {
    icon: BookOpen,
    label: "projectWiki.status.missing",
    className: "text-muted-foreground bg-muted/30",
  },
  scaffolded: {
    icon: Clock,
    label: "projectWiki.status.scaffolded",
    className: "text-blue-600 dark:text-blue-400 bg-blue-500/10",
  },
  effective: {
    icon: CheckCircle2,
    label: "projectWiki.status.effective",
    className: "text-green-600 dark:text-green-400 bg-green-500/10",
  },
  drifted: {
    icon: AlertCircle,
    label: "projectWiki.status.drifted",
    className: "text-amber-600 dark:text-amber-400 bg-amber-500/10",
  },
  invalid: {
    icon: ShieldAlert,
    label: "projectWiki.status.invalid",
    className: "text-red-600 dark:text-red-400 bg-red-500/10",
  },
};

const QUALITY_META: Record<string, { label: string; className: string }> = {
  "N/A": {
    label: "projectWiki.quality.NA",
    className: "text-muted-foreground",
  },
  L1: {
    label: "projectWiki.quality.L1",
    className: "text-blue-600 dark:text-blue-400",
  },
  L2: {
    label: "projectWiki.quality.L2",
    className: "text-green-600 dark:text-green-400",
  },
  L3: {
    label: "projectWiki.quality.L3",
    className: "text-purple-600 dark:text-purple-400",
  },
};

export function ProjectWikiPanel({ projectId, onConfigChanged }: ProjectWikiPanelProps) {
  const { t } = useTranslation();
  const { data, loading, refresh } = useProjectWikiScan(projectId);
  const { plan, preview, confirm, installing } = useProjectWikiInit(projectId);
  const { result: lintResult, loading: linting, lint } = useProjectWikiLint(projectId);
  const { data: changedFiles, loading: mappingLoading, refresh: refreshChangedFiles } = useProjectWikiChangedFiles(projectId);
  const [showInitConfirm, setShowInitConfirm] = useState(false);
  const [qualityMode, setQualityMode] = useState(false);

  useEffect(() => {
    if (showInitConfirm && !plan) {
      void preview();
    }
  }, [showInitConfirm, plan, preview]);

  const handleInitConfirm = async () => {
    const result = await confirm();
    if (result) {
      setShowInitConfirm(false);
      await refresh();
      onConfigChanged?.();
    }
  };

  if (loading && !data) {
    return (
      <section className="rounded-xl border border-border/60 bg-card/50 p-4">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          {t("projectWiki.loading", { defaultValue: "正在扫描 Wiki…" })}
        </div>
      </section>
    );
  }

  if (!data) {
    return null;
  }

  if (!data.exists) {
    return (
      <section className="rounded-xl border border-border/60 bg-card/50 p-4 space-y-3">
        <WikiHeader data={data} onRefresh={refresh} loading={loading} />
        <div className="flex items-center justify-between gap-3 rounded-lg border border-dashed border-border/50 p-3">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <BookOpen className="h-4 w-4" />
            {t("projectWiki.notInitialized", {
              defaultValue: "未检测到 wiki/index.md",
            })}
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setShowInitConfirm(true)}
          >
            <PlusCircle className="h-3.5 w-3.5 mr-1" />
            {t("projectWiki.actions.init", { defaultValue: "初始化 Wiki" })}
          </Button>
        </div>
        {showInitConfirm && plan && (
          <InitConfirmDialog
            plan={plan}
            installing={installing}
            onConfirm={handleInitConfirm}
            onCancel={() => setShowInitConfirm(false)}
          />
        )}
      </section>
    );
  }

  return (
    <section className="rounded-xl border border-border/60 bg-card/50 p-4 space-y-3">
      <WikiHeader
        data={data}
        onRefresh={refresh}
        loading={loading}
        onLint={() => lint(qualityMode)}
        linting={linting}
        qualityMode={qualityMode}
        onToggleQuality={() => setQualityMode((v) => !v)}
      />

      {/* 核心页面覆盖率 */}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <CoverageItem
          label={t("projectWiki.coverage.corePages", { defaultValue: "核心页面" })}
          value={`${[
            data.corePageCoverage.hasIndex && "index",
            data.corePageCoverage.hasOverview && "overview",
            data.corePageCoverage.hasSourceMap && "source-map",
            data.corePageCoverage.hasLog && "log",
            data.corePageCoverage.hasSchema && "SCHEMA",
          ]
            .filter(Boolean)
            .join(" / ")}`}
          ok={
            data.corePageCoverage.hasIndex &&
            data.corePageCoverage.hasOverview &&
            data.corePageCoverage.hasSourceMap
          }
        />
        <CoverageItem
          label={t("projectWiki.coverage.componentPages", { defaultValue: "组件" })}
          value={`${data.corePageCoverage.componentPages}`}
          ok={data.corePageCoverage.componentPages > 0}
        />
        <CoverageItem
          label={t("projectWiki.coverage.flowPages", { defaultValue: "流程" })}
          value={`${data.corePageCoverage.flowPages}`}
          ok={data.corePageCoverage.flowPages > 0}
        />
        <CoverageItem
          label={t("projectWiki.coverage.apiPages", { defaultValue: "接口" })}
          value={`${data.corePageCoverage.apiPages}`}
          ok={data.corePageCoverage.apiPages > 0}
        />
      </div>

      {/* 统计信息 */}
      <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <FileText className="h-3 w-3" />
          {t("projectWiki.pageCount", {
            defaultValue: "{{count}} 个页面",
            count: data.pageCount,
          })}
        </span>
        <span>
          {t("projectWiki.sourceRefs", {
            defaultValue: "{{count}} 个源码引用",
            count: data.sourceRefCount,
          })}
        </span>
        <span>
          {t("projectWiki.questions", {
            defaultValue: "{{count}} 个待查",
            count: data.questionCount,
          })}
        </span>
        {data.latestMtime && (
          <span>
            {t("projectWiki.lastUpdated", {
              defaultValue: "最近更新",
            })}
            : {formatTimestamp(data.latestMtime)}
          </span>
        )}
      </div>

      {lintResult && (
        <LintResultBlock result={lintResult} />
      )}

      {/* 变更文件映射 */}
      <ChangedFilesBlock
        data={changedFiles}
        loading={mappingLoading}
        onRefresh={refreshChangedFiles}
      />

      {showInitConfirm && plan && (
        <InitConfirmDialog
          plan={plan}
          installing={installing}
          onConfirm={handleInitConfirm}
          onCancel={() => setShowInitConfirm(false)}
        />
      )}
    </section>
  );
}

function WikiHeader({
  data,
  onRefresh,
  loading,
  onLint,
  linting,
  qualityMode,
  onToggleQuality,
}: {
  data: WikiScanResult;
  onRefresh: () => void;
  loading: boolean;
  onLint?: () => void;
  linting?: boolean;
  qualityMode?: boolean;
  onToggleQuality?: () => void;
}) {
  const { t } = useTranslation();
  const statusMeta = STATUS_META[data.baseStatus] ?? STATUS_META.missing;
  const qualityMeta = QUALITY_META[data.qualityLevel] ?? QUALITY_META["N/A"];
  const StatusIcon = statusMeta.icon;

  return (
    <div className="flex items-center gap-2">
      <BookOpen className="h-4 w-4 shrink-0 text-primary" />
      <h3 className="text-sm font-medium">
        {t("projectWiki.title", { defaultValue: "项目 Wiki / 知识基线" })}
      </h3>
      <span
        className={cn(
          "ml-auto flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium",
          statusMeta.className,
        )}
      >
        <StatusIcon className="h-3 w-3" />
        {t(statusMeta.label)}
      </span>
      <span className={cn("text-[10px] font-medium", qualityMeta.className)}>
        {t(qualityMeta.label)}
      </span>
      {onLint && onToggleQuality && (
        <Button
          size="icon"
          variant="ghost"
          className={cn("h-6 w-6", qualityMode && "text-primary")}
          onClick={onToggleQuality}
          aria-label={t("projectWiki.actions.toggleQuality", { defaultValue: "切换 quality 模式" })}
        >
          {qualityMode ? (
            <ToggleRight className="h-3.5 w-3.5" />
          ) : (
            <ToggleLeft className="h-3.5 w-3.5" />
          )}
        </Button>
      )}
      {onLint && (
        <Button
          size="icon"
          variant="ghost"
          className="h-6 w-6"
          onClick={onLint}
          disabled={linting}
          aria-label={t("projectWiki.actions.lint", { defaultValue: "运行 Lint" })}
        >
          {linting ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : (
            <Bug className="h-3 w-3" />
          )}
        </Button>
      )}
      <Button
        size="icon"
        variant="ghost"
        className="h-6 w-6"
        onClick={onRefresh}
        disabled={loading}
        aria-label={t("projectWiki.actions.refresh", { defaultValue: "刷新" })}
      >
        <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />
      </Button>
    </div>
  );
}

function LintResultBlock({ result }: { result: WikiLintResult }) {
  const { t } = useTranslation();
  const { summary } = result;

  if (summary.totalFiles === 0) {
    return (
      <div className="rounded-lg border border-border/40 bg-muted/10 p-2 text-xs text-muted-foreground">
        {t("projectWiki.lint.noFiles", { defaultValue: "无 Wiki 文件可检查" })}
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {/* 摘要条 */}
      <div
        className={cn(
          "flex items-center gap-2 rounded-lg border px-3 py-1.5 text-xs",
          summary.passed
            ? "border-green-500/30 bg-green-500/5 text-green-700 dark:text-green-300"
            : "border-red-500/30 bg-red-500/5 text-red-700 dark:text-red-300",
        )}
      >
        {summary.passed ? (
          <CheckCircle2 className="h-3.5 w-3.5" />
        ) : (
          <AlertCircle className="h-3.5 w-3.5" />
        )}
        <span className="font-medium">
          {summary.passed
            ? t("projectWiki.lint.passed", { defaultValue: "Lint 通过" })
            : t("projectWiki.lint.failed", {
                defaultValue: "Lint 失败：{{errors}} 个错误",
                errors: summary.errorCount,
              })}
        </span>
        <span className="text-muted-foreground">
          {t("projectWiki.lint.summary", {
            defaultValue: "{{files}} 文件 / {{errors}} 错误 / {{warnings}} 警告 / 等级 {{level}}",
            files: summary.totalFiles,
            errors: summary.errorCount,
            warnings: summary.warningCount,
            level: summary.qualityLevel,
          })}
        </span>
        {result.qualityMode && (
          <span className="ml-auto rounded-full bg-primary/10 px-1.5 py-0.5 text-[9px] font-medium text-primary">
            Quality
          </span>
        )}
      </div>

      {/* Errors 列表 */}
      {result.errors.length > 0 && (
        <div className="rounded-lg border border-red-500/20 bg-red-500/5 p-2">
          <p className="mb-1 text-[11px] font-medium text-red-600 dark:text-red-400">
            {t("projectWiki.lint.errors", { defaultValue: "错误" })}
            {" ("}{result.errors.length}{")"}
          </p>
          <ul className="space-y-0.5 text-[11px]">
            {result.errors.slice(0, 10).map((e, i) => (
              <li key={i} className="flex gap-1 text-muted-foreground">
                <span className="font-mono text-red-500 shrink-0">{e.ruleId}</span>
                <span className="truncate">{e.file}{e.line ? `:${e.line}` : ""}</span>
                <span className="truncate">{e.message}</span>
              </li>
            ))}
            {result.errors.length > 10 && (
              <li className="text-muted-foreground/60">
                …{t("projectWiki.lint.more", { defaultValue: "还有 {{count}} 条", count: result.errors.length - 10 })}
              </li>
            )}
          </ul>
        </div>
      )}

      {/* Warnings 列表 */}
      {result.warnings.length > 0 && (
        <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-2">
          <p className="mb-1 text-[11px] font-medium text-amber-600 dark:text-amber-400">
            {t("projectWiki.lint.warnings", { defaultValue: "警告" })}
            {" ("}{result.warnings.length}{")"}
          </p>
          <ul className="space-y-0.5 text-[11px]">
            {result.warnings.slice(0, 8).map((w, i) => (
              <li key={i} className="flex gap-1 text-muted-foreground">
                <span className="font-mono text-amber-500 shrink-0">{w.ruleId}</span>
                <span className="truncate">{w.file}{w.line ? `:${w.line}` : ""}</span>
                <span className="truncate">{w.message}</span>
              </li>
            ))}
            {result.warnings.length > 8 && (
              <li className="text-muted-foreground/60">
                …{t("projectWiki.lint.more", { defaultValue: "还有 {{count}} 条", count: result.warnings.length - 8 })}
              </li>
            )}
          </ul>
        </div>
      )}
    </div>
  );
}

function ChangedFilesBlock({
  data,
  loading,
  onRefresh,
}: {
  data: WikiChangedFilesResult | null;
  loading: boolean;
  onRefresh: () => void;
}) {
  const { t } = useTranslation();

  if (!data) return null;

  return (
    <div className="rounded-lg border border-border/40 bg-muted/10 p-2 space-y-1.5">
      <div className="flex items-center gap-1.5 text-[11px] font-medium">
        <GitCompare className="h-3 w-3" />
        {t("projectWiki.changed.title", { defaultValue: "变更映射" })}
        <Button
          size="icon"
          variant="ghost"
          className="h-5 w-5"
          onClick={onRefresh}
          disabled={loading}
        >
          {loading ? (
            <Loader2 className="h-2.5 w-2.5 animate-spin" />
          ) : (
            <RefreshCw className="h-2.5 w-2.5" />
          )}
        </Button>
      </div>

      {data.coldStart ? (
        <div className="flex items-start gap-1.5 text-[11px] text-amber-600 dark:text-amber-400">
          <Snowflake className="h-3 w-3 mt-0.5 shrink-0" />
          <div>
            <p className="font-medium">
              {t("projectWiki.changed.coldStart", { defaultValue: "冷启动态" })}
              {" ("}{data.effectiveSourcePages}/{data.threshold}{")"}
            </p>
            {data.guidance && (
              <p className="text-muted-foreground mt-0.5">{data.guidance}</p>
            )}
          </div>
        </div>
      ) : (
        <div className="space-y-1 text-[11px]">
          <div className="flex gap-3 text-muted-foreground">
            <span>
              {t("projectWiki.changed.changedFiles", { defaultValue: "变更" })}
              : {data.changedFiles.length}
            </span>
            <span>
              {t("projectWiki.changed.affectedPages", { defaultValue: "影响页面" })}
              : {data.affectedPages.length}
            </span>
            <span>
              {t("projectWiki.changed.unmapped", { defaultValue: "未映射" })}
              : {data.unmappedChangedFiles.length}
            </span>
          </div>
          {data.affectedPages.length > 0 && (
            <ul className="space-y-0.5">
              {data.affectedPages.slice(0, 5).map((p, i) => (
                <li key={i} className="flex items-center gap-1 text-muted-foreground">
                  <span className="text-green-500 shrink-0">→</span>
                  <span className="truncate">{p}</span>
                </li>
              ))}
            </ul>
          )}
          {data.unmappedChangedFiles.length > 0 && (
            <ul className="space-y-0.5">
              {data.unmappedChangedFiles.slice(0, 3).map((f, i) => (
                <li key={i} className="flex items-center gap-1 text-muted-foreground/70">
                  <FileQuestion className="h-2.5 w-2.5 shrink-0 text-amber-500" />
                  <span className="truncate">{f}</span>
                </li>
              ))}
              {data.unmappedChangedFiles.length > 3 && (
                <li className="text-muted-foreground/50">
                  …{t("projectWiki.lint.more", { defaultValue: "还有 {{count}} 条", count: data.unmappedChangedFiles.length - 3 })}
                </li>
              )}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

function CoverageItem({
  label,
  value,
  ok,
}: {
  label: string;
  value: string;
  ok: boolean;
}) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border/40 px-2 py-1">
      <span className="text-muted-foreground">{label}</span>
      <span
        className={cn(
          "font-mono text-[11px]",
          ok ? "text-green-600 dark:text-green-400" : "text-muted-foreground",
        )}
      >
        {value}
      </span>
    </div>
  );
}

function InitConfirmDialog({
  plan,
  installing,
  onConfirm,
  onCancel,
}: {
  plan: NonNullable<ReturnType<typeof useProjectWikiInit>["plan"]>;
  installing: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();

  if (plan.audit.blocked) {
    return (
      <div className="rounded-lg border border-red-500/30 bg-red-500/5 p-3 text-xs">
        <div className="flex items-center gap-1 text-red-600 dark:text-red-400 font-medium">
          <ShieldAlert className="h-3.5 w-3.5" />
          {t("projectWiki.init.blocked", {
            defaultValue: "安全审计阻断",
          })}
        </div>
        <ul className="mt-1 space-y-0.5 text-muted-foreground">
          {plan.audit.warnings.map((w, i) => (
            <li key={i}>• {w}</li>
          ))}
        </ul>
        <Button size="sm" variant="ghost" className="mt-2 h-6" onClick={onCancel}>
          {t("common.close", { defaultValue: "关闭" })}
        </Button>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border/60 bg-muted/20 p-3 space-y-2">
      <p className="text-xs font-medium">
        {t("projectWiki.init.previewTitle", { defaultValue: "Wiki 初始化预检" })}
      </p>
      <div className="flex gap-3 text-xs text-muted-foreground">
        <span>
          {t("projectWiki.init.willCreate", {
            defaultValue: "将创建 {{count}} 个文件",
            count: plan.files.filter((f) => f.willCreate).length,
          })}
        </span>
        {plan.files.some((f) => f.alreadyExists) && (
          <span>
            {t("projectWiki.init.alreadyExists", {
              defaultValue: "{{count}} 个文件已存在（将跳过）",
              count: plan.files.filter((f) => f.alreadyExists).length,
            })}
          </span>
        )}
      </div>
      <div className="flex gap-2">
        <Button size="sm" onClick={onConfirm} disabled={installing}>
          {installing ? (
            <Loader2 className="h-3.5 w-3.5 mr-1 animate-spin" />
          ) : (
            <CheckCircle2 className="h-3.5 w-3.5 mr-1" />
          )}
          {t("projectWiki.init.confirm", { defaultValue: "确认初始化" })}
        </Button>
        <Button size="sm" variant="ghost" onClick={onCancel} disabled={installing}>
          {t("common.cancel", { defaultValue: "取消" })}
        </Button>
      </div>
    </div>
  );
}

function formatTimestamp(ts: number): string {
  const date = new Date(ts * 1000);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffMin < 1) return "刚刚";
  if (diffMin < 60) return `${diffMin} 分钟前`;
  if (diffHour < 24) return `${diffHour} 小时前`;
  if (diffDay < 30) return `${diffDay} 天前`;
  return date.toLocaleDateString();
}
