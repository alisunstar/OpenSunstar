import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ChefHat,
  ChevronDown,
  ChevronRight,
  Copy,
  Download,
  Eye,
  FileText,
  FolderOpen,
  Loader2,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  recipeComposerApi,
  type InstallResult,
  type RecipeComposeParams,
  type RecipeInstallPlan,
  type StageGraph,
} from "@/lib/api/recipeComposer";
import { InstallConfirmModal } from "@/components/shared/InstallConfirmModal";
import {
  flowOrchestratorApi,
  type WorkflowModule,
  type WorkflowPreset,
  type WorkflowPresetSummary,
} from "@/lib/api/flowOrchestrator";

const PROJECT_TYPES = ["backend", "frontend", "cli"] as const;

interface ProjectRecipeComposerProps {
  projectId: string;
}

// ──────────────────────── Stage Graph SVG ────────────────────────

const NODE_W = 140;
const NODE_H = 48;
const NODE_GAP_X = 40;
const NODE_GAP_Y = 16;
const LATERAL_GAP = 24;

function StageGraphSVG({ graph }: { graph: StageGraph }) {
  if (graph.nodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-32 text-xs text-muted-foreground">
        No stages in graph
      </div>
    );
  }

  // Group nodes by depth for layout
  const depthGroups = new Map<number, typeof graph.nodes>();
  for (const node of graph.nodes) {
    const group = depthGroups.get(node.depth) ?? [];
    group.push(node);
    depthGroups.set(node.depth, group);
  }

  const maxDepth = Math.max(...graph.nodes.map((n) => n.depth));
  const maxGroupSize = Math.max(
    ...Array.from(depthGroups.values()).map((g) => g.length),
  );

  const svgW = (maxDepth + 1) * (NODE_W + NODE_GAP_X) + NODE_GAP_X;
  const pipelineH =
    maxGroupSize * (NODE_H + NODE_GAP_Y) - NODE_GAP_Y + NODE_GAP_Y * 2;
  const lateralH =
    graph.lateralNodes.length > 0
      ? graph.lateralNodes.length * 32 + LATERAL_GAP + 24
      : 0;
  const svgH = pipelineH + lateralH;

  // Compute node positions
  const positions = new Map<string, { x: number; y: number }>();
  for (const [depth, group] of depthGroups) {
    const x = NODE_GAP_X + depth * (NODE_W + NODE_GAP_X);
    const totalH = group.length * (NODE_H + NODE_GAP_Y) - NODE_GAP_Y;
    const startY = (pipelineH - totalH) / 2;
    group.forEach((node, i) => {
      positions.set(node.id, {
        x,
        y: startY + i * (NODE_H + NODE_GAP_Y),
      });
    });
  }

  const nodeColor = (node: (typeof graph.nodes)[0]) => {
    if (node.condition === "branch") return "fill-amber-100 dark:fill-amber-900/40 stroke-amber-400";
    if (node.standalone === "true") return "fill-emerald-100 dark:fill-emerald-900/40 stroke-emerald-400";
    if (node.standalone === "semi") return "fill-blue-100 dark:fill-blue-900/40 stroke-blue-400";
    return "fill-muted stroke-border";
  };

  return (
    <div className="overflow-x-auto border border-border/50 rounded-lg bg-background/30 p-2">
      <svg
        width={svgW}
        height={svgH}
        viewBox={`0 0 ${svgW} ${svgH}`}
        className="min-w-full"
        style={{ minWidth: svgW }}
      >
        <defs>
          <marker
            id="arrowhead"
            markerWidth="8"
            markerHeight="6"
            refX="8"
            refY="3"
            orient="auto"
          >
            <polygon points="0 0, 8 3, 0 6" fill="currentColor" className="text-muted-foreground" />
          </marker>
        </defs>

        {/* Edges */}
        {graph.edges.map((edge, i) => {
          const from = positions.get(edge.source);
          const to = positions.get(edge.target);
          if (!from || !to) return null;
          const x1 = from.x + NODE_W;
          const y1 = from.y + NODE_H / 2;
          const x2 = to.x;
          const y2 = to.y + NODE_H / 2;
          const mx = (x1 + x2) / 2;
          return (
            <path
              key={`edge-${i}`}
              d={`M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`}
              fill="none"
              stroke="currentColor"
              strokeWidth={1.5}
              className="text-muted-foreground/60"
              markerEnd="url(#arrowhead)"
            />
          );
        })}

        {/* Pipeline nodes */}
        {graph.nodes.map((node) => {
          const pos = positions.get(node.id);
          if (!pos) return null;
          return (
            <g key={node.id}>
              <rect
                x={pos.x}
                y={pos.y}
                width={NODE_W}
                height={NODE_H}
                rx={8}
                ry={8}
                strokeWidth={1.5}
                className={nodeColor(node)}
              />
              <text
                x={pos.x + NODE_W / 2}
                y={pos.y + NODE_H / 2 - 4}
                textAnchor="middle"
                className="fill-foreground text-[11px] font-semibold"
              >
                {node.name}
              </text>
              <text
                x={pos.x + NODE_W / 2}
                y={pos.y + NODE_H / 2 + 10}
                textAnchor="middle"
                className="fill-muted-foreground text-[9px]"
              >
                {node.artifacts[0] ?? node.id}
              </text>
            </g>
          );
        })}

        {/* Lateral nodes */}
        {graph.lateralNodes.length > 0 && (
          <>
            <text
              x={NODE_GAP_X}
              y={pipelineH + 14}
              className="fill-muted-foreground text-[10px] font-medium"
            >
              Lateral (cross-cutting)
            </text>
            {graph.lateralNodes.map((node, i) => {
              const x = NODE_GAP_X + i * (NODE_W / 2 + 16);
              const y = pipelineH + 24;
              return (
                <g key={node.id}>
                  <rect
                    x={x}
                    y={y}
                    width={NODE_W / 2 + 8}
                    height={28}
                    rx={6}
                    ry={6}
                    strokeWidth={1}
                    className="fill-purple-100 dark:fill-purple-900/30 stroke-purple-400 stroke-dashed"
                  />
                  <text
                    x={x + (NODE_W / 2 + 8) / 2}
                    y={y + 17}
                    textAnchor="middle"
                    className="fill-foreground text-[9px] font-medium"
                  >
                    {node.name}
                  </text>
                </g>
              );
            })}
          </>
        )}
      </svg>

      {/* Legend */}
      <div className="flex gap-4 mt-2 px-1 text-[10px] text-muted-foreground">
        <span className="flex items-center gap-1">
          <span className="inline-block w-3 h-3 rounded bg-emerald-200 dark:bg-emerald-800 border border-emerald-400" />
          Standalone
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-3 h-3 rounded bg-blue-200 dark:bg-blue-800 border border-blue-400" />
          Semi-standalone
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-3 h-3 rounded bg-amber-200 dark:bg-amber-800 border border-amber-400" />
          Branch
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-3 h-3 rounded bg-muted border border-border" />
          No artifacts
        </span>
      </div>
    </div>
  );
}

