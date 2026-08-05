/**
 * Project Wiki Baseline 面板
 *
 * 作为项目治理资产区块，展示在 ProjectAssetPanel 中。
 * 与 8 类 AI 资产 section 平级但独立，语义为"项目知识基线"。
 */

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  BookOpen,
  CheckCircle2,
  AlertCircle,
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
  PackageOpen,
  Download,
  Clock3,
  Eye,
  FolderOpen,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ProjectWikiViewer } from "@/components/projects/ProjectWikiViewer";
import { useAIConfig } from "@/hooks/useAIConfig";
import { cn } from "@/lib/utils";
import { projectWikiApi } from "@/lib/api/projectWiki";
import { flowOrchestratorApi } from "@/lib/api/flowOrchestrator";
import {
  useProjectWikiScan,
  useProjectWikiInit,
  useProjectWikiLint,
  useProjectWikiChangedFiles,
  useProjectWikiAcceptance,
  useProjectWikiCandidates,
  useProjectWikiComparison,
  useProjectWikiGenerator,
  useProjectWikiDocument,
} from "@/hooks/useProjectWiki";
import type {
  WikiCandidate,
  WikiComparisonReport,
  WikiScanResult,
  WikiLintResult,
  WikiChangedFilesResult,
  WikiLifecycle,
} from "@/types/projectWiki";

interface ProjectWikiPanelProps {
  projectId: string;
  onConfigChanged?: () => void;
  onOpenAiProviderSettings?: () => void;
}

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

const LIFECYCLE_META: Record<
  WikiLifecycle["phase"],
  { label: string; className: string }
> = {
  uninitialized: {
    label: "未初始化",
    className: "text-muted-foreground bg-muted/30",
  },
  pendingGeneration: {
    label: "待生成",
    className: "text-blue-600 dark:text-blue-400 bg-blue-500/10",
  },
  generating: {
    label: "正在生成",
    className: "text-blue-600 dark:text-blue-400 bg-blue-500/10",
  },
  pendingAcceptance: {
    label: "待验收",
    className: "text-amber-600 dark:text-amber-400 bg-amber-500/10",
  },
  syncedToCommit: {
    label: "已同步到 Commit",
    className: "text-green-600 dark:text-green-400 bg-green-500/10",
  },
  syncedToSnapshot: {
    label: "已同步到源码快照",
    className: "text-green-600 dark:text-green-400 bg-green-500/10",
  },
  changesDetected: {
    label: "检测到变更",
    className: "text-amber-600 dark:text-amber-400 bg-amber-500/10",
  },
  pendingSync: {
    label: "待同步",
    className: "text-amber-600 dark:text-amber-400 bg-amber-500/10",
  },
  syncing: {
    label: "同步中",
    className: "text-blue-600 dark:text-blue-400 bg-blue-500/10",
  },
  updated: {
    label: "已更新",
    className: "text-green-600 dark:text-green-400 bg-green-500/10",
  },
  failed: {
    label: "操作失败",
    className: "text-red-600 dark:text-red-400 bg-red-500/10",
  },
};

