import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  FileOutput,
  GitBranch,
  History,
  Loader2,
  ShieldCheck,
  Workflow,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  flowOrchestratorApi,
  type FlowWritePlan,
  type FlowConfig,
  type OrchestrationLogEntry,
  type SpecsChangeIndex,
  type SpecsWorkflowIndex,
  type StageGateResult,
  type WorkflowModule,
  type WorkflowPreset,
  type WorkflowPresetSummary,
} from "@/lib/api/flowOrchestrator";
import { InstallConfirmModal } from "@/components/shared/InstallConfirmModal";
import { validateChangeId } from "@/lib/changeId";
import { cn } from "@/lib/utils";

const PROJECT_TYPES = ["backend", "frontend", "cli"] as const;

interface ProjectFlowOrchestratorPanelProps {
  projectId: string;
  initialPresetId?: string;
}

function CompletenessBar({ value }: { value: number }) {
  const tone =
    value >= 80 ? "bg-emerald-500" : value >= 40 ? "bg-amber-500" : "bg-red-500";
  return (
    <div className="h-1.5 w-full rounded-full bg-muted overflow-hidden">
      <div className={cn("h-full rounded-full transition-all", tone)} style={{ width: `${value}%` }} />
    </div>
  );
}

function ChangeRow({ change }: { change: SpecsChangeIndex }) {
  const { t } = useTranslation();
  const missing = change.artifacts.filter((a) => !a.optional && (!a.exists || !a.nonEmpty));

  return (
    <div className="rounded-lg border border-border/50 bg-background/40 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-mono font-medium">{change.changeId}</span>
        <span className="text-[10px] text-muted-foreground tabular-nums">
          {change.artifactCompleteness}%
        </span>
      </div>
      <CompletenessBar value={change.artifactCompleteness} />
      {change.taskSummary && (
        <p className="text-[10px] text-muted-foreground">
          {t("flowOrchestrator.tasks", {
            defaultValue: "任务 {{done}}/{{total}} 完成",
            done: change.taskSummary.done,
            total: change.taskSummary.total,
          })}
        </p>
      )}
      {missing.length > 0 && (
        <p className="text-[10px] text-amber-600 dark:text-amber-400">
          {t("flowOrchestrator.missingArtifacts", {
            defaultValue: "缺: {{list}}",
            list: missing.map((a) => a.file).join(", "),
          })}
        </p>
      )}
    </div>
  );
}

function FlowStepCard({
  done,
  title,
  description,
}: {
  done: boolean;
  title: string;
  description: string;
}) {
  return (
    <div
      className={cn(
        "rounded-lg border px-3 py-2 space-y-1",
        done
          ? "border-emerald-500/30 bg-emerald-500/10"
          : "border-border/60 bg-background/40",
      )}
    >
      <div className="flex items-center gap-1.5 text-xs font-medium">
        {done ? (
          <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
        ) : (
          <XCircle className="h-3.5 w-3.5 text-muted-foreground/50" />
        )}
        <span>{title}</span>
      </div>
      <p className="text-[10px] text-muted-foreground leading-relaxed">
        {description}
      </p>
    </div>
  );
}