// ──────────────────────── Collapsible Section ────────────────────────

function Section({
  label,
  count,
  expanded,
  onToggle,
  children,
}: {
  label: string;
  count?: string;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
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
        {count && (
          <span className="text-[10px] text-muted-foreground font-normal">
            ({count})
          </span>
        )}
      </button>
      {expanded && <div className="pl-5 space-y-1">{children}</div>}
    </div>
  );
}

// ──────────────────────── Main Component ────────────────────────

export function ProjectRecipeComposer({
  projectId,
}: ProjectRecipeComposerProps) {
  // --- Shared state ---
  const [presets, setPresets] = useState<WorkflowPresetSummary[]>([]);
  const [modules, setModules] = useState<WorkflowModule[]>([]);
  const [loading, setLoading] = useState(true);
  const [presetId, setPresetId] = useState("standard");
  const [projectType, setProjectType] =
    useState<(typeof PROJECT_TYPES)[number]>("backend");
  const [fullPreset, setFullPreset] = useState<WorkflowPreset | null>(null);

  // --- Stage Graph ---
  const [stageGraph, setStageGraph] = useState<StageGraph | null>(null);

  // --- Recipe composer state ---
  const [recipeName, setRecipeName] = useState("My Recipe");
  const [recipeDescription, setRecipeDescription] = useState("");
  const [recipeNotes, setRecipeNotes] = useState("");
  const [selectedModules, setSelectedModules] = useState<Set<string>>(new Set());
  const [enabledStages, setEnabledStages] = useState<Set<string>>(new Set());
  const [resolvedStages, setResolvedStages] = useState<string[]>([]);

  // --- UI state ---
  const [graphExpanded, setGraphExpanded] = useState(true);
  const [modulesExpanded, setModulesExpanded] = useState(false);
  const [stagesExpanded, setStagesExpanded] = useState(true);
  const [previewExpanded, setPreviewExpanded] = useState(false);

  // --- Preview / Export / Install state ---
  const [previewContent, setPreviewContent] = useState<string>("");
  const [previewing, setPreviewing] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState<InstallResult | null>(null);
  const [changeId, setChangeId] = useState("");
  const [savedRecipes, setSavedRecipes] = useState<string[]>([]);
  const [installPlan, setInstallPlan] = useState<RecipeInstallPlan | null>(null);

  // --- Load presets + modules ---
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all([
      flowOrchestratorApi.listPresets(projectId),
      flowOrchestratorApi.listModules(projectId),
    ])
      .then(([presetList, moduleList]) => {
        if (cancelled) return;
        setPresets(presetList);
        setModules(moduleList);
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [projectId]);

  // --- Load saved recipes ---
  useEffect(() => {
    recipeComposerApi
      .listSavedRecipes(projectId)
      .then(setSavedRecipes)
      .catch(() => {});
  }, [projectId]);

  // --- Fetch full preset + stage graph ---
  useEffect(() => {
    let cancelled = false;
    Promise.all([
      flowOrchestratorApi.getPreset(presetId, projectId),
      recipeComposerApi.buildStageGraph(presetId, projectId),
    ])
      .then(([preset, graph]) => {
        if (cancelled) return;
        setFullPreset(preset);
        setStageGraph(graph);
        setSelectedModules(new Set(preset.modules));
        const pathStages = preset.paths[projectType] ?? preset.paths.backend ?? [];
        setResolvedStages(pathStages);
        setEnabledStages(new Set(pathStages));
      })
      .catch(() => {});
    return () => { cancelled = true; };
  }, [presetId, projectId, projectType]);

  // --- Computed ---
  const disabledStages = useMemo(
    () => resolvedStages.filter((s) => !enabledStages.has(s)),
    [resolvedStages, enabledStages],
  );

  const stageMap = useMemo(() => {
    const map = new Map<string, { id: string; name: string }>();
    if (fullPreset) {
      for (const s of fullPreset.stages) {
        map.set(s.id, { id: s.id, name: s.name });
      }
    }
    return map;
  }, [fullPreset]);

  // --- Handlers ---
  const toggleModule = (id: string) => {
    setSelectedModules((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleStage = (id: string) => {
    setEnabledStages((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const buildParams = (): RecipeComposeParams => ({
    presetId,
    projectType,
    name: recipeName || "My Recipe",
    description: recipeDescription || null,
    selectedModules: Array.from(selectedModules),
    disabledStages: disabledStages.length > 0 ? disabledStages : null,
    notes: recipeNotes || null,
    stageDocs: null,
  });

  const handlePreview = useCallback(async () => {
    setPreviewing(true);
    try {
      const content = await recipeComposerApi.previewRecipe(
        projectId,
        buildParams(),
      );
      setPreviewContent(content);
      setPreviewExpanded(true);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setPreviewing(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, presetId, projectType, recipeName, recipeDescription, recipeNotes, selectedModules, disabledStages]);

  const handleExport = useCallback(async () => {
    setExporting(true);
    try {
      const content = await recipeComposerApi.exportRecipe(
        projectId,
        buildParams(),
      );
      setPreviewContent(content);
      toast.success(`Recipe "${recipeName}" exported to .opensunstar/recipe/`);
      // Refresh saved recipes
      const list = await recipeComposerApi.listSavedRecipes(projectId);
      setSavedRecipes(list);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExporting(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, presetId, projectType, recipeName, recipeDescription, recipeNotes, selectedModules, disabledStages]);

  const handleCopyPreview = useCallback(async () => {
    if (!previewContent) {
      await handlePreview();
      return;
    }
    try {
      await navigator.clipboard.writeText(previewContent);
      toast.success("Recipe content copied to clipboard");
    } catch {
      toast.error("Failed to copy");
    }
  }, [previewContent, handlePreview]);

  const handleInstallPreview = useCallback(async () => {
    if (!changeId.trim()) {
      toast.error("请填写 Change ID（如 feat-auth）");
      return;
    }
    setInstalling(true);
    try {
      const plan = await recipeComposerApi.previewInstallPlan(
        projectId,
        buildParams(),
        changeId.trim(),
      );
      setInstallPlan(plan);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setInstalling(false);
    }
  }, [projectId, buildParams, changeId]);

  const handleInstallConfirm = useCallback(async () => {
    setInstallPlan(null);
    setInstalling(true);
    try {
      const result = await recipeComposerApi.installRecipe(
        projectId,
        buildParams(),
        changeId.trim(),
      );
      setInstallResult(result);
      toast.success(
        `Recipe 已安装到项目：创建 ${result.filesCreated.length} 个文件` +
        (result.filesSkipped.length > 0 ? `，跳过 ${result.filesSkipped.length} 个已存在文件` : ""),
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      setInstalling(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, presetId, projectType, recipeName, recipeDescription, recipeNotes, selectedModules, disabledStages, changeId]);

  const handleDeleteRecipe = useCallback(
    async (name: string) => {
      try {
        await recipeComposerApi.deleteSavedRecipe(projectId, name);
        setSavedRecipes((prev) => prev.filter((n) => n !== name));
        toast.success(`Recipe "${name}" deleted`);
      } catch (e) {
        toast.error(String(e));
      }
    },
    [projectId],
  );

  if (loading) {
    return (
      <div className="flex items-center gap-2 p-4 text-xs text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        Loading recipe composer...
      </div>
    );
  }

  return (
    <div className="space-y-4 p-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ChefHat className="h-4 w-4 text-primary" />
          <h3 className="text-sm font-semibold">Recipe Composer</h3>
        </div>
        <span className="text-[10px] text-muted-foreground">
          YAML+Markdown hybrid
        </span>
      </div>

      {/* Preset + Project Type */}
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase">
            Preset
          </label>
          <select
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-xs"
            value={presetId}
            onChange={(e) => setPresetId(e.target.value)}
          >
            {presets.map((p) => (
              <option key={p.id} value={p.id}>
                {p.nameZh ?? p.name}
              </option>
            ))}
          </select>
        </div>
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase">
            Project Type
          </label>
          <select
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-xs"
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
        </div>
      </div>

      {/* Recipe metadata */}
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase">
            Recipe Name
          </label>
          <input
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-xs"
            value={recipeName}
            onChange={(e) => setRecipeName(e.target.value)}
            placeholder="My Recipe"
          />
        </div>
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase">
            Description
          </label>
          <input
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-xs"
            value={recipeDescription}
            onChange={(e) => setRecipeDescription(e.target.value)}
            placeholder="Recipe composed from standard preset"
          />
        </div>
      </div>

      {/* Stage Graph */}
      <Section
        label="Stage Graph"
        count={stageGraph ? `${stageGraph.nodes.length} nodes` : undefined}
        expanded={graphExpanded}
        onToggle={() => setGraphExpanded(!graphExpanded)}
      >
        {stageGraph && <StageGraphSVG graph={stageGraph} />}
      </Section>

      {/* Module multi-select */}
      <Section
        label="Modules"
        count={`${selectedModules.size}/${modules.length}`}
        expanded={modulesExpanded}
        onToggle={() => setModulesExpanded(!modulesExpanded)}
      >
        <div className="grid grid-cols-2 gap-1">
          {modules.map((m) => (
            <label
              key={m.id}
              className="flex items-center gap-1.5 text-[11px] cursor-pointer py-0.5"
            >
              <Checkbox
                checked={selectedModules.has(m.id)}
                onCheckedChange={() => toggleModule(m.id)}
                className="h-3.5 w-3.5"
              />
              <span className="truncate" title={m.description}>
                {m.nameZh ?? m.name}
              </span>
            </label>
          ))}
        </div>
      </Section>

      {/* Stage trimmer */}
      <Section
        label="Stages"
        count={`${enabledStages.size}/${resolvedStages.length} enabled`}
        expanded={stagesExpanded}
        onToggle={() => setStagesExpanded(!stagesExpanded)}
      >
        {resolvedStages.map((sid) => {
          const info = stageMap.get(sid);
          return (
            <label
              key={sid}
              className="flex items-center gap-1.5 text-[11px] cursor-pointer py-0.5"
            >
              <Checkbox
                checked={enabledStages.has(sid)}
                onCheckedChange={() => toggleStage(sid)}
                className="h-3.5 w-3.5"
              />
              <span className="font-mono text-muted-foreground">{sid}</span>
              <span className="text-foreground">
                {info?.name ?? sid}
              </span>
            </label>
          );
        })}
      </Section>

      {/* Notes */}
      <div className="space-y-1">
        <label className="text-[10px] font-medium text-muted-foreground uppercase">
          Notes (optional)
        </label>
        <textarea
          className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-xs min-h-[48px] resize-y"
          value={recipeNotes}
          onChange={(e) => setRecipeNotes(e.target.value)}
          placeholder="Custom notes for this recipe..."
          rows={2}
        />
      </div>

      {/* Change ID input */}
      <div className="space-y-1">
        <label className="text-[10px] font-medium text-muted-foreground uppercase">
          Change ID
        </label>
        <input
          className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-xs"
          value={changeId}
          onChange={(e) => setChangeId(e.target.value)}
          placeholder="e.g. feat-auth, fix-login-bug"
        />
      </div>

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="outline"
          onClick={handlePreview}
          disabled={previewing}
          className="text-xs"
        >
          {previewing ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
          ) : (
            <Eye className="h-3.5 w-3.5 mr-1" />
          )}
          Preview
        </Button>
        <Button
          size="sm"
          variant="outline"
          onClick={handleCopyPreview}
          disabled={previewing}
          className="text-xs"
        >
          <Copy className="h-3.5 w-3.5 mr-1" />
          Copy
        </Button>
        <Button
          size="sm"
          onClick={handleExport}
          disabled={exporting || !recipeName}
          className="text-xs"
        >
          {exporting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
          ) : (
            <Download className="h-3.5 w-3.5 mr-1" />
          )}
          Export Recipe
        </Button>
        <Button
          size="sm"
          onClick={handleInstallPreview}
          disabled={installing || !changeId.trim()}
          className="text-xs"
        >
          {installing ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
          ) : (
            <FolderOpen className="h-3.5 w-3.5 mr-1" />
          )}
          Install to Project
        </Button>
      </div>

      {/* Install result */}
      {installResult && (
        <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-3 space-y-2">
          <div className="flex items-center gap-2">
            <FolderOpen className="h-4 w-4 text-emerald-500" />
            <span className="text-xs font-semibold text-emerald-700 dark:text-emerald-400">
              Installed: {installResult.filesCreated.length} files created
              {installResult.filesSkipped.length > 0 &&
                `, ${installResult.filesSkipped.length} skipped (already exist)`}
            </span>
          </div>
          <div className="space-y-1 pl-6">
            {installResult.filesCreated.map((f) => (
              <div key={f} className="text-[10px] text-emerald-600 dark:text-emerald-400 font-mono">
                + {f}
              </div>
            ))}
            {installResult.filesSkipped.map((f) => (
              <div key={f} className="text-[10px] text-muted-foreground font-mono">
                ~ {f} (exists)
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Preview panel */}
      {previewExpanded && previewContent && (
        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-semibold text-muted-foreground uppercase">
              Preview (YAML+Markdown hybrid)
            </span>
            <Button
              size="sm"
              variant="ghost"
              className="h-5 px-1.5 text-[10px]"
              onClick={() => setPreviewExpanded(false)}
            >
              Collapse
            </Button>
          </div>
          <pre className="text-[10px] leading-relaxed bg-muted/50 border border-border/50 rounded-lg p-3 overflow-x-auto max-h-[400px] overflow-y-auto whitespace-pre-wrap break-words">
            {previewContent}
          </pre>
        </div>
      )}

      {/* Saved recipes */}
      {savedRecipes.length > 0 && (
        <div className="space-y-1.5">
          <span className="text-[10px] font-semibold text-muted-foreground uppercase">
            Saved Recipes ({savedRecipes.length})
          </span>
          <div className="space-y-1">
            {savedRecipes.map((name) => (
              <div
                key={name}
                className="flex items-center justify-between rounded-md border border-border/50 bg-background/40 px-2.5 py-1.5"
              >
                <div className="flex items-center gap-1.5">
                  <FileText className="h-3 w-3 text-muted-foreground" />
                  <span className="text-[11px] font-mono">{name}.recipe.md</span>
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-5 w-5 p-0 text-muted-foreground hover:text-destructive"
                  onClick={() => handleDeleteRecipe(name)}
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Pre-flight install confirmation modal */}
      {installPlan && (
        <InstallConfirmModal
          open={!!installPlan}
          files={installPlan.files}
          audit={installPlan.audit}
          onConfirm={handleInstallConfirm}
          onCancel={() => setInstallPlan(null)}
        />
      )}
    </div>
  );
}