export function ProjectWikiPanel({
  projectId,
  onConfigChanged,
  onOpenAiProviderSettings,
}: ProjectWikiPanelProps) {
  const { t } = useTranslation();
  const { data, loading, refresh } = useProjectWikiScan(projectId);
  const [rdLoopActive, setRdLoopActive] = useState(false);
  useEffect(() => {
    let cancelled = false;
    flowOrchestratorApi
      .scanProject(projectId)
      .then((idx) => {
        if (!cancelled)
          setRdLoopActive(idx.savedProfile?.presetId === "rd-loop");
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [projectId]);
  const { plan, preview, confirm, installing } = useProjectWikiInit(projectId);
  const {
    result: lintResult,
    loading: linting,
    lint,
  } = useProjectWikiLint(projectId);
  const {
    data: changedFiles,
    loading: mappingLoading,
    refresh: refreshChangedFiles,
  } = useProjectWikiChangedFiles(projectId);
  const { loading: accepting, accept } = useProjectWikiAcceptance(projectId);
  const {
    data: candidates,
    loading: candidatesLoading,
    importingId,
    refresh: refreshCandidates,
    importCandidate,
  } = useProjectWikiCandidates(projectId);
  const {
    data: comparison,
    loading: comparisonLoading,
    compare: compareCandidates,
  } = useProjectWikiComparison(projectId);
  const { loading: generatorLoading, generate } =
    useProjectWikiGenerator(projectId);
  const {
    data: wikiDocument,
    loading: documentLoading,
    error: documentError,
    open: loadDocument,
    close: closeDocument,
  } = useProjectWikiDocument(projectId);
  const { aiConfigured, aiConfigLoading } = useAIConfig();
  const [showInitConfirm, setShowInitConfirm] = useState(false);
  const [showSnapshotConfirm, setShowSnapshotConfirm] = useState(false);
  const [qualityMode, setQualityMode] = useState(false);
  const [viewerTarget, setViewerTarget] = useState<{
    candidateId?: string;
    title: string;
  } | null>(null);
  const [openingWikiFolder, setOpeningWikiFolder] = useState(false);

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

  const handleAccept = async () => {
    const lifecycle = await accept();
    if (lifecycle) {
      await refresh();
      onConfigChanged?.();
      return true;
    }
    return false;
  };

  const requestAccept = () => {
    if (
      !data?.sourceBaseline.hasGitCommit &&
      !data?.sourceBaseline.snapshotSha256
    ) {
      setShowSnapshotConfirm(true);
      return;
    }
    void handleAccept();
  };

  const handleImportCandidate = async (candidateId: string) => {
    const result = await importCandidate(candidateId);
    if (result) {
      await refresh();
      onConfigChanged?.();
      return true;
    }
    return false;
  };

  const handleGenerateCandidate = async () => {
    const result = await generate("builtin");
    if (result) {
      await Promise.all([refresh(), refreshCandidates()]);
      onConfigChanged?.();
      return true;
    }
    await refresh();
    return false;
  };

  const handleOpenDocument = (candidateId?: string) => {
    setViewerTarget({
      candidateId,
      title: candidateId ? "候选 Wiki 预览" : "项目 Wiki",
    });
    void loadDocument(candidateId);
  };

  const handleOpenWikiFolder = async () => {
    setOpeningWikiFolder(true);
    try {
      await projectWikiApi.openFolder(projectId);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "打开 Wiki 目录失败",
      );
    } finally {
      setOpeningWikiFolder(false);
    }
  };

  const handleViewerOpenChange = (open: boolean) => {
    if (!open) {
      setViewerTarget(null);
      closeDocument();
    }
  };

  const viewer = (
    <ProjectWikiViewer
      open={viewerTarget !== null}
      title={viewerTarget?.title ?? "项目 Wiki"}
      document={wikiDocument}
      loading={documentLoading}
      error={documentError}
      onOpenChange={handleViewerOpenChange}
    />
  );

  const snapshotDialog = data && (
    <SourceSnapshotBaselineDialog
      open={showSnapshotConfirm}
      fileCount={data.sourceBaseline.snapshotFileCount}
      accepting={accepting}
      onOpenChange={setShowSnapshotConfirm}
      onConfirm={async () => {
        if (await handleAccept()) {
          setShowSnapshotConfirm(false);
        }
      }}
    />
  );

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
    const needsInitialization = data.lifecycle.phase === "uninitialized";
    return (
      <>
        <section className="rounded-xl border border-border/60 bg-card/50 p-4 space-y-3">
          <WikiHeader data={data} onRefresh={refresh} loading={loading} />
          {!needsInitialization && (
            <CandidateBlock
              lifecycle={data.lifecycle}
              candidates={candidates}
              loading={candidatesLoading}
              importingId={importingId}
              onRefresh={refreshCandidates}
              onImport={handleImportCandidate}
              comparison={comparison}
              comparisonLoading={comparisonLoading}
              onCompare={() =>
                compareCandidates(candidates.map((candidate) => candidate.id))
              }
              generating={generatorLoading}
              onGenerate={handleGenerateCandidate}
              aiConfigured={aiConfigured}
              aiConfigLoading={aiConfigLoading}
              onOpenAiProviderSettings={onOpenAiProviderSettings}
              onPreview={handleOpenDocument}
              rdLoopActive={rdLoopActive}
            />
          )}
          <div className="flex items-center justify-between gap-3 rounded-lg border border-dashed border-border/50 p-3">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <BookOpen className="h-4 w-4" />
              {needsInitialization
                ? t("projectWiki.notInitialized", {
                    defaultValue: "未检测到 wiki/index.md",
                  })
                : "Schema 已初始化；生成完成并验收后才会建立 Wiki 基线"}
            </div>
            {needsInitialization && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => setShowInitConfirm(true)}
              >
                <PlusCircle className="h-3.5 w-3.5 mr-1" />
                {t("projectWiki.actions.init", { defaultValue: "初始化 Wiki" })}
              </Button>
            )}
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
        {viewer}
      </>
    );
  }

  return (
    <>
      <section className="rounded-xl border border-border/60 bg-card/50 p-4 space-y-3">
        <WikiHeader
          data={data}
          onRefresh={refresh}
          loading={loading}
          onLint={() => lint(qualityMode)}
          linting={linting}
          qualityMode={qualityMode}
          onToggleQuality={() => setQualityMode((v) => !v)}
          onAccept={requestAccept}
          accepting={accepting}
          onView={() => handleOpenDocument()}
          onOpenFolder={() => void handleOpenWikiFolder()}
          openingFolder={openingWikiFolder}
        />

        <CandidateBlock
          lifecycle={data.lifecycle}
          candidates={candidates}
          loading={candidatesLoading}
          importingId={importingId}
          onRefresh={refreshCandidates}
          onImport={handleImportCandidate}
          comparison={comparison}
          comparisonLoading={comparisonLoading}
          onCompare={() =>
            compareCandidates(candidates.map((candidate) => candidate.id))
          }
          generating={generatorLoading}
          onGenerate={handleGenerateCandidate}
          aiConfigured={aiConfigured}
          aiConfigLoading={aiConfigLoading}
          onOpenAiProviderSettings={onOpenAiProviderSettings}
          onPreview={handleOpenDocument}
          rdLoopActive={rdLoopActive}
        />

        {data.lifecycle.phase === "pendingAcceptance" && (
          <div className="flex flex-wrap items-center gap-3 rounded-lg border border-amber-500/25 bg-amber-500/[0.045] px-3 py-2.5">
            <div className="min-w-0 flex-1 text-[11px] leading-5 text-muted-foreground">
              <p className="font-medium text-amber-700 dark:text-amber-300">
                下一步：先查看生成内容，再决定是否建立基线
              </p>
              <p>
                正式 Wiki 已进入待验收状态。请逐页核对正文、源码依据和 Lint
                结果；验收后会记录
                {data.sourceBaseline.hasGitCommit
                  ? "当前 Commit"
                  : "当前源码快照"}
                作为同步基线。
              </p>
            </div>
            <Button
              size="sm"
              variant="outline"
              className="h-7 px-2.5 text-[10px]"
              onClick={() => handleOpenDocument()}
            >
              <Eye className="mr-1 h-3 w-3" />
              预览 Wiki
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="h-7 px-2.5 text-[10px]"
              onClick={() => void handleOpenWikiFolder()}
              disabled={openingWikiFolder}
            >
              <FolderOpen className="mr-1 h-3 w-3" />
              {openingWikiFolder ? "打开中…" : "查看 Wiki"}
            </Button>
            <Button
              size="sm"
              className="h-7 px-2.5 text-[10px]"
              onClick={requestAccept}
              disabled={accepting}
            >
              {accepting && <Loader2 className="mr-1 h-3 w-3 animate-spin" />}
              验收并建立基线
            </Button>
          </div>
        )}

        {/* 核心页面覆盖率 */}
        <div className="grid grid-cols-2 gap-2 text-xs">
          <CoverageItem
            label={t("projectWiki.coverage.corePages", {
              defaultValue: "核心页面",
            })}
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
            label={t("projectWiki.coverage.componentPages", {
              defaultValue: "组件",
            })}
            value={`${data.corePageCoverage.componentPages}`}
            ok={data.corePageCoverage.componentPages > 0}
          />
          <CoverageItem
            label={t("projectWiki.coverage.flowPages", {
              defaultValue: "流程",
            })}
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

        {lintResult && <LintResultBlock result={lintResult} />}

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
      {viewer}
      {snapshotDialog}
    </>
  );
}

