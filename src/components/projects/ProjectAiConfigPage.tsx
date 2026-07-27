import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { FolderPlus, RefreshCw, Sparkles, Workflow } from "lucide-react";

import { Button } from "@/components/ui/button";
import { AgentReadinessPanel } from "@/components/kanban/AgentReadinessPanel";
import { ProjectAssetPanel } from "@/components/projects/ProjectAssetPanel";
import { ProjectBlueprintPanel } from "@/components/projects/ProjectBlueprintPanel";
import { useAgentReadiness } from "@/hooks/useAIInsight";
import { repairAssetDrift } from "@/api/aiInsight";
import { showRepairAssetFeedback } from "@/lib/repairFeedback";
import type { PageView } from "@/app/navigation";
import type { AppId } from "@/lib/api";
import type { ProjectAssetSection } from "@/lib/readinessActions";
import type { Project } from "@/types/project";

export interface ProjectAiConfigPageProps {
  projects: Project[];
  /**
   * 全应用共享的「当前项目」。这一页不另存一份 —— 侧栏、看板、抽屉都读同一个
   * `App.tsx` 里的 `selectedProjectId`，多存一份就会出现「侧栏高亮 A、这一页在
   * 配 B」的错位。
   */
  selectedProjectId: string | null;
  onSelectProject: (projectId: string) => void;
  onNavigate?: (view: PageView) => void;
  onAddProject?: () => void;
  /** 配置落盘后通知外部重扫（看板的就绪度批量结果会因此失效）。 */
  onConfigChanged?: () => void;
  targetApp?: AppId;
}