function SupportStatusMatrix({
  hasWorkflowProfile,
  hasRecipeSpecs,
  hasFlowConfig,
}: {
  hasWorkflowProfile: boolean;
  hasRecipeSpecs: boolean;
  hasFlowConfig: boolean;
}) {
  const rows = [
    {
      label: "项目流程与上下文",
      status: hasWorkflowProfile ? "已自动写入" : "待自动写入",
      detail: "保存团队约定，并刷新 Agent 可读的项目上下文索引。",
      tone: hasWorkflowProfile ? "emerald" : "muted",
    },
    {
      label: "变更执行工件",
      status: hasRecipeSpecs ? "已自动写入" : "需手动生成变更材料",
      detail: "到“变更执行方案”生成需求、设计、任务材料；未生成前 Agent/CI 只能提示缺失。",
      tone: hasRecipeSpecs ? "emerald" : "amber",
    },
    {
      label: "自动检查配置",
      status: hasFlowConfig ? "已自动写入" : "待自动写入",
      detail: "导出后生成检查规则与风险分层评审策略；GitHub Actions 模板只在缺失时创建。",
      tone: hasFlowConfig ? "emerald" : "muted",
    },
    {
      label: "Agent 规则桥接",
      status: hasWorkflowProfile || hasFlowConfig ? "已自动写入" : "只读发现",
      detail: "AGENTS.md / CLAUDE.md / GEMINI.md 只注入 OpenSunstar 管理段，保留用户内容。",
      tone: hasWorkflowProfile || hasFlowConfig ? "emerald" : "muted",
    },
    {
      label: "多 Agent 深度适配",
      status: "实验性",
      detail: "当前先保证项目上下文与门禁可消费；不同 Agent 的原生能力后续按适配器分级开放。",
      tone: "blue",
    },
  ];

  const toneClass = (tone: string) =>
    tone === "emerald"
      ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border-emerald-500/20"
      : tone === "amber"
        ? "bg-amber-500/10 text-amber-700 dark:text-amber-400 border-amber-500/20"
        : tone === "blue"
          ? "bg-blue-500/10 text-blue-700 dark:text-blue-400 border-blue-500/20"
          : "bg-muted text-muted-foreground border-border/50";

  return (
    <div className="rounded-lg border border-border/50 bg-background/30 p-3 space-y-2">
      <div>
        <p className="text-xs font-semibold">落地状态矩阵</p>
        <p className="text-[10px] text-muted-foreground mt-0.5">
          明确哪些会自动写入、哪些需要手动确认，避免误以为所有能力都已完全接管。
        </p>
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        {rows.map((row) => (
          <div key={row.label} className="rounded-md border border-border/50 bg-muted/10 px-2.5 py-2">
            <div className="flex items-center justify-between gap-2">
              <span className="text-[11px] font-medium">{row.label}</span>
              <span className={cn("text-[10px] px-1.5 py-0.5 rounded-full border", toneClass(row.tone))}>
                {row.status}
              </span>
            </div>
            <p className="text-[10px] text-muted-foreground mt-1 leading-relaxed">{row.detail}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Collapsible section header */
function SectionToggle({
  label,
  count,
  expanded,
  onToggle,
}: {
  label: string;
  count?: string;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      className="flex items-center gap-1.5 text-xs font-semibold text-foreground hover:text-primary transition-colors"
      onClick={onToggle}
    >
      {expanded ? (
        <ChevronDown className="h-3.5 w-3.5" />
      ) : (
        <ChevronRight className="h-3.5 w-3.5" />
      )}
      {label}
      {count && <span className="text-[10px] text-muted-foreground font-normal">({count})</span>}
    </button>
  );
}

export function ProjectFlowOrchestratorPanel({
  projectId,
  initialPresetId,
}: ProjectFlowOrchestratorPanelProps) {
  const { t } = useTranslation();
  const appliedInitialPreset = useRef(false);

  // --- Core state ---
  const [presets, setPresets] = useState<WorkflowPresetSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [configExporting, setConfigExporting] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [validating, setValidating] = useState(false);
  const [profileExportPlan, setProfileExportPlan] = useState<FlowWritePlan | null>(null);
  const [flowConfigExportPlan, setFlowConfigExportPlan] = useState<FlowWritePlan | null>(null);
  const [strictSemantics, setStrictSemantics] = useState(true);

  const [presetId, setPresetId] = useState("standard");
  const [projectType, setProjectType] =
    useState<(typeof PROJECT_TYPES)[number]>("backend");
  const [index, setIndex] = useState<SpecsWorkflowIndex | null>(null);
  const [selectedChangeId, setSelectedChangeId] = useState("");
  const [targetStage, setTargetStage] = useState("3-task");
  const [gateResult, setGateResult] = useState<StageGateResult | null>(null);
  const [orchestrationLog, setOrchestrationLog] = useState<OrchestrationLogEntry[]>([]);

  // --- A-1: Module multi-select ---
  const [modules, setModules] = useState<WorkflowModule[]>([]);
  const [selectedModules, setSelectedModules] = useState<Set<string>>(new Set());
  const [modulesExpanded, setModulesExpanded] = useState(false);

  // --- A-2/A-4: Full preset + stage trimmer ---
  const [fullPreset, setFullPreset] = useState<WorkflowPreset | null>(null);
  const [resolvedStages, setResolvedStages] = useState<string[]>([]);
  const [enabledStages, setEnabledStages] = useState<Set<string>>(new Set());
  const [stagesExpanded, setStagesExpanded] = useState(false);
  const [advancedDetailsExpanded, setAdvancedDetailsExpanded] = useState(false);

  const selectedPreset = useMemo(
    () => presets.find((p) => p.id === presetId),
    [presets, presetId],
  );

  const loadOrchestrationLog = useCallback(async () => {
    try {
      const entries = await flowOrchestratorApi.readOrchestrationLog(projectId, 8);
      setOrchestrationLog(entries);
    } catch (error) {
      console.warn("[ProjectFlowOrchestratorPanel] load orchestration log failed", error);
    }
  }, [projectId]);

  // --- Load presets + modules on mount ---
  const loadPresets = useCallback(async () => {
    setLoading(true);
    try {
      const [presetList, moduleList] = await Promise.all([
        flowOrchestratorApi.listPresets(projectId),
        flowOrchestratorApi.listModules(projectId),
      ]);
      setPresets(presetList);
      setModules(moduleList);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  // --- Fetch full preset detail when presetId changes ---
  useEffect(() => {
    let cancelled = false;
    flowOrchestratorApi
      .getPreset(presetId, projectId)
      .then((p) => {
        if (cancelled) return;
        setFullPreset(p);
        // Initialize selected modules from preset defaults
        setSelectedModules(new Set(p.modules));
        // Compute resolved stages for current projectType
        const pathStages = p.paths[projectType] ?? p.paths.backend ?? [];
        setResolvedStages(pathStages);
        setEnabledStages(new Set(pathStages));
        // Update target stage to first available
        if (pathStages.length > 0 && !pathStages.includes(targetStage)) {
          setTargetStage(pathStages[0]);
        }
      })
      .catch(() => {
        // preset fetch failure is non-fatal
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [presetId, projectId]);

  // --- Recompute resolved stages when projectType changes ---
  useEffect(() => {
    if (!fullPreset) return;
    const pathStages = fullPreset.paths[projectType] ?? fullPreset.paths.backend ?? [];
    setResolvedStages(pathStages);
    setEnabledStages(new Set(pathStages));
    if (pathStages.length > 0 && !pathStages.includes(targetStage)) {
      setTargetStage(pathStages[0]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectType, fullPreset]);

  const refreshScan = useCallback(async () => {
    setScanning(true);
    try {
      const result = await flowOrchestratorApi.scanProject(
        projectId,
        presetId,
        projectType,
      );
      setIndex(result);
      if (result.savedProfile?.presetId) {
        setPresetId(result.savedProfile.presetId);
      }
      if (result.savedProfile?.projectType) {
        const pt = result.savedProfile.projectType as (typeof PROJECT_TYPES)[number];
        if (PROJECT_TYPES.includes(pt)) setProjectType(pt);
      }
      const active =
        result.activeChangeId ?? result.changes[0]?.changeId ?? "";
      setSelectedChangeId(active);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setScanning(false);
    }
  }, [projectId, presetId, projectType]);

  useEffect(() => {
    void loadPresets();
  }, [loadPresets]);

  useEffect(() => {
    void loadOrchestrationLog();
  }, [loadOrchestrationLog]);

  useEffect(() => {
    appliedInitialPreset.current = false;
  }, [projectId]);

  useEffect(() => {
    if (
      appliedInitialPreset.current ||
      !initialPresetId ||
      presets.length === 0
    ) {
      return;
    }
    if (presets.some((p) => p.id === initialPresetId)) {
      setPresetId(initialPresetId);
      appliedInitialPreset.current = true;
    }
  }, [initialPresetId, presets]);

  useEffect(() => {
    void refreshScan();
  }, [refreshScan]);

  // --- Compute disabled stages for export ---
  const disabledStages = useMemo(
    () => resolvedStages.filter((s) => !enabledStages.has(s)),
    [resolvedStages, enabledStages],
  );
  const selectedChangeIdError = useMemo(
    () => (selectedChangeId ? validateChangeId(selectedChangeId) : null),
    [selectedChangeId],
  );

  const ciValidateCommand = useMemo(
    () =>
      `os flow validate --project-path . --project-type ${projectType} --change-id ${
        selectedChangeId || "<change-id>"
      } --target-stage ${targetStage} --strict --json`,
    [projectType, selectedChangeId, targetStage],
  );

  const githubActionsSnippet = useMemo(
    () =>
      [
        "- name: OpenSunstar flow gate",
        `  run: ${ciValidateCommand}`,
      ].join("\n"),
    [ciValidateCommand],
  );

  // --- Handlers ---
  const toggleModule = (moduleId: string) => {
    setSelectedModules((prev) => {
      const next = new Set(prev);
      if (next.has(moduleId)) next.delete(moduleId);
      else next.add(moduleId);
      return next;
    });
  };

  const toggleStage = (stageId: string) => {
    setEnabledStages((prev) => {
      const next = new Set(prev);
      if (next.has(stageId)) next.delete(stageId);
      else next.add(stageId);
      return next;
    });
  };

  const showSemanticWarnings = (warnings?: string[]) => {
    if (warnings && warnings.length > 0) {
      toast.warning(
        `配置检查发现 ${warnings.length} 条提示:\n${warnings.join("\n")}`,
        { duration: 8000 },
      );
    }
  };

  const handleExportPreview = async () => {
    setExporting(true);
    try {
      if (selectedChangeIdError) {
        toast.error(selectedChangeIdError);
        return;
      }
      const plan = await flowOrchestratorApi.previewProfileExport(
        projectId,
        presetId,
        projectType,
        selectedChangeId || undefined,
        Array.from(selectedModules),
        disabledStages.length > 0 ? disabledStages : undefined,
      );
      setProfileExportPlan(plan);
      showSemanticWarnings(plan.semanticWarnings);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExporting(false);
    }
  };

  const handleExportConfirm = async () => {
    setProfileExportPlan(null);
    setExporting(true);
    try {
      if (selectedChangeIdError) {
        toast.error(selectedChangeIdError);
        return;
      }
      const profile = await flowOrchestratorApi.exportProfile(
        projectId,
        presetId,
        projectType,
        selectedChangeId || undefined,
        Array.from(selectedModules),
        disabledStages.length > 0 ? disabledStages : undefined,
        strictSemantics,
      );
      toast.success(
        t("flowOrchestrator.exportOk", {
          defaultValue: "已导出 .opensunstar/workflow.profile.json",
        }),
      );
      // Show semantic validation warnings if any (S1-S5)
      showSemanticWarnings(profile?.semanticWarnings);
      await refreshScan();
      await loadOrchestrationLog();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExporting(false);
    }
  };

  const handleExportFlowConfigPreview = async () => {
    setConfigExporting(true);
    try {
      const plan = await flowOrchestratorApi.previewFlowConfigExport(
        projectId,
        presetId,
        projectType,
        Array.from(selectedModules),
        disabledStages.length > 0 ? disabledStages : undefined,
      );
      setFlowConfigExportPlan(plan);
      showSemanticWarnings(plan.semanticWarnings);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setConfigExporting(false);
    }
  };

  const handleExportFlowConfigConfirm = async () => {
    setFlowConfigExportPlan(null);
    setConfigExporting(true);
    try {
      const config: FlowConfig = await flowOrchestratorApi.exportFlowConfig(
        projectId,
        presetId,
        projectType,
        Array.from(selectedModules),
        disabledStages.length > 0 ? disabledStages : undefined,
        strictSemantics,
      );
      toast.success(
        t("flowOrchestrator.flowConfigOk", {
          defaultValue: "已导出 .opensunstar/flow-config.yaml (R9.6 安全阀已注入)",
        }),
      );
      // Show rules summary in toast
      const rules = config.rules;
      toast(
        t("flowOrchestrator.r96Summary", {
          defaultValue:
            "R9.6: max_retry={{retry}}, role_sep={{sep}}, diff_boundary={{diff}}",
          retry: rules.max_auto_retry,
          sep: rules.role_separation ? "ON" : "OFF",
          diff: rules.require_diff_boundary ? "ON" : "OFF",
        }),
      );
      // Show semantic validation warnings if any (S1-S5)
      showSemanticWarnings(config.semantic_warnings);
      await refreshScan();
      await loadOrchestrationLog();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setConfigExporting(false);
    }
  };

  const handleCopyCiCommand = async () => {
    try {
      await navigator.clipboard.writeText(githubActionsSnippet);
      toast.success(
        t("flowOrchestrator.ciSnippetCopied", {
          defaultValue: "已复制 CI 门禁片段",
        }),
      );
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleRestoreLatestReceipt = async () => {
    setRestoring(true);
    try {
      const receipt = await flowOrchestratorApi.restoreLatestReceipt(projectId);
      toast.success(
        t("flowOrchestrator.rollbackOk", {
          defaultValue: "已恢复最近一次编排变更（{{count}} 个文件操作）",
          count: receipt.steps.length,
        }),
      );
      await refreshScan();
      await loadOrchestrationLog();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setRestoring(false);
    }
  };

  const handleValidateGate = async () => {
    if (!selectedChangeId) {
      toast.error(
        t("flowOrchestrator.pickChange", { defaultValue: "请先选择 change" }),
      );
      return;
    }
    if (selectedChangeIdError) {
      toast.error(selectedChangeIdError);
      return;
    }
    setValidating(true);
    try {
      const result = await flowOrchestratorApi.validateStageGate(projectId, {
        presetId,
        projectType,
        changeId: selectedChangeId,
        targetStage,
      });
      setGateResult(result);
      await loadOrchestrationLog();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setValidating(false);
    }
  };

  // --- Stage name lookup for display ---
  const stageNameMap = useMemo(() => {
    const map = new Map<string, string>();
    fullPreset?.stages.forEach((s) => map.set(s.id, s.name));
    return map;
  }, [fullPreset]);

  const hasWorkflowProfile = Boolean(index?.savedProfile);
  const hasRecipeSpecs = Boolean(index?.hasSpecsDir && index.changes.length > 0);
  const hasFlowConfig = Boolean(index?.hasFlowConfig);

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
        <Loader2 className="h-4 w-4 animate-spin" />
        {t("flowOrchestrator.loading", { defaultValue: "加载流程预设…" })}
      </div>
    );
  }

  return (
    <section className="rounded-xl border border-border/60 bg-muted/20 p-4 space-y-4">
      {/* Header */}
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          <Workflow className="h-4 w-4 text-primary shrink-0" />
          <div>
            <h3 className="text-sm font-semibold text-foreground">
              {t("flowOrchestrator.title", { defaultValue: "项目工作规则" })}
            </h3>
            <p className="text-[11px] text-muted-foreground mt-0.5">
              {t("flowOrchestrator.subtitle", {
                defaultValue: "flow-kit 兼容 · 档位 + 模块 + .specs 索引 + 阶段门禁",
              })}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-1.5">
          {index?.hasFlowKit && (
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20">
              flow-kit
            </span>
          )}
          {index?.hasSpecsDir && (
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20">
              .specs
            </span>
          )}
          {index && !index.workspaceExists && (
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-red-500/10 text-red-600 border border-red-500/20">
              {t("flowOrchestrator.pathInvalid", { defaultValue: "路径无效" })}
            </span>
          )}
        </div>
      </div>

      {/* Preset + Project Type selectors */}
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="space-y-1">
          <span className="text-[11px] text-muted-foreground">
            {t("flowOrchestrator.preset", { defaultValue: "流程档位" })}
          </span>
          <select
            className="w-full h-9 rounded-md border border-input bg-background px-2 text-sm"
            value={presetId}
            onChange={(e) => setPresetId(e.target.value)}
          >
            {presets.map((p) => (
              <option key={p.id} value={p.id}>
                {p.nameZh ?? p.name}
                {p.r3Tier ? ` (${p.r3Tier})` : ""}
              </option>
            ))}
          </select>
          {selectedPreset && (
            <p className="text-[10px] text-muted-foreground">{selectedPreset.description}</p>
          )}
        </label>

        <label className="space-y-1">
          <span className="text-[11px] text-muted-foreground">
            {t("flowOrchestrator.projectType", { defaultValue: "项目类型" })}
          </span>
          <select
            className="w-full h-9 rounded-md border border-input bg-background px-2 text-sm"
            value={projectType}
            onChange={(e) =>
              setProjectType(e.target.value as (typeof PROJECT_TYPES)[number])
            }
          >
            {PROJECT_TYPES.map((pt) => (
              <option key={pt} value={pt}>
                {pt}
              </option>
            ))}
          </select>
        </label>
      </div>

      {/* Delivery loop guidance */}
      <div className="rounded-lg border border-border/60 bg-background/35 p-3 space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="text-xs font-semibold">
              {t("flowOrchestrator.loopTitle", {
                defaultValue: "让项目规则真正生效",
              })}
            </p>
            <p className="text-[10px] text-muted-foreground mt-0.5 leading-relaxed">
              {t("flowOrchestrator.loopHint", {
                defaultValue:
                  "先保存项目工作规则，再生成变更材料，最后接入自动检查；高级文件细节可在下方展开查看。",
              })}
            </p>
          </div>
          <label className="flex items-start gap-2 rounded-md border border-border/50 bg-muted/20 px-2 py-1.5 cursor-pointer">
            <Checkbox
              checked={strictSemantics}
              onCheckedChange={(checked) => setStrictSemantics(checked === true)}
              className="mt-0.5"
            />
            <span className="space-y-0.5">
              <span className="block text-[11px] font-medium">
                {t("flowOrchestrator.strictMode", {
                  defaultValue: "发现明显冲突时先拦住",
                })}
              </span>
              <span className="block text-[10px] text-muted-foreground leading-relaxed">
                {t("flowOrchestrator.strictModeHint", {
                  defaultValue:
                    "默认开启。适合团队协作；探索期可关闭，先生成文件再人工确认。",
                })}
              </span>
            </span>
          </label>
        </div>

        <div className="grid gap-2 md:grid-cols-3">
          <FlowStepCard
            done={hasWorkflowProfile}
            title={t("flowOrchestrator.stepProfile", {
              defaultValue: "1. 保存项目流程",
            })}
            description={
              hasWorkflowProfile
                ? t("flowOrchestrator.stepProfileDone", {
                    defaultValue: "项目工作规则已保存",
                  })
                : t("flowOrchestrator.stepProfileTodo", {
                    defaultValue: "选择档位、模块和阶段后保存",
                  })
            }
          />
          <FlowStepCard
            done={hasRecipeSpecs}
            title={t("flowOrchestrator.stepSpecs", {
              defaultValue: "2. 生成变更材料",
            })}
            description={
              hasRecipeSpecs
                ? t("flowOrchestrator.stepSpecsDone", {
                    defaultValue: "已检测到 .specs 变更工件",
                  })
                : t("flowOrchestrator.stepSpecsTodo", {
                    defaultValue: "到“变更执行方案”生成 .specs",
                  })
            }
          />
          <FlowStepCard
            done={hasFlowConfig}
            title={t("flowOrchestrator.stepCi", {
              defaultValue: "3. 接入门禁校验",
            })}
            description={
              hasFlowConfig
                ? t("flowOrchestrator.stepCiDone", {
                    defaultValue: "自动检查规则已生成",
                  })
                : t("flowOrchestrator.stepCiTodo", {
                    defaultValue: "导出 CI 检查配置并复制命令",
                  })
            }
          />
        </div>

        <SupportStatusMatrix
          hasWorkflowProfile={hasWorkflowProfile}
          hasRecipeSpecs={hasRecipeSpecs}
          hasFlowConfig={hasFlowConfig}
        />

        <div className="rounded-md bg-muted/30 border border-border/50 p-2 space-y-2">
          <SectionToggle
            label={t("flowOrchestrator.advancedDetails", { defaultValue: "高级详情：CI 命令与生成文件" })}
            expanded={advancedDetailsExpanded}
            onToggle={() => setAdvancedDetailsExpanded((v) => !v)}
          />
          {advancedDetailsExpanded && (
            <>
              <div className="flex items-center justify-between gap-2">
                <span className="text-[10px] font-medium text-muted-foreground">
                  {t("flowOrchestrator.ciSnippet", {
                    defaultValue: "GitHub Actions / 自动检查片段",
                  })}
                </span>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-6 px-2 text-[10px]"
                  onClick={() => void handleCopyCiCommand()}
                >
                  <Copy className="h-3 w-3 mr-1" />
                  {t("common.copy", { defaultValue: "复制" })}
                </Button>
              </div>
              <pre className="text-[10px] leading-relaxed overflow-x-auto whitespace-pre-wrap break-words font-mono text-muted-foreground">
                {githubActionsSnippet}
              </pre>
              <p className="text-[10px] text-muted-foreground leading-relaxed">
                自动写入：`.opensunstar/workflow.profile.json`、`.opensunstar/flow-config.yaml`、`.opensunstar/agent-context.md`。只在缺失时创建：`.github/workflows/opensunstar-flow-gate.yml`。
              </p>
              <div className="flex items-center justify-between gap-2 rounded-md border border-amber-500/20 bg-amber-500/5 px-2 py-2">
                <p className="text-[10px] text-muted-foreground leading-relaxed">
                  写入前会保存快照。如误操作，可按最近一次编排 receipt 恢复。
                </p>
                <Button
                  size="sm"
                  variant="outline"
                  className="h-7 px-2 text-[10px] shrink-0"
                  disabled={restoring}
                  onClick={() => void handleRestoreLatestReceipt()}
                >
                  {restoring ? <Loader2 className="h-3 w-3 mr-1 animate-spin" /> : null}
                  {t("flowOrchestrator.rollbackLatest", { defaultValue: "恢复上次编排" })}
                </Button>
              </div>
            </>
          )}
        </div>
      </div>

      {/* A-1: Module multi-select */}
      <div className="rounded-lg border border-border/50 bg-background/30 p-3 space-y-2">
        <SectionToggle
          label={t("flowOrchestrator.modules", { defaultValue: "高级：选择规则模块" })}
          count={`${selectedModules.size}/${modules.length}`}
          expanded={modulesExpanded}
          onToggle={() => setModulesExpanded((v) => !v)}
        />
        {modulesExpanded && (
          <div className="grid gap-1.5 sm:grid-cols-2">
            {modules.map((m) => (
              <label
                key={m.id}
                className="flex items-start gap-2 rounded-md px-2 py-1.5 hover:bg-muted/40 cursor-pointer transition-colors"
              >
                <Checkbox
                  checked={selectedModules.has(m.id)}
                  onCheckedChange={() => toggleModule(m.id)}
                  className="mt-0.5"
                />
                <div className="min-w-0">
                  <span className="text-[11px] font-medium leading-tight">
                    {m.nameZh ?? m.name}
                  </span>
                  <p className="text-[9px] text-muted-foreground leading-tight truncate">
                    {m.description}
                  </p>
                </div>
              </label>
            ))}
          </div>
        )}
      </div>

      {/* A-2: Stage trimmer */}
      {resolvedStages.length > 0 && (
        <div className="rounded-lg border border-border/50 bg-background/30 p-3 space-y-2">
          <SectionToggle
            label={t("flowOrchestrator.stages", { defaultValue: "高级：调整执行步骤" })}
            count={`${enabledStages.size}/${resolvedStages.length}`}
            expanded={stagesExpanded}
            onToggle={() => setStagesExpanded((v) => !v)}
          />
          {stagesExpanded && (
            <div className="space-y-1">
              {resolvedStages.map((sid) => (
                <label
                  key={sid}
                  className="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted/40 cursor-pointer transition-colors"
                >
                  <Checkbox
                    checked={enabledStages.has(sid)}
                    onCheckedChange={() => toggleStage(sid)}
                  />
                  <span className="text-[11px] font-mono">{sid}</span>
                  <span className="text-[10px] text-muted-foreground">
                    {stageNameMap.get(sid) ?? ""}
                  </span>
                </label>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="secondary" disabled={scanning} onClick={() => void refreshScan()}>
          {scanning ? <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" /> : null}
          {t("flowOrchestrator.rescan", { defaultValue: "刷新索引" })}
        </Button>
        <Button size="sm" disabled={exporting || !index?.workspaceExists} onClick={() => void handleExportPreview()}>
          {exporting ? <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" /> : <GitBranch className="h-3.5 w-3.5 mr-1" />}
          {t("flowOrchestrator.saveProjectFlow", { defaultValue: "保存项目流程" })}
        </Button>
        <Button
          size="sm"
          variant="outline"
          disabled={configExporting || !index?.workspaceExists}
          onClick={() => void handleExportFlowConfigPreview()}
        >
          {configExporting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
          ) : (
            <FileOutput className="h-3.5 w-3.5 mr-1" />
          )}
          {t("flowOrchestrator.exportCiConfig", { defaultValue: "导出 CI 检查配置" })}
        </Button>
      </div>

      {/* .specs Change Index */}
      <div className="space-y-2">
        <h4 className="text-xs font-semibold flex items-center gap-1.5">
          <ShieldCheck className="h-3.5 w-3.5" />
          {t("flowOrchestrator.specsIndex", { defaultValue: ".specs 变更索引" })}
        </h4>
        {scanning && !index ? (
          <p className="text-xs text-muted-foreground">{t("common.loading", { defaultValue: "加载中…" })}</p>
        ) : index && index.changes.length === 0 ? (
          <p className="text-xs text-muted-foreground">
            {t("flowOrchestrator.noChanges", {
              defaultValue: "暂无 change 目录。复制 flow-kit 并在 .specs/<id>/ 下创建工件。",
            })}
          </p>
        ) : (
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {index?.changes.map((c) => (
              <button
                key={c.changeId}
                type="button"
                className={cn(
                  "w-full text-left",
                  selectedChangeId === c.changeId && "ring-1 ring-primary rounded-lg",
                )}
                onClick={() => setSelectedChangeId(c.changeId)}
              >
                <ChangeRow change={c} />
              </button>
            ))}
          </div>
        )}
      </div>

      {/* A-4: Dynamic Stage Gate */}
      <div className="rounded-lg border border-dashed border-border/60 p-3 space-y-2">
        <p className="text-[11px] font-medium">
          {t("flowOrchestrator.stageGate", { defaultValue: "阶段门禁试算 (R2.7)" })}
        </p>
        <div className="flex flex-wrap gap-2 items-end">
          <label className="space-y-1 flex-1 min-w-[120px]">
            <span className="text-[10px] text-muted-foreground">target stage</span>
            <select
              className="w-full h-8 rounded-md border border-input bg-background px-2 text-xs"
              value={targetStage}
              onChange={(e) => setTargetStage(e.target.value)}
            >
              {resolvedStages.length > 0
                ? resolvedStages.map((s) => (
                    <option key={s} value={s}>
                      {s} {stageNameMap.get(s) ? `(${stageNameMap.get(s)})` : ""}
                    </option>
                  ))
                : [
                    "0-change",
                    "1-requirement",
                    "2-design",
                    "3-task",
                    "4-dev",
                    "5-test",
                    "6-review",
                    "7-integration",
                  ].map((s) => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
            </select>
          </label>
          <Button size="sm" variant="outline" disabled={validating} onClick={() => void handleValidateGate()}>
            {validating ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : t("flowOrchestrator.checkGate", { defaultValue: "检查" })}
          </Button>
        </div>
        {gateResult && (
          <div
            className={cn(
              "text-xs rounded-md px-2 py-2 flex gap-2",
              gateResult.allowed
                ? "bg-emerald-500/10 text-emerald-800 dark:text-emerald-300"
                : "bg-amber-500/10 text-amber-900 dark:text-amber-200",
            )}
          >
            {gateResult.allowed ? (
              <CheckCircle2 className="h-4 w-4 shrink-0 mt-0.5" />
            ) : (
              <XCircle className="h-4 w-4 shrink-0 mt-0.5" />
            )}
            <div>
              <p className="font-medium">
                {gateResult.allowed
                  ? t("flowOrchestrator.gatePass", { defaultValue: "可通过" })
                  : t("flowOrchestrator.gateBlock", { defaultValue: "缺少工件" })}
              </p>
              {!gateResult.allowed && gateResult.missingArtifacts.length > 0 && (
                <ul className="mt-1 list-disc list-inside text-[10px] opacity-90">
                  {gateResult.missingArtifacts.map((m) => (
                    <li key={m}>{m}</li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Orchestration activity timeline */}
      <div className="rounded-lg border border-border/60 bg-background/30 p-3 space-y-2">
        <div className="flex items-center justify-between gap-2">
          <h4 className="text-xs font-semibold flex items-center gap-1.5">
            <History className="h-3.5 w-3.5" />
            {t("flowOrchestrator.activity", { defaultValue: "最近编排操作" })}
          </h4>
          <Button
            size="sm"
            variant="ghost"
            className="h-6 px-2 text-[10px]"
            onClick={() => void loadOrchestrationLog()}
          >
            {t("common.refresh", { defaultValue: "刷新" })}
          </Button>
        </div>
        {orchestrationLog.length === 0 ? (
          <p className="text-[10px] text-muted-foreground">
            {t("flowOrchestrator.noActivity", {
              defaultValue: "暂无编排操作。保存流程、生成自动检查配置或运行检查后会显示在这里。",
            })}
          </p>
        ) : (
          <div className="space-y-1.5 max-h-36 overflow-y-auto">
            {orchestrationLog.map((entry, index) => (
              <div
                key={`${entry.ts ?? "no-ts"}-${entry.event}-${index}`}
                className="rounded-md border border-border/40 bg-muted/20 px-2 py-1.5"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-[11px] font-medium truncate">
                    {entry.summary}
                  </span>
                  <span className="text-[9px] text-muted-foreground shrink-0">
                    {entry.ts ? new Date(entry.ts).toLocaleString() : "-"}
                  </span>
                </div>
                <p className="text-[9px] text-muted-foreground font-mono mt-0.5">
                  {entry.event}
                </p>
              </div>
            ))}
          </div>
        )}
      </div>

      {profileExportPlan && (
        <InstallConfirmModal
          open={!!profileExportPlan}
          title={t("flowOrchestrator.confirmProfileTitle", {
            defaultValue: "确认保存项目流程",
          })}
          confirmLabel={t("flowOrchestrator.confirmProfile", {
            defaultValue: "确认保存",
          })}
          files={profileExportPlan.files}
          audit={profileExportPlan.audit}
          onConfirm={() => void handleExportConfirm()}
          onCancel={() => setProfileExportPlan(null)}
        />
      )}

      {flowConfigExportPlan && (
        <InstallConfirmModal
          open={!!flowConfigExportPlan}
          title={t("flowOrchestrator.confirmFlowConfigTitle", {
            defaultValue: "确认导出 CI 检查配置",
          })}
          confirmLabel={t("flowOrchestrator.confirmFlowConfig", {
            defaultValue: "确认导出",
          })}
          files={flowConfigExportPlan.files}
          audit={flowConfigExportPlan.audit}
          onConfirm={() => void handleExportFlowConfigConfirm()}
          onCancel={() => setFlowConfigExportPlan(null)}
        />
      )}
    </section>
  );
}
