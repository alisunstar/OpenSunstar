import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { BarChart3, Coins, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { StagePicker } from "./StagePicker";
import { AgentReadinessPanel } from "./AgentReadinessPanel";
import type { StageKey } from "@/hooks/useProjectStages";
import type { ProjectView } from "@/types/projectView";
import { AIRiskAnalysis } from "./AIRiskAnalysis";
import { CommitTrendChart } from "./CommitTrendChart";
import { useAIRisk, useAgentReadiness } from "@/hooks/useAIInsight";
import { useAIRoiReport } from "@/hooks/useAIRoiReport";
import { useAICost } from "@/contexts/AICostContext";
import { isKeyEventOwnedByNestedLayer } from "@/lib/dialogEscape";
import { activityTier7d, formatCompactNumber } from "@/lib/portfolioMetrics";
import { formatAiCostYuan, formatAiTokens } from "@/lib/aiCostFormat";
import type { AppId } from "@/lib/api";

export interface ProjectDetailSheetProps {
  /**
   * 这个项目在组合视图里的那一行（审查报告 §6.1）。
   *
   * 此前抽屉收 20 个 prop，其中 12 个是 `KanbanPage` 现场从 12 张平行 Map
   * 里 `.get(detailProject.id)` 抠出来的：`detailProject.id` 在调用点连写
   * 12 遍，抠错一个就是抽屉里混进别的项目的数据 —— 而这既没有类型能拦，
   * 也没有测试能看出来。
   */
  view: ProjectView;
  aiConfigured: boolean;
  onStageChange: (stage: StageKey) => void;
  onProgressChange: (progress: number) => void;
  onClose: () => void;
  /**
   * 去「AI资产配置」页配这个项目。抽屉曾经自己装着那一整块（第二个 Tab），
   * 现在它只负责把用户送过去 —— 480px 装不下要动手勾选的东西。
   */
  onOpenAiConfig?: () => void;
  targetApp?: AppId;
}

export function ProjectDetailSheet({
  view,
  aiConfigured,
  onStageChange,
  onProgressChange,
  onClose,
  onOpenAiConfig,
  targetApp = "claude",
}: ProjectDetailSheetProps) {
  const { t: tr } = useTranslation();
  const {
    project,
    stage,
    progress,
    codeLines,
    version,
    gitInfo,
    commits7d: activity,
    commits30d: activity30d,
    contributors,
    weeklyCommits,
    context: projectContext,
    aiTrendInsight,
  } = view;
  const sheetRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const sheet = sheetRef.current;
    if (!sheet) return;

    const focusableSelector =
      'a[href],button:not([disabled]),textarea,input,select,[tabindex]:not([tabindex="-1"])';
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape" && e.key !== "Tab") return;
      // 抽屉内部还会再弹出高危确认框（修复配置漂移会覆盖用户在 IDE/终端手改的
      // 内容）。Esc 的语义是「撤销最上面那一层」，不能一路穿透把抽屉也关掉；
      // Tab 焦点环同理，此时应由弹层自己托管。
      if (isKeyEventOwnedByNestedLayer(e, sheet)) return;
      if (e.key === "Escape") {
        onClose();
        return;
      }
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

  const { data: readinessData, isLoading: readinessLoading } =
    useAgentReadiness(project.path, true, targetApp);
  const { refreshToken } = useAICost();
  const { report: roiReport } = useAIRoiReport(30, refreshToken);
  const projectRoi = roiReport?.by_project.find(
    (p) => p.project_id === project.id,
  );

  /*
   * 就绪度面板里那些「去配 MCP / 去配 Skills」的动作，以前是在抽屉内部切到第二个
   * Tab 并滚到对应小节。现在配置页在抽屉外面，跳过去就得先把抽屉关掉 ——
   * 否则用户落在新页面上，头顶还盖着一层半透明遮罩。
   *
   * 代价是丢掉了「滚到某一小节」这个精度：`section` 参数在这里被吞掉了。
   * 要留住它得把 section 一路塞进 PageView，而 PageView 是个不带参数的扁平
   * 枚举（`navigation.ts`）—— 那是另一件事，不在这轮范围里。
   */
  const handleOpenAiConfig = useCallback(() => {
    onOpenAiConfig?.();
    onClose();
  }, [onOpenAiConfig, onClose]);

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

          {/*
           * 这里原来有一条 Tab 栏（概览 / AI 资产配置）。第二个 Tab 已经升格成
           * 工作区二级页「AI资产配置」—— 抽屉只剩一种形态，一个 Tab 的 Tab 栏
           * 只是噪音，所以整条删掉，内容直接铺开。
           */}
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
                  {tr("kanban.metric.codeLines", {
                    defaultValue: "代码行数",
                  })}
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
                  {tr("kanban.metric.contributors", {
                    defaultValue: "贡献者",
                  })}
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
                  {tr("kanban.languageBreakdown", {
                    defaultValue: "语言分布",
                  })}
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
                      {tr("kanban.lastCommit", {
                        defaultValue: "最近提交",
                      })}
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
                defaultValue: "点击页面顶部「刷新指标」按钮扫描项目代码数据",
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

          {/*
           * compact 版就绪度：只报「缺什么」，不提供勾选。真正动手配的那一整块
           * （蓝图 / 逐项就绪 / 8 类资产关联）在「AI资产配置」页，
           * 面板只留摘要和一个明确入口，并在跳转时关掉抽屉。
           */}
          <AgentReadinessPanel
            data={readinessData}
            isLoading={readinessLoading}
            onOpenProjectAssets={handleOpenAiConfig}
            compact
          />
        </div>
      </motion.div>
    </motion.div>
  );
}