function CandidateBlock({
  lifecycle,
  candidates,
  loading,
  importingId,
  onRefresh,
  onImport,
  comparison,
  comparisonLoading,
  onCompare,
  generating,
  onGenerate,
  aiConfigured,
  aiConfigLoading,
  onOpenAiProviderSettings,
  onPreview,
  rdLoopActive = false,
}: {
  lifecycle: WikiLifecycle;
  candidates: WikiCandidate[];
  loading: boolean;
  importingId: string | null;
  onRefresh: () => void;
  onImport: (candidateId: string) => Promise<boolean>;
  comparison: WikiComparisonReport | null;
  comparisonLoading: boolean;
  onCompare: () => void;
  generating: boolean;
  onGenerate: () => Promise<boolean>;
  aiConfigured: boolean;
  aiConfigLoading: boolean;
  onOpenAiProviderSettings?: () => void;
  onPreview: (candidateId: string) => void;
  rdLoopActive?: boolean;
}) {
  const { t } = useTranslation();
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [confirmingGenerate, setConfirmingGenerate] = useState(false);
  const waitingForCandidate = [
    "pendingGeneration",
    "generating",
    "changesDetected",
    "pendingSync",
    "syncing",
  ].includes(lifecycle.phase);
  const syncingExistingWiki = [
    "changesDetected",
    "pendingSync",
    "syncing",
  ].includes(lifecycle.phase);

  if (!waitingForCandidate && lifecycle.phase !== "failed") {
    return null;
  }

  const handleImport = async (candidateId: string) => {
    if (await onImport(candidateId)) {
      setConfirmingId(null);
    }
  };

  const handleGenerate = async () => {
    if (await onGenerate()) {
      setConfirmingGenerate(false);
    }
  };

  return (
    <div className="rounded-lg border border-blue-500/20 bg-blue-500/[0.035] p-3 space-y-2">
      <div className="flex items-center gap-2">
        <PackageOpen className="h-3.5 w-3.5 text-blue-500" />
        <span className="text-xs font-medium">项目 Wiki 生成</span>
        <span className="text-[10px] text-muted-foreground">
          隔离快照 · 自动导入 · 人工验收后建基线
        </span>
        {!confirmingGenerate && aiConfigured && (
          <Button
            size="sm"
            variant="ghost"
            className="ml-auto h-6 px-2 text-[10px]"
            onClick={() => setConfirmingGenerate(true)}
            disabled={generating}
          >
            {generating ? (
              <Loader2 className="mr-1 h-3 w-3 animate-spin" />
            ) : (
              <PackageOpen className="mr-1 h-3 w-3" />
            )}
            {lifecycle.phase === "changesDetected" ||
            lifecycle.phase === "pendingSync"
              ? "同步更新 Wiki"
              : candidates.length > 0
                ? "重新生成项目 Wiki"
                : "生成项目 Wiki"}
          </Button>
        )}
        {!confirmingGenerate && !aiConfigured && !aiConfigLoading && (
          <Button
            size="sm"
            variant="outline"
            className="ml-auto h-6 px-2 text-[10px]"
            onClick={onOpenAiProviderSettings}
          >
            配置 AI 提供方
          </Button>
        )}
        {aiConfigLoading && (
          <Loader2 className="ml-auto h-3 w-3 animate-spin text-muted-foreground" />
        )}
        <Button
          size="icon"
          variant="ghost"
          className="h-6 w-6"
          onClick={onRefresh}
          disabled={loading}
          aria-label="刷新生成器候选"
        >
          <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />
        </Button>
      </div>

      {confirmingGenerate && (
        <div className="flex items-center gap-2 rounded-md border border-amber-500/20 bg-amber-500/5 px-3 py-2">
          <p className="min-w-0 flex-1 text-[10px] leading-4 text-muted-foreground">
            {syncingExistingWiki
              ? "将基于当前 Git HEAD 重新生成项目 Wiki，并在导入前备份现有版本。项目源码片段会发送给「设置 → AI 提供方」中的当前 Provider；结果进入“待验收”，不会直接覆盖 Commit 基线。"
              : "将使用「设置 → AI 提供方」中当前 Provider；项目源码片段会发送给该 Provider，可能产生模型费用。结果在隔离的 Git HEAD 快照中生成并自动导入为“待验收”，不会自动建立 Commit 基线。"}
          </p>
          <Button
            size="sm"
            className="h-7 px-2 text-[10px]"
            onClick={() => void handleGenerate()}
            disabled={generating}
          >
            {generating && <Loader2 className="mr-1 h-3 w-3 animate-spin" />}
            {syncingExistingWiki ? "确认同步" : "确认生成"}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 px-2 text-[10px]"
            onClick={() => setConfirmingGenerate(false)}
            disabled={generating}
          >
            取消
          </Button>
        </div>
      )}

      {lifecycle.phase === "failed" && lifecycle.lastError && (
        <div className="rounded-md border border-red-500/20 bg-red-500/5 px-3 py-2 text-[10px] leading-4 text-red-600 dark:text-red-400">
          上次生成失败：{lifecycle.lastError}
        </div>
      )}

      {candidates.length === 0 ? (
        <div className="rounded-md border border-dashed border-blue-500/20 px-3 py-2 text-[11px] leading-5 text-muted-foreground">
          <p className="font-medium text-foreground/80">
            下一步：生成可验收的项目 Wiki
          </p>
          <p>
            OpenSunstar 会读取隔离源码快照、调用已配置的 AI Provider、校验页面
            Schema，并把结果安全导入为待验收版本。
          </p>
          {rdLoopActive && (
            <p className="mt-1 text-muted-foreground/80">
              {t("projectWiki.backfillRdLoopHint", {
                defaultValue:
                  "采用「RD 交付流程」档位时，需求收尾的知识回补产物也会作为候选进入这里，经你验收后并入知识基线。",
              })}
            </p>
          )}
        </div>
      ) : (
        <div className="space-y-1.5">
          {candidates.map((candidate) => {
            const confirming = confirmingId === candidate.id;
            const importing = importingId === candidate.id;
            return (
              <div
                key={candidate.id}
                className="flex items-center gap-3 rounded-md border border-border/50 bg-background/40 px-3 py-2"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 text-xs">
                    <span className="truncate font-medium">
                      {candidate.engine}
                    </span>
                    <span className="truncate font-mono text-[10px] text-muted-foreground">
                      {candidate.id}
                    </span>
                  </div>
                  <div className="mt-0.5 flex items-center gap-3 text-[10px] text-muted-foreground">
                    <span>{candidate.pageCount} 个页面</span>
                    <span className="flex items-center gap-1">
                      <Clock3 className="h-2.5 w-2.5" />
                      {formatTimestamp(candidate.createdAt)}
                    </span>
                  </div>
                </div>
                {confirming ? (
                  <div className="flex items-center gap-1.5">
                    <span className="max-w-52 text-[10px] text-amber-600 dark:text-amber-400">
                      将先备份正式 Wiki，导入后必须验收
                    </span>
                    <Button
                      size="sm"
                      className="h-7 px-2 text-[10px]"
                      onClick={() => void handleImport(candidate.id)}
                      disabled={importing}
                    >
                      {importing && (
                        <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                      )}
                      确认导入
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-7 px-2 text-[10px]"
                      onClick={() => setConfirmingId(null)}
                      disabled={importing}
                    >
                      取消
                    </Button>
                  </div>
                ) : (
                  <div className="flex items-center gap-1.5">
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-7 px-2 text-[10px]"
                      onClick={() => onPreview(candidate.id)}
                      aria-label="预览候选"
                    >
                      <Eye className="mr-1 h-3 w-3" />
                      预览
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 px-2 text-[10px]"
                      onClick={() => setConfirmingId(candidate.id)}
                      disabled={importingId !== null}
                    >
                      <Download className="mr-1 h-3 w-3" />
                      导入候选
                    </Button>
                  </div>
                )}
              </div>
            );
          })}
          <div className="flex items-center justify-between border-t border-border/40 pt-2">
            <div className="text-[10px] leading-4 text-muted-foreground">
              质量对照只在候选的 sourceCommit 与 model 完全一致时成立。
            </div>
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2 text-[10px]"
              onClick={onCompare}
              disabled={candidates.length < 2 || comparisonLoading}
            >
              {comparisonLoading ? (
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              ) : (
                <GitCompare className="mr-1 h-3 w-3" />
              )}
              固定条件质量对照
            </Button>
          </div>
          {comparison && <ComparisonResult report={comparison} />}
        </div>
      )}
    </div>
  );
}

