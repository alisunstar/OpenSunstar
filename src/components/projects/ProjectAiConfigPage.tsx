import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { FolderPlus, Sparkles, Workflow } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { AgentReadinessPanel } from "@/components/kanban/AgentReadinessPanel";
import { ProjectAssetPanel } from "@/components/projects/ProjectAssetPanel";
import { ProjectBlueprintPanel } from "@/components/projects/ProjectBlueprintPanel";
import { ProjectAssetHealthSummary } from "@/components/projects/ProjectAssetHealthSummary";
import { ProjectWikiPanel } from "@/components/projects/ProjectWikiPanel";
import { ProjectEnvironmentSnapshotPanel } from "@/components/projects/ProjectEnvironmentSnapshotPanel";
import { PageScopeBadge } from "@/components/shared/PageScopeBadge";
import { useAgentReadiness } from "@/hooks/useAIInsight";
import { repairAssetDrift } from "@/api/aiInsight";
import { showRepairAssetFeedback } from "@/lib/repairFeedback";
import {
  buildAiProviderSettingsIntent,
  setSettingsNavIntent,
} from "@/lib/settingsNavigation";
import type { PageView, ProjectAiConfigNavigationIntent } from "@/app/navigation";
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
  /** 打开当前项目的单一工作流配置入口。 */
  onOpenProjectWorkflow?: (projectId: string) => void;
  onAddProject?: () => void;
  /** 配置落盘后通知外部重扫（看板的就绪度批量结果会因此失效）。 */
  onConfigChanged?: () => void;
  targetApp?: AppId;
  /**
   * 深链落点（工作区重构 2026-07-30）：从「今日」告警卡或资产矩阵缺口格
   * 跳来时，不只是打开页面 —— 还要落到指定子 Tab，并把 `section` 对应的
   * 资产区滚动定位。与 `MethodologyPage` 的 intent 消费同一模式。
   */
  navigationIntent?: ProjectAiConfigNavigationIntent;
  onNavigationIntentConsumed?: () => void;
}

