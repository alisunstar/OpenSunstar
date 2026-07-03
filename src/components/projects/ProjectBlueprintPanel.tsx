import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Layers, Loader2, FileJson, Check } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  blueprintApi,
  type Blueprint,
  type BlueprintApplyPreview,
} from "@/lib/api/blueprint";
import { projectsApi } from "@/lib/api/projects";
import { cn } from "@/lib/utils";

const TARGET_OPTIONS = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "openclaw",
  "hermes",
] as const;

interface ProjectBlueprintPanelProps {
  projectId: string;
  onApplied?: () => void;
}

export function ProjectBlueprintPanel({
  projectId,
  onApplied,
}: ProjectBlueprintPanelProps) {
  const { t } = useTranslation();
  const [blueprints, setBlueprints] = useState<Blueprint[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [preview, setPreview] = useState<BlueprintApplyPreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [targetApp, setTargetApp] = useState<string>("");
  const [savingTarget, setSavingTarget] = useState(false);
  const [exporting, setExporting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [bps, project] = await Promise.all([
        blueprintApi.list(),
        projectsApi.getById(projectId),
      ]);
      setBlueprints(bps);
      if (project?.target_app) setTargetApp(project.target_app);
      if (project?.blueprint_id) setSelectedId(project.blueprint_id);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const handlePreview = async (id: string) => {
    setSelectedId(id);
    setPreviewLoading(true);
    try {
      const p = await blueprintApi.previewApply(projectId, id);
      setPreview(p);
    } catch (e) {
      toast.error(String(e));
      setPreview(null);
    } finally {
      setPreviewLoading(false);
    }
  };

  const handleApply = async () => {
    if (!selectedId) return;
    setApplying(true);
    try {
      const p = await blueprintApi.apply(projectId, selectedId);
      setPreview(p);
      toast.success(
        t("projectBlueprint.applied", {
          name: p.blueprintName,
          count: p.toLink.length,
          defaultValue: `已应用「${p.blueprintName}」，关联 ${p.toLink.length} 项资产`,
        }),
      );
      onApplied?.();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setApplying(false);
    }
  };

  const handleSaveTarget = async () => {
    setSavingTarget(true);
    try {
      await projectsApi.setTargetApp(projectId, targetApp || null);
      toast.success(
        t("projectBlueprint.targetSaved", { defaultValue: "项目目标 CLI 已保存" }),
      );
      onApplied?.();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSavingTarget(false);
    }
  };

  const handleExportSnapshot = async () => {
    setExporting(true);
    try {
      const path = await blueprintApi.exportBaselineSnapshot(projectId);
      toast.success(
        t("projectBlueprint.snapshotExported", {
          path,
          defaultValue: `基线快照已导出：${path}`,
        }),
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExporting(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
        <Loader2 className="h-4 w-4 animate-spin" />
        {t("projectBlueprint.loading", { defaultValue: "加载 Blueprint…" })}
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-border/60 bg-muted/15 p-4 space-y-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
            <Layers className="h-4 w-4 text-primary" />
            {t("projectBlueprint.title", { defaultValue: "项目基线 Blueprint" })}
          </h3>
          <p className="text-[11px] text-muted-foreground mt-1">
            {t("projectBlueprint.hint", {
              defaultValue:
                "选择模板一键关联全局库资产；合并模式仅新增关联，不删除已有配置。",
            })}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={exporting}
          onClick={() => void handleExportSnapshot()}
        >
          {exporting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
          ) : (
            <FileJson className="h-3.5 w-3.5 mr-1" />
          )}
          {t("projectBlueprint.exportSnapshot", { defaultValue: "导出快照" })}
        </Button>
      </div>

      <div className="flex flex-wrap items-end gap-2">
        <label className="text-xs text-muted-foreground flex flex-col gap-1">
          {t("projectBlueprint.targetCli", { defaultValue: "项目目标 CLI" })}
          <select
            className="rounded-md border border-border bg-background px-2 py-1 text-sm min-w-[140px]"
            value={targetApp}
            onChange={(e) => setTargetApp(e.target.value)}
          >
            <option value="">
              {t("projectBlueprint.targetDefault", {
                defaultValue: "沿用看板默认",
              })}
            </option>
            {TARGET_OPTIONS.map((app) => (
              <option key={app} value={app}>
                {app}
              </option>
            ))}
          </select>
        </label>
        <Button
          variant="secondary"
          size="sm"
          disabled={savingTarget}
          onClick={() => void handleSaveTarget()}
        >
          {savingTarget && <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />}
          {t("projectBlueprint.saveTarget", { defaultValue: "保存" })}
        </Button>
      </div>

      <div className="grid gap-2 sm:grid-cols-3">
        {blueprints.map((bp) => (
          <button
            key={bp.id}
            type="button"
            className={cn(
              "text-left rounded-lg border p-3 transition-colors",
              selectedId === bp.id
                ? "border-primary/50 bg-primary/5"
                : "border-border/60 hover:border-primary/30",
            )}
            onClick={() => void handlePreview(bp.id)}
          >
            <p className="text-sm font-medium text-foreground">{bp.name}</p>
            <p className="text-[10px] text-muted-foreground mt-1 line-clamp-2">
              {bp.description}
            </p>
            <p className="text-[10px] text-primary/80 mt-2">→ {bp.targetApp}</p>
          </button>
        ))}
      </div>

      {previewLoading && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          {t("projectBlueprint.previewing", { defaultValue: "生成预览…" })}
        </div>
      )}

      {preview && !previewLoading && (
        <div className="rounded-lg border border-border/50 bg-background/60 p-3 space-y-2">
          <p className="text-xs font-medium">
            {t("projectBlueprint.previewTitle", {
              name: preview.blueprintName,
              count: preview.toLink.length,
              defaultValue: `预览：将新增 ${preview.toLink.length} 项关联`,
            })}
          </p>
          {preview.warnings.map((w) => (
            <p key={w} className="text-[11px] text-amber-700 dark:text-amber-400">
              {w}
            </p>
          ))}
          <ul className="max-h-32 overflow-y-auto text-[11px] text-muted-foreground space-y-0.5">
            {preview.toLink.slice(0, 12).map((item) => (
              <li key={`${item.assetType}-${item.assetId}-${item.appType ?? ""}`}>
                {item.assetType}: {item.assetId}
                {item.appType ? ` (${item.appType})` : ""}
              </li>
            ))}
            {preview.toLink.length > 12 && (
              <li>…+{preview.toLink.length - 12}</li>
            )}
          </ul>
          <Button size="sm" disabled={applying} onClick={() => void handleApply()}>
            {applying ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
            ) : (
              <Check className="h-3.5 w-3.5 mr-1" />
            )}
            {t("projectBlueprint.apply", { defaultValue: "应用 Blueprint" })}
          </Button>
        </div>
      )}
    </div>
  );
}
