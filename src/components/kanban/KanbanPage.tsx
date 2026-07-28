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
import { useProjectViews } from "@/hooks/kanban/useProjectViews";
import { pickProjectViews } from "@/lib/kanban/projectView";
import { revealPathInFolder } from "@/lib/reveal";
import type { Project } from "@/types/project";
import { AINLQueryBar } from "./AINLQueryBar";
import { AICostStrip } from "./AICostStrip";
import { AICostPanel } from "./AICostPanel";
import { PortfolioMatrix } from "./PortfolioMatrix";
import { AIWeeklyReport } from "./AIWeeklyReport";
import { useAIConfig } from "@/hooks/useAIConfig";
import { AICostProvider } from "@/contexts/AICostContext";
import { PortfolioDriftSummary } from "./PortfolioDriftSummary";
import { GovernanceDashboard } from "./GovernanceDashboard";
import { TodayWorkspace } from "./TodayWorkspace";
import { PortfolioHealthSummary } from "./PortfolioHealthSummary";
import { DashboardOnboarding } from "./DashboardOnboarding";
import { ProjectAssetsMatrix } from "./ProjectAssetsMatrix";
import {
  WORKSPACE_TABPANEL_ID,
  WorkspaceTabBar,
  workspaceTabId,
} from "./WorkspaceTabBar";
import { usePortfolioAssetSummary } from "@/hooks/kanban/usePortfolioAssetSummary";
import { PortfolioDataNotice } from "./PortfolioDataNotice";
import { repairProjectDrift, previewRepairProjectDrift } from "@/api/aiInsight";
import type { RepairPreviewResult } from "@/api/aiInsight";
import { showRepairProjectFeedback } from "@/lib/repairFeedback";
import { RepairPreviewDialog } from "./RepairPreviewDialog";
import type { WorkspaceTab } from "@/types/workspace";
import type { ProjectDetailIntent } from "@/types/projectDetail";
import type { AppId } from "@/lib/api";
// 只剩下类型：窗口切换器（选项常量 + `cn` 拼样式）已经搬进 TodayWorkspace，
// 和它切的那排指标卡待在同一个文件里。这里保留的 state 仍是唯一事实来源，
// 通过 `onOverviewWindowDaysChange` 回写。
import { type PortfolioOverviewWindowDays } from "@/lib/portfolioMetrics";

interface KanbanPageProps {
  projects: Project[];
  selectedProjectId?: string;
  projectDetailIntent?: ProjectDetailIntent | null;
  workspaceTab?: WorkspaceTab;
  onWorkspaceTabChange?: (tab: WorkspaceTab) => void;
  onProjectClick: (project: Project) => void;
  /**
   * 「去配置这个项目的 AI 资产」。
   *
   * 以前它是 `onProjectClick(project, { assetsTab: true })` 的一个选项 ——
   * 同一个动词（打开项目）后面挂一个 boolean 决定去哪儿。现在这块配置是独立
   * 页面，不再是抽屉的第二个 Tab，两件事就该有两个名字：一个开抽屉看概览，
   * 一个跳去配置页。
   *
   * **必填**：它一度是可选的，于是 `AppPageRouter` 忘了往下传时编译器一声不吭，
   * 三个 Tab 上的「配置资产」「去配置」「查看项目资产」全体静默失效 —— 点了
   * 没反应，没有报错，没有类型错误。这一页在真实应用里只有一个挂载点，可选
   * 换不来任何灵活性，只换来了这个。测试要传就传 `vi.fn()`。
   */
  onOpenProjectAiConfig: (projectId: string) => void;
  onProjectRemove: (projectId: string) => void;
  onAddProject: () => void;
  onClearSelection?: () => void;
  onOpenSettings?: () => void;
  onPortfolioDataChanged?: () => void;
  targetApp?: AppId;
  onProjectsReload?: () => void | Promise<void>;
}

// ── 主组件 ─────────────────────────────────────