export function ProjectAiConfigPage({
  projects,
  selectedProjectId,
  onSelectProject,
  onNavigate,
  onOpenProjectWorkflow,
  onAddProject,
  onConfigChanged,
  targetApp = "claude",
  navigationIntent,
  onNavigationIntentConsumed,
}: ProjectAiConfigPageProps) {
  const { t } = useTranslation();
  const [scrollSection, setScrollSection] =
    useState<ProjectAssetSection | null>(null);
  const [activeTab, setActiveTab] = useState("assets");
  const [repairingCheckName, setRepairingCheckName] = useState<string | null>(
    null,
  );

  const project =
    projects.find((p) => p.id === selectedProjectId) ?? projects[0] ?? null;

  /*
   * 深链 intent 消费（照 `MethodologyPage` 的模式）：`key` 去重，消费一次即焚。
   *
   * 竞态：消费里 `onSelectProject` 会引发 `project?.id` 变化，而下面那个
   * 「切项目就重置 tab/section」的 effect 会把 intent 刚设好的落点抹掉。
   * 所以消费时把目标项目记进 `intentDrivenProjectId`，reset effect 见到
   * 这次切换是深链引起的就跳过一次 —— 只跳一次，用完即清。
   *
   * `section` 定位只在 assets 子 Tab 生效（`ProjectAssetPanel.scrollToSection`
   * 是现成的）；readiness 子 Tab 落在页签级 —— 修复入口本身就在那里。
   */
  const consumedIntentKey = useRef<number | null>(null);
  const intentDrivenProjectId = useRef<string | null>(null);
  useEffect(() => {
    if (
      !navigationIntent ||
      consumedIntentKey.current === navigationIntent.key
    ) {
      return;
    }
    consumedIntentKey.current = navigationIntent.key;
    if (
      navigationIntent.projectId &&
      navigationIntent.projectId !== selectedProjectId
    ) {
      intentDrivenProjectId.current = navigationIntent.projectId;
      onSelectProject(navigationIntent.projectId);
    }
    setActiveTab(navigationIntent.tab ?? "readiness");
    setScrollSection(
      (navigationIntent.tab ?? "readiness") === "assets"
        ? (navigationIntent.section ?? null)
        : null,
    );
    onNavigationIntentConsumed?.();
  }, [
    navigationIntent,
    selectedProjectId,
    onSelectProject,
    onNavigationIntentConsumed,
  ]);

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
    // 深链引起的那次项目切换不重置：落点已由 intent effect 设置（见上方注释）
    if (intentDrivenProjectId.current !== null) {
      if (intentDrivenProjectId.current === project?.id) {
        intentDrivenProjectId.current = null;
        return;
      }
      intentDrivenProjectId.current = null;
    }
    setScrollSection(null);
    setActiveTab("assets");
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

  const handleOpenAiProviderSettings = useCallback(() => {
    setSettingsNavIntent(buildAiProviderSettingsIntent());
    onNavigate?.("settings");
  }, [onNavigate]);

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
              {t("projectAiConfig.title", { defaultValue: "项目资产配置" })}
            </h2>
            <p className="text-sm text-muted-foreground mt-1">
              {t("projectAiConfig.subtitle", {
                defaultValue:
                  "为单个项目落地 AI 资产：套用蓝图、检查就绪度、勾选生效的资产。侧栏「Agent 配置」管的是全局库，这里管它们在本项目上的关联。",
              })}
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <PageScopeBadge scope="project" projectName={project.name} />
            <Button
              type="button"
              size="sm"
              variant="outline"
              className="h-8"
              onClick={() => onOpenProjectWorkflow?.(project.id)}
            >
              <Workflow className="mr-1.5 h-3.5 w-3.5" />
              {t("projectAiConfig.openWorkflow", {
                defaultValue: "配置项目工作流",
              })}
            </Button>
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
            <span className="rounded-md border border-border/60 bg-muted/30 px-2 py-1.5 text-xs text-muted-foreground">
              {t("projectAiConfig.targetCli", { defaultValue: "目标 CLI" })}：
              <span className="ml-1 font-medium text-foreground">
                {targetApp === "codex"
                  ? "Codex"
                  : targetApp === "gemini"
                    ? "Gemini"
                    : targetApp === "opencode"
                      ? "OpenCode"
                      : targetApp === "hermes"
                        ? "Hermes"
                        : "Claude"}
              </span>
            </span>
          </div>
        </div>
      </div>

      <Tabs
        value={activeTab}
        onValueChange={setActiveTab}
        className="px-6 py-5"
      >
        <TabsList className="grid w-full max-w-xl grid-cols-3">
          <TabsTrigger value="assets">
            {t("projectAiConfig.tabs.assets", { defaultValue: "资产关联" })}
          </TabsTrigger>
          <TabsTrigger value="readiness">
            {t("projectAiConfig.tabs.readiness", {
              defaultValue: "就绪与生效",
            })}
          </TabsTrigger>
          <TabsTrigger value="environment">
            {t("projectAiConfig.tabs.environment", {
              defaultValue: "项目环境 & Wiki",
            })}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="assets" className="mt-5 space-y-6">
          <ProjectBlueprintPanel
            key={`blueprint-${project.id}`}
            projectId={project.id}
            onApplied={() => {
              void refreshReadiness();
              handleConfigChanged();
            }}
          />
          <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
            <h3 className="text-sm font-semibold text-foreground mb-1">
              {t("kanban.detail.projectAssets", {
                defaultValue: "本项目启用的资产",
              })}
            </h3>
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
        </TabsContent>

        <TabsContent value="readiness" className="mt-5 space-y-6">
          <AgentReadinessPanel
            data={readinessData}
            isLoading={readinessLoading}
            onRefresh={refreshReadiness}
            onScanEffective={scanEffective}
            onRepairDrift={handleRepairDrift}
            repairingCheckName={repairingCheckName}
            onOpenProjectAssets={(section) => {
              setScrollSection(section ?? null);
              setActiveTab("assets");
            }}
            onNavigate={onNavigate}
          />
          <ProjectAssetHealthSummary projectId={project.id} />
        </TabsContent>

        <TabsContent value="environment" className="mt-5 space-y-6">
          <ProjectWikiPanel
            projectId={project.id}
            onConfigChanged={handleConfigChanged}
            onOpenAiProviderSettings={handleOpenAiProviderSettings}
          />
          <ProjectEnvironmentSnapshotPanel
            projectId={project.id}
            onApplied={handleConfigChanged}
          />
        </TabsContent>
      </Tabs>
    </motion.div>
  );
}
