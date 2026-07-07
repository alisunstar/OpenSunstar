import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  FileOutput,
  GitBranch,
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
  type FlowConfig,
  type SpecsChangeIndex,
  type SpecsWorkflowIndex,
  type StageGateResult,
  type WorkflowModule,
  type WorkflowPreset,
  type WorkflowPresetSummary,
} from "@/lib/api/flowOrchestrator";
import { cn } from "@/lib/utils";

const PROJECT_TYPES = ["backend", "frontend", "cli"] as const;

interface ProjectFlowOrchestratorPanelProps {
  projectId: string;
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
}: ProjectFlowOrchestratorPanelProps) {
  const { t } = useTranslation();

  // --- Core state ---
  const [presets, setPresets] = useState<WorkflowPresetSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [configExporting, setConfigExporting] = useState(false);
  const [validating, setValidating] = useState(false);

  const [presetId, setPresetId] = useState("standard");
  const [projectType, setProjectType] =
    useState<(typeof PROJECT_TYPES)[number]>("backend");
  const [index, setIndex] = useState<SpecsWorkflowIndex | null>(null);
  const [selectedChangeId, setSelectedChangeId] = useState("");
  const [targetStage, setTargetStage] = useState("3-task");
  const [gateResult, setGateResult] = useState<StageGateResult | null>(null);

  // --- A-1: Module multi-select ---
  const [modules, setModules] = useState<WorkflowModule[]>([]);
  const [selectedModules, setSelectedModules] = useState<Set<string>>(new Set());
  const [modulesExpanded, setModulesExpanded] = useState(false);

  // --- A-2/A-4: Full preset + stage trimmer ---
  const [fullPreset, setFullPreset] = useState<WorkflowPreset | null>(null);
  const [resolvedStages, setResolvedStages] = useState<string[]>([]);
  const [enabledStages, setEnabledStages] = useState<Set<string>>(new Set());
  const [stagesExpanded, setStagesExpanded] = useState(false);

  const selectedPreset = useMemo(
    () => presets.find((p) => p.id === presetId),
    [presets, presetId],
  );

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
    void refreshScan();
  }, [refreshScan]);

  // --- Compute disabled stages for export ---
  const disabledStages = useMemo(
    () => resolvedStages.filter((s) => !enabledStages.has(s)),
    [resolvedStages, enabledStages],
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

  const handleExport = async () => {
    setExporting(true);
    try {
      const profile = await flowOrchestratorApi.exportProfile(
        projectId,
        presetId,
        projectType,
        selectedChangeId || undefined,
        Array.from(selectedModules),
        disabledStages.length > 0 ? disabledStages : undefined,
      );
      toast.success(
        t("flowOrchestrator.exportOk", {
          defaultValue: "已导出 .opensunstar/workflow.profile.json",
        }),
      );
      // Show semantic validation warnings if any (S1-S5)
      if (profile?.semanticWarnings && profile.semanticWarnings.length > 0) {
        toast.warning(
          `S1-S5 语义校验发现 ${profile.semanticWarnings.length} 条警告:\n${profile.semanticWarnings.join("\n")}`,
          { duration: 8000 },
        );
      }
      await refreshScan();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExporting(false);
    }
  };

  const handleExportFlowConfig = async () => {
    setConfigExporting(true);
    try {
      const config: FlowConfig = await flowOrchestratorApi.exportFlowConfig(
        projectId,
        presetId,
        projectType,
        Array.from(selectedModules),
        disabledStages.length > 0 ? disabledStages : undefined,
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
      if (config.semantic_warnings && config.semantic_warnings.length > 0) {
        toast.warning(
          `S1-S5 语义校验发现 ${config.semantic_warnings.length} 条警告:\n${config.semantic_warnings.join("\n")}`,
          { duration: 8000 },
        );
      }
    } catch (e) {
      toast.error(String(e));
    } finally {
      setConfigExporting(false);
    }
  };

  const handleValidateGate = async () => {
    if (!selectedChangeId) {
      toast.error(
        t("flowOrchestrator.pickChange", { defaultValue: "请先选择 change" }),
      );
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
              {t("flowOrchestrator.title", { defaultValue: "SDD 流程编排" })}
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

      {/* A-1: Module multi-select */}
      <div className="rounded-lg border border-border/50 bg-background/30 p-3 space-y-2">
        <SectionToggle
          label={t("flowOrchestrator.modules", { defaultValue: "方法论模块" })}
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
            label={t("flowOrchestrator.stages", { defaultValue: "阶段裁剪" })}
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
        <Button size="sm" disabled={exporting || !index?.workspaceExists} onClick={() => void handleExport()}>
          {exporting ? <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" /> : <GitBranch className="h-3.5 w-3.5 mr-1" />}
          {t("flowOrchestrator.exportProfile", { defaultValue: "导出 Profile" })}
        </Button>
        <Button
          size="sm"
          variant="outline"
          disabled={configExporting || !index?.workspaceExists}
          onClick={() => void handleExportFlowConfig()}
        >
          {configExporting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
          ) : (
            <FileOutput className="h-3.5 w-3.5 mr-1" />
          )}
          {t("flowOrchestrator.exportFlowConfig", { defaultValue: "导出 FlowConfig" })}
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
    </section>
  );
}