export function KanbanPage({
  projects,
  selectedProjectId,
  projectDetailIntent,
  workspaceTab = "dashboard",
  onWorkspaceTabChange,
  onProjectClick,
  onOpenProjectAiConfig,
  onProjectRemove,
  onAddProject,
  onClearSelection,
  onPortfolioDataChanged,
  targetApp = "claude",
  onProjectsReload,
}: KanbanPageProps) {
  const { t } = useTranslation();
  const reloadProjects = onProjectsReload ?? (() => undefined);
  const { getStage, setStage } = useProjectStages(projects, reloadProjects);
  const { progress: progressMap, setProjectProgress } = useProjectProgress(
    projects,
    reloadProjects,
  );
  const [internalDetailId, setInternalDetailId] = useState<string | null>(null);
  const [portfolioRefreshToken, setPortfolioRefreshToken] = useState(0);
  const [repairingProjectId, setRepairingProjectId] = useState<string | null>(
    null,
  );
  const [repairPreviewOpen, setRepairPreviewOpen] = useState(false);
  const [repairPreviewLoading, setRepairPreviewLoading] = useState(false);
  const [repairPreviewData, setRepairPreviewData] =
    useState<RepairPreviewResult | null>(null);
  const [repairPreviewProject, setRepairPreviewProject] = useState<{
    id: string;
    name: string;
    path: string;
  } | null>(null);
  const [repairSelectedNames, setRepairSelectedNames] = useState<Set<string>>(
    new Set(),
  );
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

  // Reset search when switching tabs — prevents board search state leaking into dashboard
  useEffect(() => {
    setSearchQuery("");
  }, [workspaceTab, setSearchQuery]);

  const { dupGroups, dupScanning, scanDuplicates, removeFromDupGroups } =
    useDuplicateProjectScan(projects);

  const { aiSummaryMap, aiHealthMap, aiLoadingMap, aiTrendInsightMap } =
    usePortfolioAIAnalysis({
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

  const {
    agentReadinessMap,
    loading: readinessLoading,
    failedCount: readinessFailedCount,
  } = useAgentReadinessBatch({
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
    error: assetError,
    lastUpdatedAt: assetLastUpdatedAt,
  } = usePortfolioAssetSummary(projects, portfolioRefreshToken);

  const [readinessLastUpdatedAt, setReadinessLastUpdatedAt] = useState<
    number | null
  >(null);

  useEffect(() => {
    if (!readinessLoading && projects.length > 0) {
      setReadinessLastUpdatedAt(Date.now());
    }
  }, [
    readinessLoading,
    portfolioRefreshToken,
    agentReadinessMap,
    projects.length,
  ]);

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
      // Step 1: open preview dialog and fetch drift items
      setRepairPreviewProject({
        id: project.id,
        name: project.name,
        path: project.path,
      });
      setRepairPreviewOpen(true);
      setRepairPreviewLoading(true);
      setRepairPreviewData(null);
      setRepairSelectedNames(new Set());

      try {
        const preview = await previewRepairProjectDrift(
          project.path,
          targetApp,
        );
        setRepairPreviewData(preview);

        // auto-select all items by default
        if (preview && preview.items.length > 0) {
          setRepairSelectedNames(
            new Set(preview.items.map((i) => i.check_name)),
          );
        }
      } finally {
        setRepairPreviewLoading(false);
      }
    },
    [targetApp],
  );

  const handleRepairPreviewConfirm = useCallback(
    async (selectedNames: string[]) => {
      if (!repairPreviewProject) return;
      setRepairingProjectId(repairPreviewProject.id);
      setRepairPreviewOpen(false);
      try {
        const result = await repairProjectDrift(
          repairPreviewProject.path,
          targetApp,
          selectedNames.length > 0 ? selectedNames : undefined,
        );
        const ok = showRepairProjectFeedback(result, t);
        if (ok || result) {
          bumpPortfolioRefresh();
        }
      } finally {
        setRepairingProjectId(null);
        setRepairPreviewProject(null);
        setRepairPreviewData(null);
      }
    },
    [repairPreviewProject, targetApp, bumpPortfolioRefresh, t],
  );

  const handleRepairPreviewCancel = useCallback(() => {
    setRepairPreviewOpen(false);
    setRepairPreviewProject(null);
    setRepairPreviewData(null);
    setRepairSelectedNames(new Set());
  }, []);

  const handleRepairToggleItem = useCallback((checkName: string) => {
    setRepairSelectedNames((prev) => {
      const next = new Set(prev);
      if (next.has(checkName)) next.delete(checkName);
      else next.add(checkName);
      return next;
    });
  }, []);

  const handleRepairSelectAll = useCallback(() => {
    if (!repairPreviewData) return;
    setRepairSelectedNames(
      new Set(repairPreviewData.items.map((i) => i.check_name)),
    );
  }, [repairPreviewData]);

  const handleRepairDeselectAll = useCallback(() => {
    setRepairSelectedNames(new Set());
  }, []);

  useEffect(() => {
    if (!projectDetailIntent) return;
    setInternalDetailId(projectDetailIntent.projectId);
  }, [projectDetailIntent]);

  const {
    projectContextsMap,
    portfolioPoints,
    portfolioScoreKind,
    totalCodeLines,
    commitsInWindowMap,
    totalCommitsInWindow,
    averageActivityLabel,
    averageActivityColor,
    // 这里原来还取了 `formatCompactNumber`，用来格式化「总代码行数」那张卡的
    // 数值。那张卡搬去了 TodayWorkspace，格式化跟着一起搬 —— 留在这里就是
    // 一个没人调用的函数。
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
    agentReadinessMap,
    aiConfigured,
    scanning,
    overviewWindowDays,
    getStage,
    t,
  });

  // 14 张按 project.id 索引的平行表在这里合成一张视图表（审查报告 §6.1）：
  // 从此 `.get(id)` 只发生在 `buildProjectViews` 里一处，下游组件收对象而不是
  // 收一把 Map + 一个 id 自己去拼。
  const { viewMap } = useProjectViews({
    projects,
    getStage,
    progressMap,
    codeLinesMap,
    versionMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    contributorsMap,
    weeklyCommitsMap,
    aiSummaryMap,
    aiLoadingMap,
    aiHealthMap,
    aiTrendInsightMap,
    agentReadinessMap,
    assetMap,
    projectContextsMap,
  });

  const groupedViews = useMemo(
    () => ({
      mvp: pickProjectViews(viewMap, grouped.mvp),
      rapid: pickProjectViews(viewMap, grouped.rapid),
      stable: pickProjectViews(viewMap, grouped.stable),
    }),
    [viewMap, grouped],
  );

  const { removeConfirm, handleRemove, confirmRemoveProject, cancelRemove } =
    useKanbanRemoveProject(projects, onProjectRemove, t, removeFromDupGroups);

  const activeDetailId = selectedProjectId ?? internalDetailId;
  const detailView = useMemo(
    () => (activeDetailId ? (viewMap.get(activeDetailId) ?? null) : null),
    [viewMap, activeDetailId],
  );

  const openDetail = (project: Project) => {
    onProjectClick(project);
    setInternalDetailId(project.id);
  };

  const openAiConfig = useCallback(
    (project: Project) => {
      onOpenProjectAiConfig(project.id);
    },
    [onOpenProjectAiConfig],
  );

  const closeDetail = useCallback(() => {
    setInternalDetailId(null);
    onClearSelection?.();
  }, [onClearSelection]);

  const handleOpenFolder = async (path: string) => {
    await revealPathInFolder(path, { alertOnError: true });
  };

  const handleRefresh = () => {
    refreshScan();
    refreshConfig();
    bumpPortfolioRefresh();
  };

  const totalCount = projects.length;

  /**
   * 标签栏与它控制的那块面板必须同生共死：空态下不渲染标签栏，那块内容区
   * 就不能自称 `role="tabpanel"` —— 一个没有任何 tab 指向它的 tabpanel 会
   * 让读屏器报出一个凭空出现的「标签面板」。
   */
  const showWorkspaceTabs = !empty && !!onWorkspaceTabChange;

  // `AICostProvider` 是成本汇总的唯一拉取方：`AICostStrip` / `AINLQueryBar`
  // 从 context 取数，不再各自 `getAICostSummary(30)`（审查报告 §3.1 重复挂载）。
  return (
    <AICostProvider aiConfigured={aiConfigured}>
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
                  : /*
                     * 副标题落审查报告 §2.5 的 B 方案。原文承诺的「风险」与
                     * 「进度」两个词在工作区层面都没有实现：风险只存在于单
                     * 项目按需的 `AIRiskAnalysis`，进度只在 `TodayWorkspace`
                     * 里生成一条 `stage==="mvp" && progress<50` 的文本，四张
                     * SummaryCard 没有一张是进度（§2.4 逐词核查）。
                     * A 方案（「按严重度排出待办」）要等 §2.4 的排序修好
                     * ——`readiness ?? 999` 现在会把从未扫描的项目当成最健康的
                     * 沉到底部——在那之前说「排出待办」同样是空头支票。
                     */
                    t("workspace.subtitle", {
                      defaultValue:
                        "汇总各项目 AI 配置状态与近期活跃度，帮你先看该看的",
                    })}
              </p>
              {showWorkspaceTabs && onWorkspaceTabChange && (
                <div className="mt-3">
                  <WorkspaceTabBar
                    activeTab={workspaceTab}
                    onChange={onWorkspaceTabChange}
                  />
                </div>
              )}
            </div>

            <div className="flex flex-wrap items-center justify-end gap-2 shrink-0">
              {/* 智能周报不再常驻这里 —— 它是一份组合级摘要，却压在三个 Tab
                  头上，在「AI 资产总览」顶部尤其突兀（审查报告 §3.3）。
                  已随「今日工作台」下沉。 */}
              {!empty && aiConfigured && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setRoiPanelOpen(true)}
                >
                  <BarChart3 className="w-4 h-4 mr-1" />
                  {/* 复用 AICostStrip 那条键：同一个入口的同一个名字，
                      两处硬编码迟早会各改各的。 */}
                  {t("ai.cost.roiReport", { defaultValue: "AI 投入报告" })}
                </Button>
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
                  <PopoverContent
                    className="w-80 p-0"
                    align="end"
                    sideOffset={8}
                  >
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

        {/*
         * 三个 Tab 的落地面板（审查报告 §7）。
         *
         * 它不是新加的一层布局——父级是 `overflow-y-auto` 的普通块流，里面
         * 每一段自带 `px-6`，多包一个裸 `<div>` 在视觉上是 0 像素的改动。
         * 加它只为一件事：`role="tab"` 承诺了「我控制着一块内容」，之前那
         * 块内容在 DOM 里没有任何标记，读屏器切换 Tab 后不知道该把用户带到
         * 哪里去。
         *
         * 没有拆成三个 panel（每个 Tab 一个）：三段内容中间夹着
         * `PortfolioDataNotice` 与空态/无结果这两块**跨 Tab 共享**的兄弟节点，
         * 拆开就要把它们复制三份或者提取组件，改动面远大于收益。共享单面板
         * + `aria-labelledby` 跟着当前 Tab 走，读屏器念出来的是一样的。
         *
         * 不给 tabIndex={0}：面板里从来都有可聚焦元素，加了只会在 Tab 键序
         * 上多出一个空站。
         */}
        <div
          id={showWorkspaceTabs ? WORKSPACE_TABPANEL_ID : undefined}
          role={showWorkspaceTabs ? "tabpanel" : undefined}
          aria-labelledby={
            showWorkspaceTabs ? workspaceTabId(workspaceTab) : undefined
          }
        >
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

          {/* 数据不完整告警放在 Tab 内容之前、三个 Tab 之上：assetMap 与
            agentReadinessMap 同时喂三个 Tab，读不到时三处都会退化成
            「未扫 / 未配置 / 缺 MCP」（审查报告 §5.6）。 */}
          {!empty && (
            <PortfolioDataNotice
              assetError={assetError}
              readinessFailedCount={readinessFailedCount}
              totalProjects={projects.length}
              refreshing={portfolioDataRefreshing}
              className="mx-6 mb-2"
              onRetry={bumpPortfolioRefresh}
            />
          )}

          {!empty && workspaceTab === "dashboard" && (
            // 「今日工作台」只回答一个问题：今天该动哪个项目（审查报告 §3.3）。
            // 因此这里不放 GovernanceDashboard（→ AI 资产总览）、
            // PortfolioMatrix（→ 项目看板，那是项目彼此的关系，不是今天的事）。
            <div className="px-6 pb-6 space-y-4">
              <DashboardOnboarding />
              {/*
              成本条归这里而不是项目看板：它答的是「本期烧了多少、还剩多少
              预算」，是每天开机第一眼要扫的数，跟「今天该动哪个项目」同一
              个决策；项目看板那边留纯粹的项目关系视图。§3.1 的重复挂载已经
              删掉了另一份，全应用只此一处，别再往别的 Tab 上加。
            */}
              <AICostStrip
                aiConfigured={aiConfigured}
                projectCount={totalCount}
                onOpenRoiPanel={() => setRoiPanelOpen(true)}
              />
              {/*
              问答栏紧跟成本条，不再留在这个 Tab 的最底下。

              `AICostStrip` 在没配 AI 时整条 `return null`，理由写的是「同一屏的
              AINLQueryBar 已经承担了『去设置里配置』的引导」—— 但那句话当时
              并不成立：问答栏在阶段分布下面，和成本条隔着五块面板、约一屏半。
              于是没配 AI 的用户在页顶什么也看不到，唯一的解释躺在页尾。
              把两者放到一起，那条注释才对得上：没配时这里出现一次引导，配了
              之后这里出现成本条 + 问答栏，都在开机第一眼扫得到的位置。
            */}
              <AINLQueryBar
                projectContexts={Array.from(projectContextsMap.values())}
                aiConfigured={aiConfigured}
                projectCount={totalCount}
              />
              {aiConfigured && (
                <div className="flex flex-wrap items-center justify-end gap-2">
                  <AIWeeklyReport
                    projectContexts={Array.from(projectContextsMap.values())}
                    aiConfigured={aiConfigured}
                  />
                </div>
              )}
              {/*
              摘要区在前、清单在后：先看「一共多少、整体多健康」，再看「具体
              该动谁」。TodayWorkspace 现在只出数不出列表，所以它是这一屏的
              抬头，而不是夹在两块清单中间的第三块。
            */}
              <TodayWorkspace
                projects={projects}
                getStage={getStage}
                progressMap={progressMap}
                agentReadinessMap={agentReadinessMap}
                commitsInWindowMap={commitsInWindowMap}
                overviewWindowDays={overviewWindowDays}
                onOverviewWindowDaysChange={setOverviewWindowDays}
                totalCodeLines={totalCodeLines}
                totalCommitsInWindow={totalCommitsInWindow}
                averageActivityLabel={averageActivityLabel}
                averageActivityColor={averageActivityColor}
                lastUpdatedAt={portfolioLastUpdatedAt}
                isRefreshing={portfolioDataRefreshing}
              />
              {/*
              这个 Tab 里唯一一份「需要动手的项目」清单。TodayWorkspace 曾经
              在下方另挂一份「建议优先处理」，两份读同一个 agentReadinessMap
              却各算各的理由、各排各的序（§3.1）。留下这一份是因为它多给了
              交通灯六档分级和修复入口。
            */}
              <PortfolioHealthSummary
                projects={projects}
                agentReadinessMap={agentReadinessMap}
                assetMap={assetMap}
                loading={readinessLoading}
                onOpenProject={openDetail}
                onOpenProjectAiConfig={openAiConfig}
                // 与下方 PortfolioDriftSummary 接同一条修复链路：先拉预览、由
                // 用户勾选确认后才写盘。共用 repairingProjectId，避免两个入口
                // 对同一项目并发提交（审查报告 §4.3）。
                onRepairProject={(p) => void handleRepairProjectDrift(p)}
                repairingProjectId={repairingProjectId}
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
              <div className="rounded-xl border border-border/60 bg-card/30 p-4">
                <div className="flex items-center gap-6">
                  <span className="text-xs font-medium text-muted-foreground">
                    {t("kanban.stageDistribution", {
                      defaultValue: "阶段分布",
                    })}
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
                      <span
                        className={`w-2.5 h-2.5 rounded-full ${item.color}`}
                      />
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
            </div>
          )}

          {!empty && workspaceTab === "board" && (
            // 「项目看板」是项目集合的空间视图：卡片网格 + 四象限图（审查报告 §3.3）。
            // AICostStrip 与 AINLQueryBar 的重复挂载都已删，两者现在都归「今日工作台」。
            <div className="px-6 pb-2 space-y-2">
              {/*
              这里曾经写 `aiConfigured && …`（审查报告 §2.5）。矩阵不调用任何
              模型，纯 recharts 画本地已有的数字，却因为组件名带 `AI` 前缀被
              一路门到没配 Key 的用户看不见。门控删了，"有没有点"由数据自己
              决定 —— `portfolioPoints` 现在只装真实存在的分数，一个都没有时
              组件返回 null，不会留下一张空图。
            */}
              {portfolioPoints.length > 0 && (
                <PortfolioMatrix
                  points={portfolioPoints}
                  scoreKind={portfolioScoreKind}
                />
              )}
            </div>
          )}

          {!empty && workspaceTab === "assetsMatrix" && (
            // 「AI 资产总览」是配置落地的唯一权威视图（审查报告 §3.3），
            // 因此 GovernanceDashboard（配置生效率）从「今日工作台」收到这里 ——
            // 生效率和明细矩阵是同一个问题的两个粒度，本就不该隔着一个 Tab。
            <div className="px-6 pb-8 space-y-4">
              <GovernanceDashboard
                projects={projects}
                agentReadinessMap={agentReadinessMap}
                targetApp={targetApp}
                loading={readinessLoading}
              />
              <ProjectAssetsMatrix
                projects={projects}
                getStage={getStage}
                agentReadinessMap={agentReadinessMap}
                assetMap={assetMap}
                loading={readinessLoading}
                onOpenProject={openDetail}
                onOpenProjectAiConfig={openAiConfig}
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
                {(["mvp", "rapid", "stable"] as const).map((stageKey) => (
                  <StageSection
                    key={stageKey}
                    stage={stageKey}
                    views={groupedViews[stageKey]}
                    onProjectClick={openDetail}
                    onProjectRemove={handleRemove}
                    onStageChange={(projectId, next) =>
                      setStage(projectId, next)
                    }
                    onOpenFolder={handleOpenFolder}
                  />
                ))}
              </>
            ) : null}
          </div>
        </div>

        <AnimatePresence>
          {detailView && (
            <ProjectDetailSheet
              view={detailView}
              aiConfigured={aiConfigured}
              onStageChange={(s) => setStage(detailView.id, s)}
              onProgressChange={(p) => setProjectProgress(detailView.id, p)}
              onClose={closeDetail}
              onOpenAiConfig={() => onOpenProjectAiConfig(detailView.id)}
              targetApp={targetApp}
            />
          )}
        </AnimatePresence>
        <AICostPanel open={roiPanelOpen} onOpenChange={setRoiPanelOpen} />
        <RepairPreviewDialog
          open={repairPreviewOpen}
          loading={repairPreviewLoading}
          preview={repairPreviewData}
          projectName={repairPreviewProject?.name ?? ""}
          repairing={repairingProjectId === repairPreviewProject?.id}
          selectedNames={repairSelectedNames}
          onSelectAll={handleRepairSelectAll}
          onDeselectAll={handleRepairDeselectAll}
          onToggleItem={handleRepairToggleItem}
          onConfirm={(names) => void handleRepairPreviewConfirm(names)}
          onCancel={handleRepairPreviewCancel}
        />
      </motion.div>
    </AICostProvider>
  );
}
