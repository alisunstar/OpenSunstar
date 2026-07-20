import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import {
  AlertCircle,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  ChefHat,
  ExternalLink,
  FileStack,
  FolderOpen,
  HelpCircle,
  Loader2,
  MinusCircle,
  Palette,
  RefreshCw,
  Workflow,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ProjectFlowOrchestratorPanel } from "@/components/projects/ProjectFlowOrchestratorPanel";
import { ProjectRecipeComposer } from "@/components/projects/ProjectRecipeComposer";
import ProjectDesignContractPanel from "@/components/projects/ProjectDesignContractPanel";
import { ProjectRulesContextPanel } from "@/components/projects/ProjectRulesContextPanel";
import {
  sddApi,
  type SddDescriptorSummary,
  type SddDetectionResult,
} from "@/lib/api/sdd";
import type { Project } from "@/types/project";
import { cn } from "@/lib/utils";

// ─── Constants ───────────────────────────────────────────────────────────────

const FRAMEWORK_COLORS: Record<string, string> = {
  "bmad-method": "from-blue-500/20 to-blue-600/10 border-blue-500/30",
  "task-master": "from-amber-500/20 to-amber-600/10 border-amber-500/30",
  superpowers: "from-purple-500/20 to-purple-600/10 border-purple-500/30",
  gstack: "from-emerald-500/20 to-emerald-600/10 border-emerald-500/30",
  openspec: "from-cyan-500/20 to-cyan-600/10 border-cyan-500/30",
  "spec-kit": "from-rose-500/20 to-rose-600/10 border-rose-500/30",
  "flow-kit": "from-teal-500/20 to-teal-600/10 border-teal-500/30",
};

/** Map install_type code to human-readable label for framework cards. */
function installTypeLabel(type: string): string {
  switch (type) {
    case "npm": return "npm";
    case "uvx": return "uvx (Python)";
    case "plugin": return "IDE 插件";
    case "file_copy": return "文件复制";
    default: return type;
  }
}

// ─── Types ───────────────────────────────────────────────────────────────────

/** projectId → descriptorId → detection result */
type AllDetections = Record<string, Record<string, SddDetectionResult>>;

function groupDetections(
  allResults: Record<string, SddDetectionResult[]>,
): AllDetections {
  const grouped: AllDetections = {};
  for (const [projectId, results] of Object.entries(allResults)) {
    const map: Record<string, SddDetectionResult> = {};
    for (const r of results) {
      map[r.descriptorId] = r;
    }
    grouped[projectId] = map;
  }
  return grouped;
}

function isDetectionPositive(result: SddDetectionResult): boolean {
  return (
    result.detected &&
    (result.confidence === "verified" || result.confidence === "inferred")
  );
}

function presetLabelKey(presetId: string): string {
  switch (presetId) {
    case "full":
      return "methodology.presetFull";
    case "standard":
      return "methodology.presetStandard";
    case "mvp":
      return "methodology.presetMvp";
    case "review-only":
      return "methodology.presetReviewOnly";
    default:
      return presetId;
  }
}

interface MethodologyPageProps {
  projects: Project[];
}

// ─── Sub-components ──────────────────────────────────────────────────────────

/** Aggregate detection status across all scanned projects for one framework. */
function AggregateDetectionBadge({
  perProject,
  scannedProjectCount,
  hasScannedAny,
}: {
  perProject: Array<{ project: Project; result: SddDetectionResult }>;
  scannedProjectCount: number;
  hasScannedAny: boolean;
}) {
  const { t } = useTranslation();

  if (!hasScannedAny) {
    return (
      <span className="text-[10px] px-2 py-0.5 rounded-full bg-muted text-muted-foreground border border-border/50">
        {t("methodology.notScanned", { defaultValue: "未扫描" })}
      </span>
    );
  }

  const verifiedCount = perProject.filter(
    (p) => p.result.confidence === "verified",
  ).length;
  const inferredCount = perProject.filter(
    (p) => p.result.confidence === "inferred",
  ).length;

  if (verifiedCount > 0) {
    return (
      <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20 flex items-center gap-1">
        <CheckCircle2 className="w-3 h-3" />
        {t("methodology.detectedInProjects", {
          defaultValue: "{{detected}}/{{total}} 项目已检测到",
          detected: verifiedCount,
          total: scannedProjectCount,
        })}
      </span>
    );
  }

  if (inferredCount > 0) {
    return (
      <span className="text-[10px] px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-700 dark:text-amber-400 border border-amber-500/20 flex items-center gap-1">
        <HelpCircle className="w-3 h-3" />
        {t("methodology.inferredInProjects", {
          defaultValue: "{{inferred}}/{{total}} 项目可能使用",
          inferred: inferredCount,
          total: scannedProjectCount,
        })}
      </span>
    );
  }

  return (
    <span className="text-[10px] px-2 py-0.5 rounded-full bg-muted text-muted-foreground border border-border/50 flex items-center gap-1">
      <XCircle className="w-3 h-3" />
      {t("methodology.absentInAll", {
        defaultValue: "{{total}} 项目均未检测到",
        total: scannedProjectCount,
      })}
    </span>
  );
}

