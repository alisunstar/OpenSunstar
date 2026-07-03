import { useState, useMemo, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";
import {
  Search,
  LayoutGrid,
  FolderArchive,
  Plus,
  RefreshCw,
  CopyCheck,
  AlertTriangle,
  X,
  BarChart3,
} from "lucide-react";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { StageSection } from "./StageSection";
import { SummaryCard } from "./SummaryCard";
import { ProjectDetailSheet } from "./ProjectDetailSheet";
import type { StageKey } from "@/hooks/useProjectStages";
import { useProjectStages } from "@/hooks/useProjectStages";
import { useProjectProgress } from "@/hooks/useProjectProgress";
import { useProjectMetricsScan } from "@/hooks/kanban/useProjectMetricsScan";
import { useKanbanFilters } from "@/hooks/kanban/useKanbanFilters";
import { usePortfolioDerivedMetrics } from "@/hooks/kanban/usePortfolioDerivedMetrics";
import { useDuplicateProjectScan } from "@/hooks/kanban/useDuplicateProjectScan";
import { usePortfolioAIAnalysis } from "@/hooks/kanban/usePortfolioAIAnalysis";
import { useAgentReadinessBatch } from "@/hooks/kanban/useAgentReadinessBatch";
import { useKanbanRemoveProject } from "@/hooks/kanban/useKanbanRemoveProject";
import { revealPathInFolder } from "@/lib/reveal";
import type { Project } from "@/types/project";
import type { PageView } from "@/App";
import { AINLQueryBar } from "./AINLQueryBar";
import { AICostStrip } from "./AICostStrip";
import { AICostPanel } from "./AICostPanel";
import { AIPortfolioMatrix } from "./AIPortfolioMatrix";
import { AIWeeklyReport } from "./AIWeeklyReport";
import { useAIConfig } from "@/hooks/useAIConfig";
import { AICostProvider } from "@/contexts/AICostContext";
import { PortfolioDriftSummary } from "./PortfolioDriftSummary";
import { GovernanceDashboard } from "./GovernanceDashboard";
import { TodayWorkspace } from "./TodayWorkspace";
import { ProjectAssetsMatrix } from "./ProjectAssetsMatrix";
import { WorkspaceTabBar } from "./WorkspaceTabBar";
import { usePortfolioAssetSummary } from "@/hooks/kanban/usePortfolioAssetSummary";
import { repairProjectDrift } from "@/api/aiInsight";
import { showRepairProjectFeedback } from "@/lib/repairFeedback";
import type { WorkspaceTab } from "@/types/workspace";
import type { ProjectDetailIntent } from "@/types/projectDetail";
import type { AppId } from "@/lib/api";
import {
  PORTFOLIO_OVERVIEW_WINDOW_OPTIONS,
  type PortfolioOverviewWindowDays,
} from "@/lib/portfolioMetrics";
import { cn } from "@/lib/utils";

interface KanbanPageProps {
  projects: Project[];
  selectedProjectId?: string;
  projectDetailIntent?: ProjectDetailIntent | null;
  workspaceTab?: WorkspaceTab;
  onWorkspaceTabChange?: (tab: WorkspaceTab) => void;
  onProjectClick: (project: Project) => void;
  onProjectRemove: (projectId: string) => void;
  onAddProject: () => void;
  onClearSelection?: () => void;
  onOpenSettings?: () => void;
  onNavigate?: (view: PageView) => void;
  onPortfolioDataChanged?: () => void;
  targetApp?: AppId;
}

// ── 主组件 ─────────────────────────────────────

export function KanbanPage({
  projects,
  selectedProjectId,
  projectDetailIntent,
  workspaceTab = "dashboard",
  onWorkspaceTabChange,
  onProjectClick,
  onProjectRemove,
  onAddProject,
  onClearSelection,
  onOpenSettings,
  onNavigate,
  onPortfolioDataChanged,
  targetApp = "claude",
}: KanbanPageProps) {
  const { t } = useTranslation();
  const { stages, getStage, setStage } = useProjectStages();
  const { progress: progressMap, getProgress, setProjectProgress } =
    useProjectProgress();
  const [internalDetailId, setInternalDetailId] = useState<string | null>(null);
  const [detailInitialTab, setDetailInitialTab] = useState<
    "overview" | "aiAssets"
  >("overview");
  const [portfolioRefreshToken, setPortfolioRefreshToken] = useState(0);
  const [repairingProjectId, setRepairingProjectId] = useState<string | null>(null);
  const [roiPanelOpen, setRoiPanelOpen] = useState(false);
  const [overviewWindowDays, setOverviewWindowDays] =
    useState<PortfolioOverviewWindowDays>(7);

  const { aiConfigured, refreshConfig, getConfig } = useAIConfig();

  const {
    codeLinesMap,
    versionMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    contributorsMap,
    weeklyCommitsMap,
    scanning,
    scanProgress,
    scanEpoch,
    refreshScan,
  } = useProjectMetricsScan(projects);

  const { searchQuery, setSearchQuery, grouped, empty, noResults } =
    useKanbanFilters(projects, getStage);

  const { dupGroups, dupScanning, scanDuplicates, removeFromDupGroups } =
    useDuplicateProjectScan(projects);

  const {
    aiSummaryMap,
    aiHealthMap,
    aiLoadingMap,
    aiTrendInsightMap,
  } = usePortfolioAIAnalysis({
    projects,
    scanning,
    aiConfigured,
    getConfig,
    getStage,
    progressMap,
    codeLinesMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    contributorsMap,
    versionMap,
    weeklyCommitsMap,
    scanEpoch,
  });

  const { agentReadinessMap, loading: readinessLoading } = useAgentReadinessBatch({
    projects,
    scanning,
    scanEpoch,
    portfolioRefreshToken,
    getConfig,
    targetApp,
  });

  const {
    assetMap,
    loading: assetSummaryLoading,
    lastUpdatedAt: assetLastUpdatedAt,
  } = usePortfolioAssetSummary(projects, portfolioRefreshToken);

  const [readinessLastUpdatedAt, setReadinessLastUpdatedAt] = useState<
    number | null
  >(null);

  useEffect(() => {
    if (!readinessLoading && projects.length > 0) {
      setReadinessLastUpdatedAt(Date.now());
    }
  }, [readinessLoading, portfolioRefreshToken, agentReadinessMap, projects.length]);

  const portfolioLastUpdatedAt = useMemo(() => {
    const times = [assetLastUpdatedAt, readinessLastUpdatedAt].filter(
      (t): t is number => t != null,
    );
    return times.length > 0 ? Math.max(...times) : null;
  }, [assetLastUpdatedAt, readinessLastUpdatedAt]);

  const portfolioDataRefreshing =
    assetSummaryLoading || readinessLoading || scanning;

  const bumpPortfolioRefresh = useCallback(() => {
    setPortfolioRefreshToken((token) => token + 1);
    onPortfolioDataChanged?.();
  }, [onPortfolioDataChanged]);

  const handleRepairProjectDrift = useCallback(
    async (project: Project) => {
      setRepairingProjectId(project.id);
      try {
        const result = await repairProjectDrift(project.path, targetApp);
        const ok = showRepairProjectFeedback(result, t);
        if (ok || result) {
          bumpPortfolioRefresh();
        }
      } finally {
        setRepairingProjectId(null);
      }
    },
    [bumpPortfolioRefresh, targetApp, t],
  );

  useEffect(() => {
    if (!projectDetailIntent) return;
    setInternalDetailId(projectDetailIntent.projectId);
    setDetailInitialTab(projectDetailIntent.tab);
  }, [projectDetailIntent]);

  const {
    projectContextsMap,
    portfolioPoints,
    totalCodeLines,
    totalCommitsInWindow,
    averageActivityLabel,
    averageActivityColor,
    formatCompactNumber,
  } = usePortfolioDerivedMetrics({
    projects,
    codeLinesMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    weeklyCommitsMap,
    contributorsMap,
    versionMap,
    progressMap,
    aiHealthMap,
    aiConfigured,
    scanning,
    overviewWindowDays,
    getStage,
    t,
  });

  const {
    removeConfirm,
    handleRemove,
    confirmRemoveProject,
    cancelRemove,
  } = useKanbanRemoveProject(projects, onProjectRemove, t, removeFromDupGroups);

  const activeDetailId = selectedProjectId ?? internalDetailId;
  const detailProject = useMemo(
    () => projects.find((p) => p.id === activeDetailId) ?? null,
    [projects, activeDetailId],
  );

  const openDetail = (
    project: Project,
    options?: { assetsTab?: boolean },
  ) => {
    onProjectClick(project);
    setInternalDetailId(project.id);
    setDetailInitialTab(options?.assetsTab ? "aiAssets" : "overview");
  };

  const closeDetail = () => {
    setInternalDetailId(null);
    onClearSelection?.();
  };

  const handleOpenFolder = async (path: string) => {
    await revealPathInFolder(path, { alertOnError: true });
  };

  const handleRefresh = () => {
    refreshScan();
    refreshConfig();
    bumpPortfolioRefresh();
  };

  const handlePortfolioConfigChanged = () => {
    bumpPortfolioRefresh();
  };

  const totalCount = projects.length;

  return (
    <AICostProvider>
    <motion.div
      className="flex-1 overflow-y-auto"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
    >
      <ConfirmDialog
        isOpen={removeConfirm !== null}
        title={t("kanban.confirmRemoveTitle", {
          defaultValue: "从项目AI看板中移除？",
        })}
        message={t("kanban.confirmRemove", {
          name: removeConfirm?.name ?? "",
          defaultValue:
            "将移除「{{name}}」；不会删除磁盘上的仓库文件，阶段与进度等本地看板数据也会清除。",
        })}
        confirmText={t("kanban.confirmRemoveBtn", { defaultValue: "移除" })}
        onConfirm={confirmRemoveProject}
        onCancel={cancelRemove}
      />
      {/* 页面头部 — sticky，滚动时保持 AI 操作按钮可见 */}
      <div className="sticky top-0 z-20 shrink-0 border-b border-border/30 bg-background/95 backdrop-blur-sm px-6 pt-6 pb-4">
        <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
              <LayoutGrid className="w-5 h-5 text-primary shrink-0" />
              {t("workspace.title", { defaultValue: "工作区" })}
            </h2>
            <p className="text-sm text-muted-foreground mt-1">
              {scanning
                ? t("kanban.scanning", {
                    done: scanProgress.done,
                    total: scanProgress.total,
                    defaultValue: `正在扫描 ${scanProgress.done}/${scanProgress.total} 个项目…`,
                  })
                : t("workspace.subtitle", {
                    defaultValue:
                      "每日开工先看进度与 AI 资产配置，再进入项目看板迭代",
                  })}
            </p>
            {!empty && onWorkspaceTabChange && (
              <div className="mt-3">
                <WorkspaceTabBar
                  activeTab={workspaceTab}
                  onChange={onWorkspaceTabChange}
                />
              </div>
            )}
          </div>

          <div className="flex flex-wrap items-center justify-end gap-2 shrink-0">
            {!empty && aiConfigured && (
              <>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setRoiPanelOpen(true)}
                >
                  <BarChart3 className="w-4 h-4 mr-1" />
                  AI 投入报告
                </Button>
                <AIWeeklyReport
                  projectContexts={Array.from(projectContextsMap.values())}
                  aiConfigured={aiConfigured}
                />
              </>
            )}

            {!empty && projects.length > 1 && (
              <Popover>
                <PopoverTrigger asChild>
                  <Button variant="ghost" size="sm" className="relative">
                    {dupScanning ? (
                      <RefreshCw className="w-4 h-4 mr-1 animate-spin" />
                    ) : (
                      <CopyCheck className="w-4 h-4 mr-1" />
                    )}
                    {t("health.scan", { defaultValue: "重复检测" })}
                    {dupGroups !== null && dupGroups.length > 0 && (
                      <span className="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-amber-500 text-[10px] font-bold text-white px-1">
                        {dupGroups.reduce((s, g) => s + g.projects.length, 0)}
                      </span>
                    )}
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-80 p-0" align="end" sideOffset={8}>
                  <div className="p-4 space-y-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <CopyCheck className="w-4 h-4 text-amber-500" />
                        <h4 className="text-sm font-semibold text-foreground">
                          {t("board.duplicateCleanup", {
                            defaultValue: "重复资产检测",
                          })}
                        </h4>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-7 text-xs"
                        disabled={dupScanning}
                        onClick={scanDuplicates}
                      >
                        {dupScanning ? (
                          <RefreshCw className="w-3 h-3 mr-1 animate-spin" />
                        ) : (
                          <CopyCheck className="w-3 h-3 mr-1" />
                        )}
                        {t("health.scan", { defaultValue: "检测重复" })}
                      </Button>
                    </div>
                    <p className="text-[11px] text-muted-foreground/70">
                      {t("health.subtitle", {
                        defaultValue:
                          "检测同名项目与重复路径，识别可能的冗余添加",
                      })}
                    </p>

                    {dupGroups !== null && (
                      <>
                        {dupGroups.length === 0 ? (
                          <div className="flex items-center gap-2 text-xs text-emerald-600 dark:text-emerald-400 py-2 px-3 rounded-lg bg-emerald-500/5">
                            <CopyCheck className="w-3.5 h-3.5" />
                            {t("health.noDuplicates", {
                              defaultValue: "未发现重复项目",
                            })}
                          </div>
                        ) : (
                          <div className="space-y-2 max-h-[300px] overflow-y-auto">
                            {dupGroups.map((group, gi) => (
                              <div
                                key={gi}
                                className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3"
                              >
                                <div className="flex items-center gap-2 mb-2">
                                  <AlertTriangle className="w-3.5 h-3.5 text-amber-500" />
                                  <span className="text-xs font-medium text-amber-600 dark:text-amber-400">
                                    {group.reason}
                                  </span>
                                </div>
                                <div className="space-y-1">
                                  {group.projects.map((p) => (
                                    <div
                                      key={p.id}
                                      className="flex items-center justify-between text-xs py-1 px-2 rounded bg-muted/30"
                                    >
                                      <span className="truncate flex-1">
                                        <span className="font-medium">
                                          {p.name}
                                        </span>
                                        <span className="text-muted-foreground ml-2 font-mono text-[10px]">
                                          {p.path}
                                        </span>
                                      </span>
                                      <Button
                                        variant="ghost"
                                        size="icon"
                                        className="h-5 w-5 text-muted-foreground hover:text-destructive"
                                        onClick={() => handleRemove(p.id)}
                                        title={t("kanban.remove", {
                                          defaultValue: "移除项目",
                                        })}
                                      >
                                        <X className="w-3 h-3" />
                                      </Button>
                                    </div>
                                  ))}
                                </div>
                              </div>
                            ))}
                          </div>
                        )}
                      </>
                    )}
                  </div>
                </PopoverContent>
              </Popover>
            )}

            {!empty && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleRefresh}
                disabled={scanning}
              >
                <RefreshCw
                  className={`w-4 h-4 mr-1 ${scanning ? "animate-spin" : ""}`}
                />
                {t("kanban.refresh", { defaultValue: "刷新指标" })}
              </Button>
            )}
            <Button variant="outline" size="sm" onClick={onAddProject}>
              <Plus className="w-4 h-4 mr-1" />
              {t("kanban.addProject", { defaultValue: "添加项目" })}
            </Button>
          </div>
        </div>
      </div>

      {!empty && workspaceTab === "board" && (
        <div className="px-6 pb-4">
          <div className="relative max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground/50 pointer-events-none" />
            <Input
              className="pl-9"
              placeholder={t("kanban.searchPlaceholder", {
                defaultValue: "搜索项目...",
              })}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </div>
      )}

      {!empty && workspaceTab === "dashboard" && (
        <div className="px-6 pb-6 space-y-4">
          <GovernanceDashboard
            projects={projects}
            agentReadinessMap={agentReadinessMap}
            targetApp={targetApp}
            loading={readinessLoading}
          />
          <PortfolioDriftSummary
            projects={projects}
            agentReadinessMap={agentReadinessMap}
            targetApp={targetApp}
            lastUpdatedAt={
              readinessLastUpdatedAt != null
                ? Math.floor(readinessLastUpdatedAt / 1000)
                : null
            }
            onOpenProject={(p) => openDetail(p)}
            onRepairProject={(p) => void handleRepairProjectDrift(p)}
            repairingProjectId={repairingProjectId}
          />
          <TodayWorkspace
            projects={projects}
            getStage={getStage}
            progressMap={progressMap}
            agentReadinessMap={agentReadinessMap}
            assetMap={assetMap}
            commits7dMap={commits7dMap}
            overviewWindowDays={overviewWindowDays}
            lastUpdatedAt={portfolioLastUpdatedAt}
            isRefreshing={portfolioDataRefreshing}
            onOpenProject={openDetail}
          />

          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-foreground">
              {t("board.summary.title", { defaultValue: "项目总览" })}
            </h3>
            <div
              className="flex items-center gap-1 rounded-lg border border-border/50 bg-muted/20 p-0.5"
              role="group"
              aria-label={t("board.summary.windowLabel", {
                defaultValue: "Git 活跃统计周期",
              })}
            >
              {PORTFOLIO_OVERVIEW_WINDOW_OPTIONS.map((days) => (
                <button
                  key={days}
                  type="button"
                  className={cn(
                    "rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors",
                    overviewWindowDays === days
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                  onClick={() => setOverviewWindowDays(days)}
                  aria-pressed={overviewWindowDays === days}
                >
                  {days} {t("board.summary.daysUnit", { defaultValue: "天" })}
                </button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <SummaryCard
              label={t("board.summary.totalProjects", {
                defaultValue: "总项目数",
              })}
              value={String(totalCount)}
            />
            <SummaryCard
              label={t("board.summary.totalCodeLines", {
                defaultValue: "总代码行数",
              })}
              value={
                totalCodeLines > 0 ? formatCompactNumber(totalCodeLines) : "—"
              }
              unit={
                totalCodeLines > 0
                  ? t("board.summary.linesUnit", { defaultValue: "行" })
                  : undefined
              }
            />
            <SummaryCard
              label={t("board.summary.avgActivity", {
                days: overviewWindowDays,
                defaultValue: `平均活跃度（${overviewWindowDays} 天）`,
              })}
              value={averageActivityLabel}
              color={averageActivityColor}
              sub={
                totalCommitsInWindow > 0
                  ? t("kanban.commitsWindowHint", {
                      count: totalCommitsInWindow,
                      days: overviewWindowDays,
                      defaultValue: `近 ${overviewWindowDays} 天共 {{count}} 次提交`,
                    })
                  : t("kanban.noRecentActivityWindow", {
                      days: overviewWindowDays,
                      defaultValue: `近 ${overviewWindowDays} 天无提交`,
                    })
              }
            />
            <SummaryCard
              label={t("board.summary.commitsInWindow", {
                days: overviewWindowDays,
                defaultValue: `近 ${overviewWindowDays} 天提交`,
              })}
              value={
                totalCommitsInWindow > 0 ? String(totalCommitsInWindow) : "—"
              }
              unit={
                totalCommitsInWindow > 0
                  ? t("board.summary.updatesUnit", { defaultValue: "次" })
                  : undefined
              }
            />
          </div>

          <div className="rounded-xl border border-border/60 bg-card/30 p-4">
            <div className="flex items-center gap-6">
              <span className="text-xs font-medium text-muted-foreground">
                {t("kanban.stageDistribution", { defaultValue: "阶段分布" })}
              </span>
              {(
                [
                  {
                    key: "mvp" as StageKey,
                    label: "MVP 阶段（未上线）",
                    color: "bg-purple-500",
                    count: grouped.mvp.length,
                  },
                  {
                    key: "rapid" as StageKey,
                    label: "快速迭代阶段（已上线）",
                    color: "bg-emerald-500",
                    count: grouped.rapid.length,
                  },
                  {
                    key: "stable" as StageKey,
                    label: "慢迭代阶段（稳定维护）",
                    color: "bg-blue-500",
                    count: grouped.stable.length,
                  },
                ] as const
              ).map((item) => (
                <div key={item.key} className="flex items-center gap-2">
                  <span className={`w-2.5 h-2.5 rounded-full ${item.color}`} />
                  <span className="text-xs text-foreground/80">
                    {item.label}
                  </span>
                  <span className="text-xs font-semibold text-foreground tabular-nums">
                    {item.count}
                    <span className="text-muted-foreground font-normal ml-0.5">
                      (
                      {totalCount > 0
                        ? Math.round((item.count / totalCount) * 100)
                        : 0}
                      %)
                    </span>
                  </span>
                </div>
              ))}
            </div>
          </div>

          {aiConfigured && portfolioPoints.length > 0 && (
            <AIPortfolioMatrix points={portfolioPoints} />
          )}
        </div>
      )}

      {!empty && (workspaceTab === "dashboard" || workspaceTab === "board") && (
        <div className="px-6 pb-2 space-y-2">
          <AICostStrip
            aiConfigured={aiConfigured}
            projectCount={totalCount}
            onOpenRoiPanel={() => setRoiPanelOpen(true)}
            onOpenSettings={onOpenSettings}
          />
          <AINLQueryBar
            projectContexts={Array.from(projectContextsMap.values())}
            aiConfigured={aiConfigured}
            projectCount={totalCount}
            onOpenSettings={onOpenSettings}
          />
        </div>
      )}

      {!empty && workspaceTab === "assetsMatrix" && (
        <div className="px-6 pb-8">
          <ProjectAssetsMatrix
            projects={projects}
            getStage={getStage}
            progressMap={progressMap}
            agentReadinessMap={agentReadinessMap}
            assetMap={assetMap}
            loading={assetSummaryLoading}
            onOpenProject={openDetail}
          />
        </div>
      )}

      <div className="px-6 pb-8 space-y-8">
        {empty ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <FolderArchive className="w-16 h-16 text-muted-foreground/30 mb-4" />
            <h3 className="text-base font-semibold text-foreground">
              {t("kanban.empty.title", { defaultValue: "暂无项目" })}
            </h3>
            <p className="text-sm text-muted-foreground mt-1.5 max-w-sm">
              {t("kanban.empty.description", {
                defaultValue: "点击下方按钮或在侧边栏添加你的第一个项目",
              })}
            </p>
            <Button onClick={onAddProject} className="mt-4" size="sm">
              <Plus className="w-4 h-4 mr-1" />
              {t("kanban.addProject", { defaultValue: "添加项目" })}
            </Button>
          </div>
        ) : noResults ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <Search className="w-12 h-12 text-muted-foreground/30 mb-3" />
            <p className="text-sm text-muted-foreground">
              {t("kanban.noResults", {
                defaultValue: `没有找到匹配「${searchQuery}」的项目`,
              })}
            </p>
          </div>
        ) : workspaceTab === "board" ? (
          <>
            <StageSection
              stage="mvp"
              projects={grouped.mvp}
              stages={stages}
              progressMap={progressMap}
              aiSummaryMap={aiSummaryMap}
              aiLoadingMap={aiLoadingMap}
              aiHealthMap={aiHealthMap}
              agentReadinessMap={agentReadinessMap}
              onProjectClick={openDetail}
              onProjectRemove={handleRemove}
              onStageChange={(projectId, stage) => setStage(projectId, stage)}
              onOpenFolder={handleOpenFolder}
            />
            <StageSection
              stage="rapid"
              projects={grouped.rapid}
              stages={stages}
              progressMap={progressMap}
              aiSummaryMap={aiSummaryMap}
              aiLoadingMap={aiLoadingMap}
              aiHealthMap={aiHealthMap}
              agentReadinessMap={agentReadinessMap}
              onProjectClick={openDetail}
              onProjectRemove={handleRemove}
              onStageChange={(projectId, stage) => setStage(projectId, stage)}
              onOpenFolder={handleOpenFolder}
            />
            <StageSection
              stage="stable"
              projects={grouped.stable}
              stages={stages}
              progressMap={progressMap}
              aiSummaryMap={aiSummaryMap}
              aiLoadingMap={aiLoadingMap}
              aiHealthMap={aiHealthMap}
              agentReadinessMap={agentReadinessMap}
              onProjectClick={openDetail}
              onProjectRemove={handleRemove}
              onStageChange={(projectId, stage) => setStage(projectId, stage)}
              onOpenFolder={handleOpenFolder}
            />
          </>
        ) : null}
      </div>

      <AnimatePresence>
        {detailProject && (
          <ProjectDetailSheet
            project={detailProject}
            stage={getStage(detailProject.id)}
            progress={getProgress(detailProject.id)}
            codeLines={codeLinesMap.get(detailProject.id)}
            version={versionMap.get(detailProject.id)}
            gitInfo={gitInfoMap.get(detailProject.id)}
            activity={commits7dMap.get(detailProject.id)}
            activity30d={commits30dMap.get(detailProject.id)}
            contributors={contributorsMap.get(detailProject.id)}
            weeklyCommits={weeklyCommitsMap.get(detailProject.id)}
            projectContext={projectContextsMap.get(detailProject.id) ?? null}
            aiTrendInsight={aiTrendInsightMap.get(detailProject.id) ?? null}
            aiConfigured={aiConfigured}
            onStageChange={(s) => setStage(detailProject.id, s)}
            onProgressChange={(p) => setProjectProgress(detailProject.id, p)}
            onClose={closeDetail}
            onNavigate={onNavigate}
            initialTab={detailInitialTab}
            onPortfolioConfigChanged={handlePortfolioConfigChanged}
            targetApp={targetApp}
          />
        )}
      </AnimatePresence>
      <AICostPanel open={roiPanelOpen} onOpenChange={setRoiPanelOpen} />
    </motion.div>
    </AICostProvider>
  );
}