function ComparisonResult({ report }: { report: WikiComparisonReport }) {
  return (
    <div
      className={cn(
        "rounded-md border p-2",
        report.comparable
          ? "border-green-500/20 bg-green-500/[0.035]"
          : "border-amber-500/20 bg-amber-500/[0.035]",
      )}
    >
      <div className="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px]">
        <span
          className={cn(
            "font-medium",
            report.comparable
              ? "text-green-600 dark:text-green-400"
              : "text-amber-600 dark:text-amber-400",
          )}
        >
          {report.comparable
            ? "条件一致，可进行横向对照"
            : "当前不可形成优劣结论"}
        </span>
        {report.baseCommit && (
          <span className="font-mono text-muted-foreground">
            Commit {report.baseCommit.slice(0, 8)}
          </span>
        )}
        {report.model && (
          <span className="text-muted-foreground">模型 {report.model}</span>
        )}
      </div>
      {report.blockers.length > 0 && (
        <ul className="mb-2 space-y-0.5 text-[10px] text-amber-600 dark:text-amber-400">
          {report.blockers.map((blocker) => (
            <li key={blocker}>• {blocker}</li>
          ))}
        </ul>
      )}
      {report.results.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[620px] text-left text-[10px]">
            <thead className="text-muted-foreground">
              <tr className="border-b border-border/40">
                <th className="px-2 py-1 font-medium">引擎</th>
                <th className="px-2 py-1 font-medium">等级</th>
                <th className="px-2 py-1 font-medium">页面</th>
                <th className="px-2 py-1 font-medium">有效引用</th>
                <th className="px-2 py-1 font-medium">无效引用</th>
                <th className="px-2 py-1 font-medium">待查</th>
                <th className="px-2 py-1 font-medium">生成耗时</th>
              </tr>
            </thead>
            <tbody>
              {report.results.map((result) => (
                <tr
                  key={result.candidateId}
                  className="border-b border-border/20 last:border-0"
                >
                  <td className="px-2 py-1.5 font-medium">{result.engine}</td>
                  <td className="px-2 py-1.5 font-mono">
                    {result.qualityLevel}
                  </td>
                  <td className="px-2 py-1.5">{result.pageCount}</td>
                  <td className="px-2 py-1.5 text-green-600 dark:text-green-400">
                    {result.sourceRefCount}
                  </td>
                  <td
                    className={cn(
                      "px-2 py-1.5",
                      result.invalidSourceRefCount > 0 &&
                        "text-amber-600 dark:text-amber-400",
                    )}
                  >
                    {result.invalidSourceRefCount}
                  </td>
                  <td className="px-2 py-1.5">{result.questionCount}</td>
                  <td className="px-2 py-1.5">
                    {result.generationSeconds == null
                      ? "—"
                      : `${result.generationSeconds.toFixed(1)}s`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
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
  onAccept,
  accepting,
  onView,
  onOpenFolder,
  openingFolder = false,
}: {
  data: WikiScanResult;
  onRefresh: () => void;
  loading: boolean;
  onLint?: () => void;
  linting?: boolean;
  qualityMode?: boolean;
  onToggleQuality?: () => void;
  onAccept?: () => void;
  accepting?: boolean;
  onView?: () => void;
  onOpenFolder?: () => void;
  openingFolder?: boolean;
}) {
  const { t } = useTranslation();
  const qualityMeta = QUALITY_META[data.qualityLevel] ?? QUALITY_META["N/A"];
  const lifecycleMeta = LIFECYCLE_META[data.lifecycle.phase];

  return (
    <div className="flex items-center gap-2">
      <BookOpen className="h-4 w-4 shrink-0 text-primary" />
      <h3 className="text-sm font-medium">
        {t("projectWiki.title", { defaultValue: "项目 Wiki / 知识基线" })}
      </h3>
      <span
        className={cn(
          "ml-auto flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium",
          lifecycleMeta.className,
        )}
      >
        {data.lifecycle.phase === "generating" ||
        data.lifecycle.phase === "syncing" ? (
          <Loader2 className="h-3 w-3 animate-spin" />
        ) : (
          <BookOpen className="h-3 w-3" />
        )}
        {lifecycleMeta.label}
      </span>
      <span className={cn("text-[10px] font-medium", qualityMeta.className)}>
        {t(qualityMeta.label)}
      </span>
      {onView && (
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-2 text-[10px]"
          onClick={onView}
        >
          <Eye className="mr-1 h-3 w-3" />
          预览 Wiki
        </Button>
      )}
      {onOpenFolder && (
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-2 text-[10px]"
          onClick={onOpenFolder}
          disabled={openingFolder}
        >
          <FolderOpen className="mr-1 h-3 w-3" />
          {openingFolder ? "打开中…" : "查看 Wiki"}
        </Button>
      )}
      {onLint && onToggleQuality && (
        <Button
          size="icon"
          variant="ghost"
          className={cn("h-6 w-6", qualityMode && "text-primary")}
          onClick={onToggleQuality}
          aria-label={t("projectWiki.actions.toggleQuality", {
            defaultValue: "切换 quality 模式",
          })}
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
          aria-label={t("projectWiki.actions.lint", {
            defaultValue: "运行 Lint",
          })}
        >
          {linting ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : (
            <Bug className="h-3 w-3" />
          )}
        </Button>
      )}
      {onAccept && data.lifecycle.phase === "pendingAcceptance" && (
        <Button
          size="sm"
          variant="outline"
          className="h-6 px-2 text-[10px]"
          onClick={onAccept}
          disabled={accepting}
        >
          {accepting && <Loader2 className="mr-1 h-3 w-3 animate-spin" />}
          验收并建立基线
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
            defaultValue:
              "{{files}} 文件 / {{errors}} 错误 / {{warnings}} 警告 / 等级 {{level}}",
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
            {" ("}
            {result.errors.length}
            {")"}
          </p>
          <ul className="space-y-0.5 text-[11px]">
            {result.errors.slice(0, 10).map((e, i) => (
              <li key={i} className="flex gap-1 text-muted-foreground">
                <span className="font-mono text-red-500 shrink-0">
                  {e.ruleId}
                </span>
                <span className="truncate">
                  {e.file}
                  {e.line ? `:${e.line}` : ""}
                </span>
                <span className="truncate">{e.message}</span>
              </li>
            ))}
            {result.errors.length > 10 && (
              <li className="text-muted-foreground/60">
                …
                {t("projectWiki.lint.more", {
                  defaultValue: "还有 {{count}} 条",
                  count: result.errors.length - 10,
                })}
              </li>
            )}
          </ul>
        </div>
      )}

      {/* Warnings 列表 */}
      {result.warnings.length > 0 && (
        <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-2">
          <p className="mb-1 text-[11px] font-medium text-amber-600 dark:text-amber-400">
            {t("projectWiki.lint.warnings", {
              defaultValue: "{{count}} 个警告",
              count: result.warnings.length,
            })}
          </p>
          <ul className="space-y-0.5 text-[11px]">
            {result.warnings.slice(0, 8).map((w, i) => (
              <li key={i} className="flex gap-1 text-muted-foreground">
                <span className="font-mono text-amber-500 shrink-0">
                  {w.ruleId}
                </span>
                <span className="truncate">
                  {w.file}
                  {w.line ? `:${w.line}` : ""}
                </span>
                <span className="truncate">{w.message}</span>
              </li>
            ))}
            {result.warnings.length > 8 && (
              <li className="text-muted-foreground/60">
                …
                {t("projectWiki.lint.more", {
                  defaultValue: "还有 {{count}} 条",
                  count: result.warnings.length - 8,
                })}
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
              {" ("}
              {data.effectiveSourcePages}/{data.threshold}
              {")"}
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
              {t("projectWiki.changed.changedFiles", { defaultValue: "变更" })}:{" "}
              {data.changedFiles.length}
            </span>
            <span>
              {t("projectWiki.changed.affectedPages", {
                defaultValue: "影响页面",
              })}
              : {data.affectedPages.length}
            </span>
            <span>
              {t("projectWiki.changed.unmapped", { defaultValue: "未映射" })}:{" "}
              {data.unmappedChangedFiles.length}
            </span>
          </div>
          {data.affectedPages.length > 0 && (
            <ul className="space-y-0.5">
              {data.affectedPages.slice(0, 5).map((p, i) => (
                <li
                  key={i}
                  className="flex items-center gap-1 text-muted-foreground"
                >
                  <span className="text-green-500 shrink-0">→</span>
                  <span className="truncate">{p}</span>
                </li>
              ))}
            </ul>
          )}
          {data.unmappedChangedFiles.length > 0 && (
            <ul className="space-y-0.5">
              {data.unmappedChangedFiles.slice(0, 3).map((f, i) => (
                <li
                  key={i}
                  className="flex items-center gap-1 text-muted-foreground/70"
                >
                  <FileQuestion className="h-2.5 w-2.5 shrink-0 text-amber-500" />
                  <span className="truncate">{f}</span>
                </li>
              ))}
              {data.unmappedChangedFiles.length > 3 && (
                <li className="text-muted-foreground/50">
                  …
                  {t("projectWiki.lint.more", {
                    defaultValue: "还有 {{count}} 条",
                    count: data.unmappedChangedFiles.length - 3,
                  })}
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

function SourceSnapshotBaselineDialog({
  open,
  fileCount,
  accepting,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  fileCount: number | null;
  accepting: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => Promise<void>;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>记录源码快照基线</DialogTitle>
          <DialogDescription className="leading-5">
            当前项目没有可用的 Git
            Commit。将以当前源码文件清单和内容摘要建立快照基线，后续初始化 Git
            并产生 Commit 后，下一次验收会自动升级为 Commit 基线。
          </DialogDescription>
        </DialogHeader>
        <div className="rounded-lg border border-blue-500/20 bg-blue-500/[0.045] p-3 text-xs leading-5 text-muted-foreground">
          <p className="font-medium text-foreground">本次会记录</p>
          <ul className="mt-1 list-disc space-y-1 pl-4">
            <li>源码文件路径、文件数量与内容 SHA-256 摘要</li>
            <li>当前已验收 Wiki 的内容摘要</li>
            <li>后续源码变更检测所需的对比状态</li>
          </ul>
          {fileCount !== null && (
            <p className="mt-2">上次快照覆盖 {fileCount} 个文件。</p>
          )}
        </div>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={accepting}
          >
            取消
          </Button>
          <Button
            type="button"
            onClick={() => void onConfirm()}
            disabled={accepting}
          >
            {accepting && <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />}
            记录快照并验收
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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
        <Button
          size="sm"
          variant="ghost"
          className="mt-2 h-6"
          onClick={onCancel}
        >
          {t("common.close", { defaultValue: "关闭" })}
        </Button>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border/60 bg-muted/20 p-3 space-y-2">
      <p className="text-xs font-medium">
        {t("projectWiki.init.previewTitle", {
          defaultValue: "Wiki 初始化预检",
        })}
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
        <Button
          size="sm"
          variant="ghost"
          onClick={onCancel}
          disabled={installing}
        >
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