function MethodologyCard({
  descriptor,
  perProjectDetections,
  scannedProjectCount,
  hasScannedAny,
  recommendedPreset,
  onGoToOrchestration,
}: {
  descriptor: SddDescriptorSummary;
  perProjectDetections: Array<{
    project: Project;
    result: SddDetectionResult;
  }>;
  scannedProjectCount: number;
  hasScannedAny: boolean;
  recommendedPreset?: string | null;
  onGoToOrchestration?: (projectId: string, presetId?: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const colorClass =
    FRAMEWORK_COLORS[descriptor.id] ??
    "from-gray-500/20 to-gray-600/10 border-gray-500/30";
  const description =
    i18n.language === "zh" || i18n.language === "zh-TW"
      ? descriptor.descriptionZh ?? descriptor.descriptionEn
      : descriptor.descriptionEn ?? descriptor.descriptionZh;

  // Collect signal matches across projects (dedup by signal text)
  const allSignals = useMemo(() => {
    const seen = new Set<string>();
    const signals: Array<{ signal: string; projectName: string }> = [];
    for (const { project, result } of perProjectDetections) {
      for (const sm of result.signalMatches) {
        const key = `${sm.signal}@${project.name}`;
        if (!seen.has(key)) {
          seen.add(key);
          signals.push({ signal: sm.signal, projectName: project.name });
        }
      }
    }
    return signals.slice(0, 6); // cap display
  }, [perProjectDetections]);

  const primaryDetectedProject = useMemo(
    () =>
      perProjectDetections.find((entry) =>
        isDetectionPositive(entry.result),
      )?.project,
    [perProjectDetections],
  );

  const showOrchestrationCta =
    hasScannedAny && primaryDetectedProject && onGoToOrchestration;

  return (
    <div
      className={cn(
        "rounded-xl border bg-gradient-to-br p-4 space-y-3 transition-all hover:shadow-md",
        colorClass,
      )}
    >
      <div className="flex items-start justify-between">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {descriptor.name}
          </h3>
          <p className="text-[10px] text-muted-foreground mt-0.5 font-mono">
            {descriptor.version}
          </p>
        </div>
        <AggregateDetectionBadge
          perProject={perProjectDetections}
          scannedProjectCount={scannedProjectCount}
          hasScannedAny={hasScannedAny}
        />
      </div>

      <p className="text-xs text-muted-foreground leading-relaxed line-clamp-2">
        {description}
      </p>

      {/* Signal matches detail — shows which project matched which signal */}
      {allSignals.length > 0 && (
        <div className="space-y-1">
          {allSignals.map((sm, i) => (
            <div
              key={i}
              className="text-[10px] text-muted-foreground flex items-center gap-1"
            >
              <CheckCircle2 className="w-3 h-3 text-emerald-500 shrink-0" />
              <span className="truncate">{sm.signal}</span>
              <span className="text-muted-foreground/60 shrink-0">
                · {sm.projectName}
              </span>
            </div>
          ))}
        </div>
      )}

      {showOrchestrationCta && (
        <div className="space-y-2 pt-1 border-t border-border/40">
          {recommendedPreset && (
            <p className="text-[10px] text-muted-foreground">
              {t("methodology.recommendedPreset", {
                defaultValue: "推荐档位：{{preset}}",
                preset: t(presetLabelKey(recommendedPreset), {
                  defaultValue: recommendedPreset,
                }),
              })}
            </p>
          )}
          <Button
            size="sm"
            variant="secondary"
            className="h-7 w-full text-xs"
            onClick={() =>
              onGoToOrchestration?.(
                primaryDetectedProject.id,
                recommendedPreset ?? undefined,
              )
            }
          >
            {t("methodology.goToOrchestration", {
              defaultValue: "去工作流配置",
            })}
            <ArrowRight className="w-3 h-3 ml-1" />
          </Button>
        </div>
      )}

      <div className="flex items-center justify-between pt-1">
        <span className="text-[10px] text-muted-foreground">
          {installTypeLabel(descriptor.installType)} · {descriptor.phaseModel}
        </span>
        {descriptor.repoUrl && (
          <a
            href={descriptor.repoUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-[10px] text-primary hover:underline flex items-center gap-0.5"
          >
            GitHub <ExternalLink className="w-3 h-3" />
          </a>
        )}
      </div>
    </div>
  );
}

/** Compact cell badge for the detection matrix. */
function MatrixCell({ result }: { result?: SddDetectionResult }) {
  if (!result) {
    return <MinusCircle className="w-3.5 h-3.5 text-muted-foreground/30 mx-auto" />;
  }
  if (result.confidence === "verified") {
    return (
      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500 mx-auto" />
    );
  }
  if (result.confidence === "inferred") {
    return <HelpCircle className="w-3.5 h-3.5 text-amber-500 mx-auto" />;
  }
  return <XCircle className="w-3.5 h-3.5 text-muted-foreground/40 mx-auto" />;
}

/** Project × Framework detection matrix table. */
function DetectionMatrix({
  descriptors,
  projects,
  allDetections,
}: {
  descriptors: SddDescriptorSummary[];
  projects: Project[];
  allDetections: AllDetections;
}) {
  const { t } = useTranslation();

  if (projects.length === 0) {
    return null;
  }

  const scannedProjectIds = Object.keys(allDetections);
  if (scannedProjectIds.length === 0) {
    return null;
  }

  return (
    <div className="rounded-xl border border-border/60 bg-muted/20 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Workflow className="w-4 h-4 text-primary" />
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {t("methodology.detectionMatrix", { defaultValue: "检测矩阵" })}
          </h3>
          <p className="text-[11px] text-muted-foreground mt-0.5">
            {t("methodology.detectionMatrixHint", {
              defaultValue: "项目 × 框架检测结果（✓ 已检测到 / ? 可能使用 / ✗ 未检测到 / — 未扫描）",
            })}
          </p>
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-xs border-collapse">
          <thead>
            <tr className="border-b border-border/60">
              <th className="text-left py-2 px-2 font-medium text-muted-foreground sticky left-0 bg-muted/20">
                {t("methodology.matrixFramework", {
                  defaultValue: "框架 \\ 项目",
                })}
              </th>
              {projects.map((p) => (
                <th
                  key={p.id}
                  className="text-center py-2 px-2 font-medium text-muted-foreground min-w-[80px] max-w-[120px]"
                  title={p.name}
                >
                  <span className="block truncate">{p.name}</span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {descriptors.map((d) => (
              <tr
                key={d.id}
                className="border-b border-border/30 hover:bg-muted/30"
              >
                <td className="py-2 px-2 font-medium text-foreground sticky left-0 bg-background/60">
                  <span className="text-[11px]">{d.name}</span>
                  <span className="text-[9px] text-muted-foreground ml-1 font-mono">
                    {d.version}
                  </span>
                </td>
                {projects.map((p) => (
                  <td key={p.id} className="py-2 px-2 text-center">
                    <MatrixCell result={allDetections[p.id]?.[d.id]} />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ─── Main Page ───────────────────────────────────────────────────────────────

export function MethodologyPage({ projects }: MethodologyPageProps) {
  const { t } = useTranslation();
  const [descriptors, setDescriptors] = useState<SddDescriptorSummary[]>([]);
  const [allDetections, setAllDetections] = useState<AllDetections>({});
  const [loading, setLoading] = useState(true);
  const [loadingSaved, setLoadingSaved] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [activeTab, setActiveTab] = useState("rulesContext");

  // Tab 2 orchestration: selected project for flow config
  const [orchestrationProjectId, setOrchestrationProjectId] = useState("");
  const [orchestrationInitialPreset, setOrchestrationInitialPreset] =
    useState<string | undefined>();
  const [presetRecommendations, setPresetRecommendations] = useState<
    Record<string, string | null>
  >({});

  const loadDescriptors = useCallback(async () => {
    setLoading(true);
    try {
      const list = await sddApi.listDescriptors();
      setDescriptors(list);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const applyDetectionResults = useCallback(
    (allResults: Record<string, SddDetectionResult[]>) => {
      setAllDetections(groupDetections(allResults));
    },
    [],
  );

  const loadSavedDetections = useCallback(async () => {
    if (projects.length === 0) {
      setAllDetections({});
      return;
    }
    setLoadingSaved(true);
    try {
      const saved = await sddApi.getAllSavedDetections();
      applyDetectionResults(saved);
    } catch (e) {
      console.error("[MethodologyPage] load saved detections failed", e);
    } finally {
      setLoadingSaved(false);
    }
  }, [applyDetectionResults, projects.length]);

  useEffect(() => {
    void loadDescriptors();
  }, [loadDescriptors]);

  useEffect(() => {
    void loadSavedDetections();
  }, [loadSavedDetections]);

  const handleScanAll = async () => {
    setScanning(true);
    try {
      const allResults = await sddApi.detectAllProjects();
      applyDetectionResults(allResults);

      const scannedCount = Object.keys(allResults).length;
      toast.success(
        t("methodology.scanCompleteWithCount", {
          defaultValue: "已扫描 {{count}} 个项目",
          count: scannedCount,
        }),
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      setScanning(false);
    }
  };

  const scannedProjectCount = useMemo(
    () => Object.keys(allDetections).length,
    [allDetections],
  );
  const hasScannedAny = scannedProjectCount > 0;

  // 统计至少在一个项目中被检测到（verified / inferred）的框架数
  const detectedFrameworkCount = useMemo(() => {
    let count = 0;
    for (const d of descriptors) {
      const detectedInAny = Object.values(allDetections).some((projMap) => {
        const result = projMap[d.id];
        return result ? isDetectionPositive(result) : false;
      });
      if (detectedInAny) count++;
    }
    return count;
  }, [descriptors, allDetections]);

  const getPerProjectDetections = useCallback(
    (descriptorId: string) => {
      const result: Array<{ project: Project; result: SddDetectionResult }> =
        [];
      for (const project of projects) {
        const projMap = allDetections[project.id];
        if (projMap && projMap[descriptorId]) {
          result.push({
            project,
            result: projMap[descriptorId],
          });
        }
      }
      return result;
    },
    [projects, allDetections],
  );

  const detectedDescriptors = useMemo(
    () =>
      descriptors.filter((d) => {
        const perProject = getPerProjectDetections(d.id);
        return perProject.some((entry) => isDetectionPositive(entry.result));
      }),
    [descriptors, getPerProjectDetections],
  );

  const catalogDescriptors = useMemo(() => {
    if (!hasScannedAny || detectedFrameworkCount === 0) {
      return descriptors;
    }
    const detectedIds = new Set(detectedDescriptors.map((d) => d.id));
    return descriptors.filter((d) => !detectedIds.has(d.id));
  }, [
    descriptors,
    detectedDescriptors,
    detectedFrameworkCount,
    hasScannedAny,
  ]);

  useEffect(() => {
    if (!hasScannedAny || detectedDescriptors.length === 0) {
      setPresetRecommendations({});
      return;
    }

    let cancelled = false;
    void (async () => {
      const next: Record<string, string | null> = {};
      for (const descriptor of detectedDescriptors) {
        const primaryProject = getPerProjectDetections(descriptor.id).find(
          (entry) => isDetectionPositive(entry.result),
        )?.project;
        if (!primaryProject) continue;
        try {
          next[descriptor.id] = await sddApi.recommendPreset(
            primaryProject.id,
          );
        } catch {
          next[descriptor.id] = null;
        }
      }
      if (!cancelled) {
        setPresetRecommendations(next);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [detectedDescriptors, getPerProjectDetections, hasScannedAny]);

  const handleGoToOrchestration = useCallback(
    (projectId: string, presetId?: string) => {
      setOrchestrationProjectId(projectId);
      setOrchestrationInitialPreset(presetId);
      setActiveTab("orchestration");
    },
    [],
  );

  const handleGoToRecipe = useCallback(() => {
    if (projects.length > 0 && !orchestrationProjectId) {
      setOrchestrationProjectId(projects[0].id);
    }
    setActiveTab("recipe");
  }, [orchestrationProjectId, projects]);

  const renderFrameworkCards = (items: SddDescriptorSummary[]) => (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {items.map((d) => (
        <MethodologyCard
          key={d.id}
          descriptor={d}
          perProjectDetections={getPerProjectDetections(d.id)}
          scannedProjectCount={scannedProjectCount}
          hasScannedAny={hasScannedAny}
          recommendedPreset={presetRecommendations[d.id]}
          onGoToOrchestration={handleGoToOrchestration}
        />
      ))}
    </div>
  );

  if (loading || loadingSaved) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-6 h-6 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <motion.div
      className="flex-1 flex flex-col min-h-0"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
    >
      <div className="shrink-0 px-6 pt-6 pb-2">
        <div className="flex items-center justify-between">
          <div className="space-y-3">
            <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
              <BookOpen className="w-5 h-5 text-primary" />
              {t("methodology.title", { defaultValue: "工作流与治理" })}
            </h2>
            <p className="text-sm text-muted-foreground mt-1">
              {t("methodology.subtitle", {
                defaultValue:
                  "为项目建立可执行流程、变更执行方案与治理检查；方法论识别作为可选诊断。",
              })}
            </p>
            {projects.length > 0 && (
              <label className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">当前配置项目</span>
                <select
                  aria-label="当前配置项目"
                  className="h-8 min-w-48 rounded-md border border-input bg-background px-2 text-sm text-foreground"
                  value={orchestrationProjectId}
                  onChange={(e) => setOrchestrationProjectId(e.target.value)}
                >
                  <option value="">请选择项目</option>
                  {projects.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>
          <Button
            size="sm"
            variant="outline"
            disabled={scanning || projects.length === 0}
            onClick={handleScanAll}
            title={
              projects.length === 0
                ? t("methodology.scanAllNoProjects", {
                    defaultValue: "请先添加项目再扫描",
                  })
                : undefined
            }
          >
            {scanning ? (
              <Loader2 className="w-4 h-4 animate-spin mr-1.5" />
            ) : (
              <RefreshCw className="w-4 h-4 mr-1.5" />
            )}
            {t("methodology.scanAll", { defaultValue: "只读发现项目" })}
          </Button>
        </div>
      </div>

      <div className="flex-1 min-h-0 px-6 pb-6">
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="h-full flex flex-col"
        >
          <TabsList className="shrink-0 mb-4">
            <TabsTrigger value="rulesContext">
              <FileStack className="w-3.5 h-3.5 mr-1.5" />
              {t("methodology.tabRulesContext", { defaultValue: "规则与上下文" })}
            </TabsTrigger>
            <TabsTrigger value="orchestration">
              <Workflow className="w-3.5 h-3.5 mr-1.5" />
              {t("methodology.tabOrchestration", { defaultValue: "工作流配置" })}
            </TabsTrigger>
            <TabsTrigger value="recipe">
              <ChefHat className="w-3.5 h-3.5 mr-1.5" />
              {t("methodology.tabRecipe", { defaultValue: "变更执行方案" })}
              <span className="ml-1 text-[9px] px-1 py-0.5 rounded bg-amber-500/20 text-amber-600 dark:text-amber-400 font-medium leading-none">
                Beta
              </span>
            </TabsTrigger>
            <TabsTrigger value="market">
              {t("methodology.tabMarket", { defaultValue: "现状发现" })}
              {detectedFrameworkCount > 0 && (
                <span className="ml-1.5 text-[10px] px-1.5 py-0.5 rounded-full bg-emerald-500/20 text-emerald-600 dark:text-emerald-400">
                  {detectedFrameworkCount}
                </span>
              )}
            </TabsTrigger>
            <TabsTrigger value="designContract">
              <Palette className="w-3.5 h-3.5 mr-1.5" />
              {t("methodology.tabDesignContract", { defaultValue: "设计合约" })}
              <span className="ml-1 text-[9px] px-1 py-0.5 rounded bg-amber-500/20 text-amber-600 dark:text-amber-400 font-medium leading-none">
                Beta
              </span>
            </TabsTrigger>
          </TabsList>

          {/* Tab 1: Framework Market + Detection Matrix */}
          <TabsContent
            value="market"
            className="flex-1 min-h-0 overflow-y-auto space-y-4"
          >
            {!hasScannedAny && projects.length > 0 && (
              <div className="rounded-lg border border-border/60 bg-muted/20 p-4 flex flex-col sm:flex-row sm:items-center gap-4">
                <div className="flex gap-3 flex-1">
                  <BookOpen className="w-5 h-5 text-primary shrink-0 mt-0.5" />
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-foreground">
                      {t("methodology.scanPromptTitle", {
                        defaultValue: "尚未发现项目中的方法论痕迹",
                      })}
                    </p>
                    <p className="text-xs text-muted-foreground leading-relaxed">
                      {t("methodology.scanPromptBody", {
                        defaultValue:
                          "点击「只读发现项目」进行无写入探测，只用于识别已有实践与推荐流程档位；也可以直接进入工作流配置 / 变更执行方案。",
                      })}
                    </p>
                  </div>
                </div>
                <div className="flex flex-wrap gap-2 shrink-0">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={scanning}
                    onClick={handleScanAll}
                  >
                    {scanning ? (
                      <Loader2 className="w-4 h-4 animate-spin mr-1.5" />
                    ) : (
                      <RefreshCw className="w-4 h-4 mr-1.5" />
                    )}
                    {t("methodology.scanAll", { defaultValue: "只读发现项目" })}
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handleGoToOrchestration(projects[0]?.id ?? "")}
                  >
                    {t("methodology.goToOrchestration", {
                      defaultValue: "去工作流配置",
                    })}
                  </Button>
                  <Button size="sm" variant="ghost" onClick={handleGoToRecipe}>
                    {t("methodology.goToRecipe", {
                      defaultValue: "变更执行方案",
                    })}
                  </Button>
                </div>
              </div>
            )}

            {hasScannedAny &&
              detectedFrameworkCount === 0 &&
              projects.length > 0 && (
              <div className="rounded-lg border border-primary/20 bg-primary/5 p-4 flex flex-col sm:flex-row sm:items-center gap-4">
                <div className="flex gap-3 flex-1">
                  <Workflow className="w-5 h-5 text-primary shrink-0 mt-0.5" />
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-foreground">
                      {t("methodology.directOrchestrationTitle", {
                        defaultValue: "不依赖框架也可开始编排",
                      })}
                    </p>
                    <p className="text-xs text-muted-foreground leading-relaxed">
                      {t("methodology.directOrchestrationHint", {
                        defaultValue:
                          "可独立保存项目工作规则，也可生成变更执行材料；具体文件名放在高级详情中查看。",
                      })}
                    </p>
                  </div>
                </div>
                <div className="flex flex-wrap gap-2 shrink-0">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() =>
                      handleGoToOrchestration(
                        orchestrationProjectId || projects[0]?.id || "",
                      )
                    }
                  >
                    {t("methodology.goToOrchestration", {
                      defaultValue: "去工作流配置",
                    })}
                  </Button>
                  <Button size="sm" variant="ghost" onClick={handleGoToRecipe}>
                    {t("methodology.goToRecipe", {
                      defaultValue: "变更执行方案",
                    })}
                  </Button>
                </div>
              </div>
            )}

            {hasScannedAny && detectedFrameworkCount === 0 && (
              <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-4 flex gap-3">
                <AlertCircle className="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
                <div className="space-y-1">
                  <p className="text-sm font-medium text-foreground">
                    {t("methodology.noneDetectedTitle", {
                      defaultValue: "未发现可识别的方法论痕迹",
                    })}
                  </p>
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    {t("methodology.noneDetectedGuidance", {
                      defaultValue:
                        "这不阻断编排。可从工作流配置直接选择 review-only / mvp / standard，也可后续再引入 flow-kit 或 spec-kit。",
                    })}
                  </p>
                </div>
              </div>
            )}

            {detectedDescriptors.length > 0 && (
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 text-emerald-500" />
                  <h3 className="text-sm font-semibold text-foreground">
                    {t("methodology.detectedFrameworks", {
                      defaultValue: "已发现的实践痕迹",
                    })}
                  </h3>
                </div>
                {renderFrameworkCards(detectedDescriptors)}
              </div>
            )}

            {catalogDescriptors.length > 0 && (
              <Accordion
                type="single"
                collapsible
                defaultValue={
                  hasScannedAny && detectedFrameworkCount > 0
                    ? undefined
                    : "catalog"
                }
                className="rounded-xl border border-border/60 bg-muted/10 px-4"
              >
                <AccordionItem value="catalog" className="border-none">
                  <AccordionTrigger className="hover:no-underline py-4">
                    <div className="text-left">
                      <p className="text-sm font-semibold text-foreground">
                        {t("methodology.recommendedFrameworks", {
                          defaultValue: "方法论参考目录",
                        })}
                      </p>
                      <p className="text-xs text-muted-foreground font-normal mt-0.5">
                        {t("methodology.recommendedFrameworksHint", {
                          defaultValue:
                            "仅作为了解与选型参考；当前页面不会安装框架，也不会修改项目文件",
                        })}
                      </p>
                    </div>
                  </AccordionTrigger>
                  <AccordionContent className="pb-4">
                    {renderFrameworkCards(catalogDescriptors)}
                  </AccordionContent>
                </AccordionItem>
              </Accordion>
            )}

            {descriptors.length === 0 && (
              <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
                <BookOpen className="w-10 h-10 mb-3 opacity-40" />
                <p className="text-sm">
                  {t("methodology.noDescriptors", {
                    defaultValue: "框架目录为空。请确认数据库已初始化。",
                  })}
                </p>
              </div>
            )}

            <DetectionMatrix
              descriptors={descriptors}
              projects={projects}
              allDetections={allDetections}
            />
          </TabsContent>

          {/* Tab 1: 工作流配置 */}
          <TabsContent
            value="orchestration"
            className="flex-1 min-h-0 overflow-y-auto"
          >
            <div className="space-y-4">
              {/* Project selector */}
              <div className="hidden rounded-lg border border-border/60 bg-muted/20 p-4 space-y-3">
                <div className="flex items-center gap-2">
                  <FolderOpen className="w-4 h-4 text-primary" />
                  <p className="text-sm font-medium text-foreground">
                    {t("methodology.orchestrationEntry", {
                      defaultValue: "工作流配置",
                    })}
                  </p>
                </div>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  {t("methodology.orchestrationHint", {
                    defaultValue:
                      "独立维度：选择一个项目，保存团队工作规则并生成自动检查配置。无需先完成方法论框架扫描。",
                  })}
                </p>

                {projects.length === 0 ? (
                  <div className="rounded-lg border border-dashed border-border/60 p-6 flex flex-col items-center justify-center text-center">
                    <FolderOpen className="w-8 h-8 text-muted-foreground/40 mb-2" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.orchestrationNoProjects", {
                        defaultValue:
                          "暂无项目。请先在工作区添加项目，再进行流程编排配置。",
                      })}
                    </p>
                  </div>
                ) : (
                  <label className="block space-y-1">
                    <span className="text-[11px] text-muted-foreground">
                      {t("methodology.orchestrationSelectProject", {
                        defaultValue: "选择项目",
                      })}
                    </span>
                    <select
                      className="w-full h-9 rounded-md border border-input bg-background px-2 text-sm"
                      value={orchestrationProjectId}
                      onChange={(e) => setOrchestrationProjectId(e.target.value)}
                    >
                      <option value="">
                        {t("methodology.orchestrationChooseProject", {
                          defaultValue: "— 请选择项目 —",
                        })}
                      </option>
                      {projects.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>

              {/* Embedded FlowOrchestrator panel when a project is selected */}
              {projects.length > 0 && orchestrationProjectId ? (
                <ProjectFlowOrchestratorPanel
                  key={`${orchestrationProjectId}-${orchestrationInitialPreset ?? "default"}`}
                  projectId={orchestrationProjectId}
                  initialPresetId={orchestrationInitialPreset}
                />
              ) : (
                projects.length > 0 && (
                  <div className="rounded-lg border border-dashed border-border/60 p-8 flex flex-col items-center justify-center text-center">
                    <Workflow className="w-10 h-10 text-muted-foreground/40 mb-3" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.orchestrationPickProjectHint", {
                        defaultValue:
                          "请在上方下拉框选择一个项目，开始配置项目工作规则。",
                      })}
                    </p>
                  </div>
                )
              )}
            </div>
          </TabsContent>

          {/* Tab 2: 变更执行方案 (Beta) */}
          <TabsContent
            value="recipe"
            className="flex-1 min-h-0 overflow-y-auto"
          >
            <div className="space-y-4">
              {/* Project selector (shared with orchestration tab) */}
              <div className="hidden rounded-lg border border-border/60 bg-muted/20 p-4 space-y-3">
                <div className="flex items-center gap-2">
                  <FolderOpen className="w-4 h-4 text-primary" />
                  <p className="text-sm font-medium text-foreground">
                    {t("methodology.recipeEntry", {
                      defaultValue: "变更执行方案",
                    })}
                  </p>
                </div>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  {t("methodology.recipeHint", {
                    defaultValue:
                      "独立维度：用可视化步骤图生成变更执行材料。可独立使用，无需先配置其他维度。",
                  })}
                </p>

                {projects.length === 0 ? (
                  <div className="rounded-lg border border-dashed border-border/60 p-6 flex flex-col items-center justify-center text-center">
                    <FolderOpen className="w-8 h-8 text-muted-foreground/40 mb-2" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.recipeNoProjects", {
                        defaultValue:
                          "暂无项目。请先在工作区添加项目。",
                      })}
                    </p>
                  </div>
                ) : (
                  <label className="block space-y-1">
                    <span className="text-[11px] text-muted-foreground">
                      {t("methodology.recipeSelectProject", {
                        defaultValue: "选择项目",
                      })}
                    </span>
                    <select
                      className="w-full h-9 rounded-md border border-input bg-background px-2 text-sm"
                      value={orchestrationProjectId}
                      onChange={(e) => setOrchestrationProjectId(e.target.value)}
                    >
                      <option value="">
                        {t("methodology.recipePickPlaceholder", {
                          defaultValue: "— 选择项目 —",
                        })}
                      </option>
                      {projects.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>

              {/* Change execution plan editor when project selected */}
              {projects.length > 0 && orchestrationProjectId ? (
                <ProjectRecipeComposer
                  key={`recipe-${orchestrationProjectId}`}
                  projectId={orchestrationProjectId}
                />
              ) : (
                projects.length > 0 && (
                  <div className="rounded-lg border border-dashed border-border/60 p-8 flex flex-col items-center justify-center text-center">
                    <ChefHat className="w-10 h-10 text-muted-foreground/40 mb-3" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.recipePickProjectHint", {
                        defaultValue:
                          "请在上方下拉框选择一个项目，开始生成变更执行材料。",
                      })}
                    </p>
                  </div>
                )
              )}
            </div>
          </TabsContent>

          {/* Tab 4: 设计合约 (Design Contract — Beta) */}
          <TabsContent
            value="designContract"
            className="flex-1 min-h-0 overflow-y-auto"
          >
            <div className="space-y-4">
              {/* Project selector (shared with orchestration/recipe tabs) */}
              <div className="hidden rounded-lg border border-border/60 bg-muted/20 p-4 space-y-3">
                <div className="flex items-center gap-2">
                  <Palette className="w-4 h-4 text-primary" />
                  <p className="text-sm font-medium text-foreground">
                    {t("methodology.designContractEntry", {
                      defaultValue: "设计合约配置器",
                    })}
                  </p>
                </div>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  {t("methodology.designContractHint", {
                    defaultValue:
                      "独立维度：选择品牌模板或自定义配色，生成设计说明与设计令牌。任何有前端/UI 的项目均可独立使用。",
                  })}
                </p>

                {projects.length === 0 ? (
                  <div className="rounded-lg border border-dashed border-border/60 p-6 flex flex-col items-center justify-center text-center">
                    <FolderOpen className="w-8 h-8 text-muted-foreground/40 mb-2" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.designContractNoProjects", {
                        defaultValue:
                          "暂无项目。请先在工作区添加项目。",
                      })}
                    </p>
                  </div>
                ) : (
                  <label className="block space-y-1">
                    <span className="text-[11px] text-muted-foreground">
                      {t("methodology.designContractSelectProject", {
                        defaultValue: "选择项目",
                      })}
                    </span>
                    <select
                      className="w-full h-9 rounded-md border border-input bg-background px-2 text-sm"
                      value={orchestrationProjectId}
                      onChange={(e) => setOrchestrationProjectId(e.target.value)}
                    >
                      <option value="">
                        {t("methodology.designContractPickPlaceholder", {
                          defaultValue: "— 选择项目 —",
                        })}
                      </option>
                      {projects.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>

              {/* Design Contract panel when project selected */}
              {projects.length > 0 && orchestrationProjectId ? (
                <ProjectDesignContractPanel
                  key={`design-${orchestrationProjectId}`}
                  projectId={orchestrationProjectId}
                />
              ) : (
                projects.length > 0 && (
                  <div className="rounded-lg border border-dashed border-border/60 p-8 flex flex-col items-center justify-center text-center">
                    <Palette className="w-10 h-10 text-muted-foreground/40 mb-3" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.designContractPickProjectHint", {
                        defaultValue:
                          "请在上方下拉框选择一个项目，开始配置设计合约。",
                      })}
                    </p>
                  </div>
                )
              )}
            </div>
          </TabsContent>

          {/* Tab 5: 规则与上下文 (Rules & Context) */}
          <TabsContent
            value="rulesContext"
            className="flex-1 min-h-0 overflow-y-auto"
          >
            <div className="space-y-4">
              {projects.length > 0 && orchestrationProjectId ? (
                <ProjectRulesContextPanel
                  key={`rules-${orchestrationProjectId}`}
                  projectId={orchestrationProjectId}
                />
              ) : (
                projects.length > 0 && (
                  <div className="rounded-lg border border-dashed border-border/60 p-8 flex flex-col items-center justify-center text-center">
                    <FileStack className="w-10 h-10 text-muted-foreground/40 mb-3" />
                    <p className="text-sm text-muted-foreground">
                      {t("methodology.rulesContextPickProjectHint", {
                        defaultValue:
                          "请在上方下拉框选择一个项目，查看关联的规则片段与上下文文件。",
                      })}
                    </p>
                  </div>
                )
              )}
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </motion.div>
  );
}
