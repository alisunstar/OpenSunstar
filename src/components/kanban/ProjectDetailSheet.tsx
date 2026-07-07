import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { BarChart3, Coins, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { StagePicker } from "./StagePicker";
import { AgentReadinessPanel } from "./AgentReadinessPanel";
import { ProjectAssetPanel } from "@/components/projects/ProjectAssetPanel";
import { ProjectBlueprintPanel } from "@/components/projects/ProjectBlueprintPanel";
import { ProjectFlowOrchestratorPanel } from "@/components/projects/ProjectFlowOrchestratorPanel";
import type { StageKey } from "@/hooks/useProjectStages";
import type { Project } from "@/types/project";
import type { CodeLineResult, Contributor } from "@/api/codeMetrics";
import type { ProjectGitInfo } from "@/api/projectGit";
import type { ProjectContextInput } from "@/api/aiInsight";
import { AIRiskAnalysis } from "./AIRiskAnalysis";
import { CommitTrendChart } from "./CommitTrendChart";
import { useAIRisk, useAgentReadiness } from "@/hooks/useAIInsight";
import { repairAssetDrift } from "@/api/aiInsight";
import { showRepairAssetFeedback } from "@/lib/repairFeedback";
import { useAIRoiReport } from "@/hooks/useAIRoiReport";
import { useAICost } from "@/contexts/AICostContext";
import { activityTier7d, formatCompactNumber } from "@/lib/portfolioMetrics";
import { formatAiCostYuan, formatAiTokens } from "@/lib/aiCostFormat";
import type { PageView } from "@/App";
import type { AppId } from "@/lib/api";
import type { ProjectAssetSection } from "@/lib/readinessActions";
import { cn } from "@/lib/utils";

import type { ProjectDetailTab } from "@/types/projectDetail";

export type DetailTab = ProjectDetailTab;

export interface ProjectDetailSheetProps {
  project: Project;
  stage: StageKey;
  progress: number | undefined;
  codeLines?: CodeLineResult;
  version?: string;
  gitInfo?: ProjectGitInfo;
  activity?: number;
  activity30d?: number;
  contributors?: Contributor[];
  weeklyCommits?: number[];
  projectContext: ProjectContextInput | null;
  aiTrendInsight: string | null;
  aiConfigured: boolean;
  onStageChange: (stage: StageKey) => void;
  onProgressChange: (progress: number) => void;
  onClose: () => void;
  onNavigate?: (view: PageView) => void;
  initialTab?: DetailTab;
  onPortfolioConfigChanged?: () => void;
  targetApp?: AppId;
}

export function ProjectDetailSheet({
  project,
  stage,
  progress,
  codeLines,
  version,
  gitInfo,
  activity,
  activity30d,
  contributors,
  weeklyCommits,
  projectContext,
  aiTrendInsight,
  aiConfigured,
  onStageChange,
  onProgressChange,
  onClose,
  onNavigate,
  initialTab = "overview",
  onPortfolioConfigChanged,
  targetApp = "claude",
}: ProjectDetailSheetProps) {
  const { t: tr } = useTranslation();
  const sheetRef = useRef<HTMLDivElement>(null);
  const [activeTab, setActiveTab] = useState<DetailTab>(initialTab);
  const [scrollSection, setScrollSection] = useState<ProjectAssetSection | null>(
    null,
  );
  const [repairingCheckName, setRepairingCheckName] = useState<string | null>(null);

  useEffect(() => {
    setActiveTab(initialTab);
    setScrollSection(null);
  }, [project.id, initialTab]);

  useEffect(() => {
    const sheet = sheetRef.current;
    if (!sheet) return;

    const focusableSelector =
      'a[href],button:not([disabled]),textarea,input,select,[tabindex]:not([tabindex="-1"])';
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key !== "Tab") return;
      const focusable = sheet.querySelectorAll<HTMLElement>(focusableSelector);
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    sheet.focus();
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const riskEnabled = aiConfigured && projectContext !== null;
  const riskHook = useAIRisk({
    projectId: project.id,
    context: projectContext,
    enabled: riskEnabled,
  });
  const [riskLoaded, setRiskLoaded] = useState(false);

  const handleRiskRefresh = () => {
    setRiskLoaded(true);
    riskHook.refresh();
  };

  const { data: readinessData, isLoading: readinessLoading, refresh: refreshReadiness, scanEffective } =
    useAgentReadiness(project.path, true, targetApp);
  const { refreshToken } = useAICost();
  const { report: roiReport } = useAIRoiReport(30, refreshToken);
  const projectRoi = roiReport?.by_project.find((p) => p.project_id === project.id);

  const openAssetsTab = useCallback((section?: ProjectAssetSection) => {
    setActiveTab("aiAssets");
    setScrollSection(section ?? null);
  }, []);

  const handleNavigate = useCallback(
    (view: PageView) => {
      onNavigate?.(view);
      onClose();
    },
    [onNavigate, onClose],
  );

  const handleConfigChanged = useCallback(() => {
    scanEffective();
    onPortfolioConfigChanged?.();
  }, [scanEffective, onPortfolioConfigChanged]);

  const handleRepairDrift = useCallback(
    async (checkName: string) => {
      setRepairingCheckName(checkName);
      try {
        const result = await repairAssetDrift(project.path, checkName, targetApp);
        const ok = showRepairAssetFeedback(result, tr);
        if (ok || result) {
          scanEffective();
          onPortfolioConfigChanged?.();
        }
      } finally {
        setRepairingCheckName(null);
      }
    },
    [project.path, targetApp, scanEffective, onPortfolioConfigChanged, tr],
  );

  function activityLabel(count: number): { text: string; color: string } {
    const tier = activityTier7d(count);
    if (tier >= 4)
      return {
        text: tr("kanban.activity.veryHigh", { defaultValue: "很高" }),
        color: "text-emerald-500",
      };
    if (tier >= 3)
      return {
        text: tr("kanban.activity.high", { defaultValue: "高" }),
        color: "text-emerald-400",
      };
    if (tier >= 2)
      return {
        text: tr("kanban.activity.medium", { defaultValue: "中" }),
        color: "text-amber-500",
      };
    return {
      text: tr("kanban.activity.low", { defaultValue: "低" }),
      color: "text-muted-foreground",
    };
  }

  return (
    <motion.div
      className="fixed inset-0 z-[60] flex justify-end"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label={project.name}
    >
      <div className="absolute inset-0 bg-black/20 backdrop-blur-sm" />

      <motion.div
        ref={sheetRef}
        tabIndex={-1}
        className="relative w-[480px] max-w-[90vw] h-full bg-background border-l border-border shadow-2xl overflow-y-auto outline-none"
        initial={{ x: "100%" }}
        animate={{ x: 0 }}
        exit={{ x: "100%" }}
        transition={{ type: "spring", damping: 25, stiffness: 200 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-6 space-y-6">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-lg font-semibold text-foreground">
                {project.name}
              </h2>
              <p className="text-sm text-muted-foreground font-mono mt-1">
                {project.path}
              </p>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={onClose}
              aria-label="关闭详情"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          <div
            className="flex gap-1 rounded-lg border border-border/50 bg-muted/20 p-0.5"
            role="tablist"
            aria-label={tr("kanban.detail.tabs", { defaultValue: "项目详情" })}
          >
            {(
              [
                ["overview", tr("kanban.detail.overview", { defaultValue: "概览" })],
                [
                  "aiAssets",
                  tr("kanban.detail.aiAssets", { defaultValue: "AI 资产配置" }),
                ],
              ] as const
            ).map(([tab, label]) => (
              <button
                key={tab}
                type="button"
                role="tab"
                aria-selected={activeTab === tab}
                className={cn(
                  "flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
                  activeTab === tab
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
                onClick={() => setActiveTab(tab)}
              >
                {label}
              </button>
            ))}
          </div>

          {activeTab === "overview" && (
            <>
          <div>
            <h3 className="text-sm font-semibold text-foreground mb-2">
              {tr("kanban.stageLabel", { defaultValue: "项目阶段" })}
            </h3>
            <p className="text-xs text-muted-foreground mb-3">
              {tr("kanban.stageHint", {
                defaultValue: "选择项目当前所处的开发阶段",
              })}
            </p>
            <StagePicker value={stage} onChange={onStageChange} />
          </div>

          {project.description && (
            <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
              <h3 className="text-sm font-semibold text-foreground mb-1">
                {tr("projects.description", { defaultValue: "项目描述" })}
              </h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {project.description}
              </p>
            </div>
          )}

          {stage === "mvp" && (
            <div className="rounded-xl border border-purple-500/20 bg-purple-500/5 p-4">
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-semibold text-foreground">
                  {tr("kanban.progress", { defaultValue: "开发进度" })}
                </h3>
                <span className="text-lg font-bold text-purple-600 dark:text-purple-400 tabular-nums">
                  {progress ?? 0}%
                </span>
              </div>
              <input
                type="range"
                min={0}
                max={100}
                step={5}
                value={progress ?? 0}
                onChange={(e) => onProgressChange(Number(e.target.value))}
                className="w-full h-1.5 rounded-full appearance-none bg-muted/50 cursor-pointer
                  [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4
                  [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full
                  [&::-webkit-slider-thumb]:bg-purple-500 [&::-webkit-slider-thumb]:shadow-md
                  [&::-webkit-slider-thumb]:cursor-pointer"
              />
              <div className="flex justify-between mt-1.5">
                <span className="text-[10px] text-muted-foreground">0%</span>
                <span className="text-[10px] text-muted-foreground">100%</span>
              </div>
            </div>
          )}

          <div className="rounded-xl border border-border/60 bg-muted/20 p-4 space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">
                {tr("kanban.addedAt", { defaultValue: "添加时间" })}
              </span>
              <span className="text-foreground font-medium">
                {new Date(project.addedAt).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">
                {tr("kanban.projectPath", { defaultValue: "本地路径" })}
              </span>
              <span className="text-foreground font-medium font-mono text-xs truncate max-w-[250px]">
                {project.path}
              </span>
            </div>
          </div>

          <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
            <div className="flex items-center gap-2 mb-3">
              <BarChart3 className="w-4 h-4 text-blue-500" />
              <h3 className="text-sm font-semibold text-foreground">
                {tr("kanban.metrics", { defaultValue: "代码指标" })}
              </h3>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.codeLines", { defaultValue: "代码行数" })}
                </span>
                <span className="text-lg font-bold text-foreground mt-1 tabular-nums">
                  {codeLines ? formatCompactNumber(codeLines.code_lines) : "—"}
                </span>
                {codeLines && (
                  <span className="text-[10px] text-muted-foreground/60">
                    {codeLines.files}{" "}
                    {tr("kanban.files", { defaultValue: "个文件" })}
                    {" · "}
                    {codeLines.languages.length}{" "}
                    {tr("kanban.languages", { defaultValue: "种语言" })}
                  </span>
                )}
              </div>

              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.version", { defaultValue: "版本" })}
                </span>
                <span className="text-lg font-bold text-foreground mt-1">
                  {version || "—"}
                </span>
              </div>

              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.commits7d", {
                    defaultValue: "近 7 天提交",
                  })}
                </span>
                <span
                  className={`text-lg font-bold mt-1 tabular-nums ${
                    typeof activity === "number"
                      ? activityLabel(activity).color
                      : "text-foreground/25"
                  }`}
                >
                  {typeof activity === "number" ? activity : "—"}
                </span>
                {typeof activity === "number" && (
                  <span className="text-[10px] text-muted-foreground/60 tabular-nums">
                    {activityLabel(activity).text}
                    {typeof activity30d === "number"
                      ? ` · 30天 ${activity30d}次`
                      : ""}
                  </span>
                )}
              </div>

              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.contributors", { defaultValue: "贡献者" })}
                </span>
                <span className="text-lg font-bold text-foreground mt-1 tabular-nums">
                  {contributors ? contributors.length : "—"}
                </span>
                {contributors && contributors.length > 0 && (
                  <span className="text-[10px] text-muted-foreground/60 truncate max-w-[120px]">
                    {contributors
                      .slice(0, 3)
                      .map((c) => c.name)
                      .join(", ")}
                  </span>
                )}
              </div>
            </div>

            {codeLines && codeLines.languages.length > 0 && (
              <div className="mt-3 pt-3 border-t border-border/40">
                <span className="text-xs text-muted-foreground mb-2 block">
                  {tr("kanban.languageBreakdown", { defaultValue: "语言分布" })}
                </span>
                <div className="space-y-1.5">
                  {codeLines.languages.slice(0, 6).map((lang) => {
                    const pct =
                      codeLines.code_lines > 0
                        ? Math.round(
                            (lang.code_lines / codeLines.code_lines) * 100,
                          )
                        : 0;
                    return (
                      <div
                        key={lang.language}
                        className="flex items-center gap-2 text-xs"
                      >
                        <span className="w-16 text-muted-foreground truncate">
                          {lang.language}
                        </span>
                        <div className="flex-1 h-1.5 rounded-full bg-muted/50 overflow-hidden">
                          <div
                            className="h-full rounded-full bg-blue-500/60"
                            style={{ width: `${Math.max(pct, 2)}%` }}
                          />
                        </div>
                        <span className="w-10 text-right tabular-nums text-muted-foreground">
                          {pct}%
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          {gitInfo && gitInfo.is_repo && (
            <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
              <h3 className="text-sm font-semibold text-foreground mb-3">
                {tr("kanban.gitInfo", { defaultValue: "Git 仓库" })}
              </h3>
              <div className="space-y-2 text-sm">
                {gitInfo.branch && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      {tr("kanban.branch", { defaultValue: "当前分支" })}
                    </span>
                    <span className="text-foreground font-mono font-medium">
                      {gitInfo.branch}
                    </span>
                  </div>
                )}
                {gitInfo.remote_url && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      {tr("kanban.remote", { defaultValue: "远程地址" })}
                    </span>
                    <span className="text-foreground font-mono text-xs truncate max-w-[240px]">
                      {gitInfo.remote_url}
                    </span>
                  </div>
                )}
                {gitInfo.last_commit_hash && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      {tr("kanban.lastCommit", { defaultValue: "最近提交" })}
                    </span>
                    <span className="text-foreground font-mono text-xs">
                      {gitInfo.last_commit_hash.slice(0, 7)}
                    </span>
                  </div>
                )}
                {gitInfo.last_commit_message && (
                  <p className="text-xs text-muted-foreground/70 leading-relaxed pt-1 border-t border-border/30">
                    {gitInfo.last_commit_message}
                  </p>
                )}
                {gitInfo.last_commit_author && gitInfo.last_commit_date && (
                  <p className="text-xs text-muted-foreground/50">
                    {gitInfo.last_commit_author} · {gitInfo.last_commit_date}
                  </p>
                )}
              </div>
            </div>
          )}

          {!codeLines && !gitInfo?.is_repo && (
            <p className="text-xs text-muted-foreground/60 text-center py-2">
              {tr("kanban.scanHint", {
                defaultValue:
                  "点击页面顶部「刷新指标」按钮扫描项目代码数据",
              })}
            </p>
          )}

          {weeklyCommits && weeklyCommits.length > 0 && (
            <CommitTrendChart
              weeklyCommits={weeklyCommits}
              projectName={project.name}
              aiInsight={aiTrendInsight}
              projectId={project.id}
            />
          )}

          {aiConfigured && (
            <div className="rounded-xl border border-border/60 bg-card/40 p-4">
              <div className="flex items-center gap-2 mb-2">
                <Coins className="h-4 w-4 text-primary" />
                <h3 className="text-sm font-semibold text-foreground">
                  本项目 AI 投入（30 天）
                </h3>
              </div>
              {projectRoi ? (
                <div className="space-y-1.5 text-xs text-muted-foreground">
                  <p>
                    消耗{" "}
                    <span className="font-medium text-foreground tabular-nums">
                      {formatAiCostYuan(projectRoi.cost)}
                    </span>
                    <span className="text-muted-foreground/50 ml-1 tabular-nums">
                      · {formatAiTokens(projectRoi.tokens)}
                    </span>
                  </p>
                  <p>
                    {projectRoi.insight_count} 次分析
                    {projectRoi.risk_count > 0 && (
                      <span className="text-amber-500/90">
                        {" "}
                        · 发现 {projectRoi.risk_count} 项风险
                      </span>
                    )}
                    {projectRoi.useful_count > 0 && (
                      <span className="text-emerald-500/90">
                        {" "}
                        · {projectRoi.useful_count} 次标记有用
                      </span>
                    )}
                  </p>
                  {projectRoi.top_risks.length > 0 && (
                    <ul className="list-disc pl-4 text-[11px] text-muted-foreground/70 space-y-0.5 pt-1">
                      {projectRoi.top_risks.map((r, i) => (
                        <li key={i}>{r}</li>
                      ))}
                    </ul>
                  )}
                </div>
              ) : (
                <p className="text-xs text-muted-foreground/60">
                  近 30 天暂无该项目的 AI 调用记录。
                </p>
              )}
            </div>
          )}

          {aiConfigured && (
            <AIRiskAnalysis
              data={riskHook.data}
              isLoading={riskHook.isLoading}
              onRefresh={handleRiskRefresh}
              hasLoaded={riskLoaded}
              projectId={project.id}
            />
          )}

          <AgentReadinessPanel
            data={readinessData}
            isLoading={readinessLoading}
            onRefresh={refreshReadiness}
            onScanEffective={scanEffective}
            onRepairDrift={handleRepairDrift}
            repairingCheckName={repairingCheckName}
            onOpenProjectAssets={openAssetsTab}
            onNavigate={handleNavigate}
            compact
          />
            </>
          )}

          {activeTab === "aiAssets" && (
            <div className="space-y-6">
              <ProjectBlueprintPanel
                projectId={project.id}
                onApplied={() => {
                  void refreshReadiness();
                  handleConfigChanged();
                }}
              />
              <ProjectFlowOrchestratorPanel projectId={project.id} />
              <AgentReadinessPanel
                data={readinessData}
                isLoading={readinessLoading}
                onRefresh={refreshReadiness}
                onScanEffective={scanEffective}
                onRepairDrift={handleRepairDrift}
                repairingCheckName={repairingCheckName}
                onOpenProjectAssets={openAssetsTab}
                onNavigate={handleNavigate}
              />
              <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
                <h3 className="text-sm font-semibold text-foreground mb-1">
                  {tr("kanban.detail.projectAssets", {
                    defaultValue: "本项目启用的资产",
                  })}
                </h3>
                <p className="text-[11px] text-muted-foreground mb-4">
                  {tr("kanban.detail.projectAssetsHint", {
                    defaultValue:
                      "勾选后仅对当前项目生效；与侧栏全局库双向同步。",
                  })}
                </p>
                <ProjectAssetPanel
                  projectId={project.id}
                  scrollToSection={scrollSection}
                  onConfigChanged={handleConfigChanged}
                  onNavigateToGlobal={handleNavigate}
                />
              </div>
            </div>
          )}
        </div>
      </motion.div>
    </motion.div>
  );
}