export function ProjectAiConfigPage({
  projects,
  selectedProjectId,
  onSelectProject,
  onNavigate,
  onAddProject,
  onConfigChanged,
  targetApp = "claude",
}: ProjectAiConfigPageProps) {
  const { t } = useTranslation();
  const [scrollSection, setScrollSection] =
    useState<ProjectAssetSection | null>(null);
  const [repairingCheckName, setRepairingCheckName] = useState<string | null>(
    null,
  );

  const project =
    projects.find((p) => p.id === selectedProjectId) ?? projects[0] ?? null;

  /*
   * 从侧栏直接点进来时可能一个项目都没选过。这一页没有项目就完全无事可做，
   * 所以替用户认领第一个 —— 但认领的是**共享的**那份状态，而不是页内私有的
   * 一份，否则离开这一页后「当前项目」会莫名其妙地回退。
   */
  useEffect(() => {
    if (selectedProjectId === null && projects.length > 0) {
      onSelectProject(projects[0].id);
    }
  }, [selectedProjectId, projects, onSelectProject]);

  useEffect(() => {
    setScrollSection(null);
  }, [project?.id]);

  const {
    data: readinessData,
    isLoading: readinessLoading,
    refresh: refreshReadiness,
    scanEffective,
  } = useAgentReadiness(project?.path ?? null, project !== null, targetApp);

  const handleConfigChanged = useCallback(() => {
    scanEffective();
    onConfigChanged?.();
  }, [scanEffective, onConfigChanged]);

  const handleRepairDrift = useCallback(
    async (checkName: string) => {
      if (!project) return;
      setRepairingCheckName(checkName);
      try {
        const result = await repairAssetDrift(
          project.path,
          checkName,
          targetApp,
        );
        const ok = showRepairAssetFeedback(result, t);
        if (ok || result) {
          scanEffective();
          onConfigChanged?.();
        }
      } finally {
        setRepairingCheckName(null);
      }
    },
    [project, targetApp, scanEffective, onConfigChanged, t],
  );

  if (projects.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 px-6 text-center">
        <Sparkles className="h-8 w-8 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">
          {t("projectAiConfig.emptyProjects", {
            defaultValue: "还没有添加项目，先添加一个再来配置 AI 资产。",
          })}
        </p>
        {onAddProject && (
          <Button size="sm" onClick={onAddProject} className="rounded-lg">
            <FolderPlus className="h-3.5 w-3.5 mr-1.5" />
            {t("sidebar.addProject", { defaultValue: "添加项目" })}
          </Button>
        )}
      </div>
    );
  }

  if (!project) return null;

  return (
    <motion.div
      className="flex-1 overflow-y-auto"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
    >
      <div className="sticky top-0 z-20 border-b border-border/30 bg-background/95 backdrop-blur-sm px-6 pt-6 pb-4">
        <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
              <Sparkles className="w-5 h-5 text-primary shrink-0" />
              {t("projectAiConfig.title", { defaultValue: "项目 · AI 配置" })}
            </h2>
            <p className="text-sm text-muted-foreground mt-1">
              {t("projectAiConfig.subtitle", {
                defaultValue:
                  "为单个项目落地 AI 资产：套用蓝图、检查就绪度、勾选生效的资产。侧栏「Agent 配置」管的是全局库，这里管它们在本项目上的关联。",
              })}
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <label className="sr-only" htmlFor="project-ai-config-switcher">
              {t("projectAiConfig.switcher", { defaultValue: "当前配置项目" })}
            </label>
            <select
              id="project-ai-config-switcher"
              aria-label={t("projectAiConfig.switcher", {
                defaultValue: "当前配置项目",
              })}
              className="h-8 min-w-48 max-w-72 rounded-md border border-input bg-background px-2 text-sm text-foreground"
              value={project.id}
              onChange={(e) => onSelectProject(e.target.value)}
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <Button
              variant="outline"
              size="sm"
              className="rounded-lg"
              onClick={() => void refreshReadiness()}
              disabled={readinessLoading}
            >
              <RefreshCw
                className={`h-3.5 w-3.5 mr-1.5 ${readinessLoading ? "animate-spin" : ""}`}
              />
              {t("common.refresh", { defaultValue: "刷新" })}
            </Button>
          </div>
        </div>
      </div>

      <div className="px-6 py-5 space-y-6">
        {/*
         * 上排两块并列：左边是「一次性套一整套」（蓝图），右边是「逐项看差在哪」
         * （就绪度）—— 先决定用哪套基线，再看这套基线在本项目上还缺什么。
         * 抽屉时代它们只能上下堆着，看右边时看不见左边。
         */}
        <div className="grid gap-6 xl:grid-cols-2 items-start">
          <ProjectBlueprintPanel
            key={`blueprint-${project.id}`}
            projectId={project.id}
            onApplied={() => {
              void refreshReadiness();
              handleConfigChanged();
            }}
          />
          <AgentReadinessPanel
            data={readinessData}
            isLoading={readinessLoading}
            onRefresh={refreshReadiness}
            onScanEffective={scanEffective}
            onRepairDrift={handleRepairDrift}
            repairingCheckName={repairingCheckName}
            onOpenProjectAssets={(section) => setScrollSection(section ?? null)}
            onNavigate={onNavigate}
          />
        </div>

        {/*
         * 工作流编排**不**在这里再挂一份。`ProjectFlowOrchestratorPanel` 已经是
         * 「流程与方法论 → 工作流配置」那一页的主体，那里还带着预设推荐、变更
         * 执行方案、设计合约一整条链路。在这儿复制一份等于第三个挂载点，
         * 也就是审查报告 §3.1 反复清理过的那类重复 —— 改口径时永远漏掉一处。
         * 所以这里只留一个去处明确的入口。
         */}
        <button
          type="button"
          onClick={() => onNavigate?.("methodology")}
          className="w-full flex items-center gap-3 rounded-xl border border-border/60 bg-muted/20 px-4 py-3 text-left transition-colors hover:bg-muted/40"
        >
          <Workflow className="h-4 w-4 text-primary shrink-0" />
          <span className="min-w-0 flex-1">
            <span className="block text-sm font-medium text-foreground">
              {t("projectAiConfig.orchestrationLink", {
                defaultValue: "工作流编排",
              })}
            </span>
            <span className="block text-[11px] text-muted-foreground">
              {t("projectAiConfig.orchestrationHint", {
                defaultValue:
                  "阶段、预设与变更执行方案在「流程与方法论」里配置，会带着当前项目跳过去。",
              })}
            </span>
          </span>
        </button>

        <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
          <h3 className="text-sm font-semibold text-foreground mb-1">
            {t("kanban.detail.projectAssets", {
              defaultValue: "本项目启用的资产",
            })}
          </h3>
          {/*
           * 这句从抽屉搬过来时**不是**原样照抄：抽屉里的 defaultValue 写的是
           * 「与侧栏全局库双向同步」，而 `zh.json`（也就是用户真正看到的那句）
           * 写的是下面这句。两句意思还相反 —— 前者承诺双向，后者说清楚全局库
           * 在侧栏维护、项目关联只落本地库。搬运时按 zh.json 对齐，顺手清掉
           * 这条 `i18n:sync` 漂移；否则等于把一句已知说错的话抄进新页面。
           */}
          <p className="text-[11px] text-muted-foreground mb-4">
            {t("kanban.detail.projectAssetsHint", {
              defaultValue:
                "勾选后仅对当前项目生效；全局库在侧栏维护，项目关联仅存于本地数据库。",
            })}
          </p>
          <ProjectAssetPanel
            key={`assets-${project.id}`}
            projectId={project.id}
            scrollToSection={scrollSection}
            onConfigChanged={handleConfigChanged}
            onNavigateToGlobal={onNavigate}
          />
        </div>
      </div>
    </motion.div>
  );
}
