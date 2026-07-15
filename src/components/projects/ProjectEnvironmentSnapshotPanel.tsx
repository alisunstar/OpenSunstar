import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Camera,
  CheckCircle2,
  ChevronDown,
  GitCompare,
  Loader2,
  Play,
  RotateCcw,
  Trash2,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  projectsApi,
  type ProjectEnvironmentApplyPreview,
  type ProjectEnvironmentApplyReceipt,
  type ProjectEnvironmentDimension,
  type ProjectEnvironmentDiff,
  type ProjectEnvironmentSnapshot,
} from "@/lib/api/projects";

interface ProjectEnvironmentSnapshotPanelProps {
  projectId: string;
  onApplied?: () => void;
}

const DIMENSION_OPTIONS: Array<{
  value: ProjectEnvironmentDimension;
  label: string;
  description: string;
}> = [
  {
    value: "provider",
    label: "AI 服务",
    description: "当前供应商与模型路由",
  },
  {
    value: "mcp",
    label: "工具连接",
    description: "MCP 服务启用状态",
  },
  {
    value: "skills",
    label: "智能体能力",
    description: "Skills 启用状态",
  },
  {
    value: "prompt",
    label: "项目指令",
    description: "Prompt 与项目约定",
  },
];

const ALL_DIMENSIONS = DIMENSION_OPTIONS.map((option) => option.value);

function dimensionLabels(dimensions: ProjectEnvironmentDimension[]) {
  const selected = new Set(dimensions);
  return DIMENSION_OPTIONS.filter((option) => selected.has(option.value)).map(
    (option) => option.label,
  );
}

function dimensionLabel(dimension: string) {
  return (
    DIMENSION_OPTIONS.find((option) => option.value === dimension)?.label ??
    dimension
  );
}

function formatUnixTime(value?: number | null) {
  if (!value) return "—";
  return new Date(value * 1000).toLocaleString();
}

function formatValues(values: string[]) {
  return values.length > 0 ? values.join(", ") : "空";
}

function diffLine(diff: ProjectEnvironmentDiff) {
  return `${diff.app} / ${dimensionLabel(diff.dimension)}: ${formatValues(diff.before)} -> ${formatValues(diff.after)}`;
}

function verificationSummary(receipt: ProjectEnvironmentApplyReceipt) {
  const passed = receipt.verifications.filter((item) => item.passed).length;
  return `${passed}/${receipt.verifications.length}`;
}

export function ProjectEnvironmentSnapshotPanel({
  projectId,
  onApplied,
}: ProjectEnvironmentSnapshotPanelProps) {
  const { t } = useTranslation();
  const [snapshots, setSnapshots] = useState<ProjectEnvironmentSnapshot[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [scopeOpen, setScopeOpen] = useState(false);
  const [includedDimensions, setIncludedDimensions] = useState<
    ProjectEnvironmentDimension[]
  >([...ALL_DIMENSIONS]);
  const [preview, setPreview] = useState<ProjectEnvironmentApplyPreview | null>(
    null,
  );
  const [lastReceipt, setLastReceipt] =
    useState<ProjectEnvironmentApplyReceipt | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setSnapshots(await projectsApi.listEnvironmentSnapshots(projectId));
    } catch (error) {
      toast.error(String(error));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const defaultName = useMemo(() => {
    const hour = new Date().getHours();
    return hour < 12 ? "上午工作环境" : "项目工作环境";
  }, []);

  const handleCreate = async () => {
    if (includedDimensions.length === 0) {
      toast.error("至少选择一项快照内容");
      return;
    }
    const snapshotName = name.trim() || defaultName;
    setCreating(true);
    try {
      await projectsApi.createEnvironmentSnapshot(
        projectId,
        snapshotName,
        includedDimensions,
      );
      setName("");
      setIncludedDimensions([...ALL_DIMENSIONS]);
      setScopeOpen(false);
      await load();
      toast.success("项目环境快照已创建");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setCreating(false);
    }
  };

  const toggleDimension = (
    dimension: ProjectEnvironmentDimension,
    checked: boolean,
  ) => {
    setIncludedDimensions((current) =>
      checked
        ? ALL_DIMENSIONS.filter(
            (candidate) =>
              candidate === dimension || current.includes(candidate),
          )
        : current.filter((candidate) => candidate !== dimension),
    );
  };

  const handlePreview = async (snapshotId: string) => {
    setBusyId(snapshotId);
    try {
      setPreview(await projectsApi.previewEnvironmentSnapshotApply(snapshotId));
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusyId(null);
    }
  };

  const handleApply = async () => {
    if (!preview) return;
    const snapshotId = preview.snapshot.id;
    setBusyId(snapshotId);
    try {
      const receipt = await projectsApi.applyEnvironmentSnapshot(snapshotId);
      setLastReceipt(receipt);
      setPreview(null);
      await load();
      onApplied?.();
      toast.success(
        `项目环境已应用，写后验证 ${verificationSummary(receipt)} 通过`,
      );
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusyId(null);
    }
  };

  const handleRollback = async (snapshotId: string) => {
    if (!window.confirm("确认回滚到最近一次应用前的环境状态？")) return;
    setBusyId(snapshotId);
    try {
      const receipt = await projectsApi.rollbackEnvironmentSnapshot(snapshotId);
      setLastReceipt(receipt);
      await load();
      onApplied?.();
      toast.success(
        `项目环境已回滚，写后验证 ${verificationSummary(receipt)} 通过`,
      );
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusyId(null);
    }
  };

  const handleDelete = async (snapshotId: string) => {
    if (!window.confirm("确认删除这个项目环境快照？")) return;
    setBusyId(snapshotId);
    try {
      await projectsApi.deleteEnvironmentSnapshot(snapshotId);
      await load();
      toast.success("项目环境快照已删除");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusyId(null);
    }
  };

  return (
    <section className="rounded-lg border border-border/60 bg-card/50 p-3 space-y-3">
      <div className="flex items-start gap-2">
        <Camera className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-medium">
            {t("projectEnvironment.title", {
              defaultValue: "项目环境快照",
            })}
          </h3>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            {t("projectEnvironment.hint", {
              defaultValue:
                "保存当前供应商、MCP、Skills 与 Prompt 激活状态；应用前预览差异，应用后写入验证回执并可回滚。",
            })}
          </p>
        </div>
      </div>

      <div className="flex gap-2">
        <Input
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder={defaultName}
          className="h-8 text-xs"
          onKeyDown={(event) => {
            if (event.key === "Enter") void handleCreate();
          }}
        />
        <Button
          size="sm"
          className="h-8 shrink-0"
          disabled={creating || includedDimensions.length === 0}
          onClick={() => void handleCreate()}
        >
          {creating ? (
            <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />
          ) : (
            <Camera className="mr-1 h-3.5 w-3.5" />
          )}
          创建
        </Button>
      </div>

      <Collapsible open={scopeOpen} onOpenChange={setScopeOpen}>
        <CollapsibleTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 w-full justify-between px-2 text-xs font-normal text-muted-foreground"
          >
            <span>
              快照包含内容：
              {includedDimensions.length === ALL_DIMENSIONS.length
                ? "全部 4 项"
                : `${includedDimensions.length} 项`}
            </span>
            <span className="flex items-center gap-1 text-primary">
              自定义
              <ChevronDown
                className={`h-3.5 w-3.5 transition-transform ${scopeOpen ? "rotate-180" : ""}`}
              />
            </span>
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="pt-2">
          <div className="grid gap-2 rounded-md border border-border/50 bg-muted/20 p-2 sm:grid-cols-2">
            {DIMENSION_OPTIONS.map((option) => {
              const checked = includedDimensions.includes(option.value);
              return (
                <label
                  key={option.value}
                  className="flex cursor-pointer items-start gap-2 rounded px-2 py-1.5 hover:bg-muted/50"
                >
                  <Checkbox
                    checked={checked}
                    onCheckedChange={(value) =>
                      toggleDimension(option.value, value === true)
                    }
                    aria-label={option.label}
                  />
                  <span className="min-w-0">
                    <span className="block text-xs font-medium">
                      {option.label}
                    </span>
                    <span className="block text-[11px] text-muted-foreground">
                      {option.description}
                    </span>
                  </span>
                </label>
              );
            })}
          </div>
          {includedDimensions.length === 0 && (
            <p className="mt-1.5 text-[11px] text-destructive">
              至少选择一项内容；未选择的维度不会被保存、应用、验证或回滚。
            </p>
          )}
        </CollapsibleContent>
      </Collapsible>

      {loading ? (
        <div className="flex items-center gap-2 py-2 text-xs text-muted-foreground">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          加载环境快照...
        </div>
      ) : snapshots.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          暂无项目环境快照。先把当前工具环境保存下来，再用于一键恢复。
        </p>
      ) : (
        <div className="space-y-2">
          {snapshots.map((snapshot) => {
            const appCount = Object.keys(snapshot.payload.apps ?? {}).length;
            const scopeLabels = dimensionLabels(
              snapshot.payload.includedDimensions ?? ALL_DIMENSIONS,
            );
            const busy = busyId === snapshot.id;
            return (
              <div
                key={snapshot.id}
                className="rounded-md border border-border/50 bg-background/60 px-3 py-2"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">
                      {snapshot.name}
                    </p>
                    <p className="mt-0.5 text-[11px] text-muted-foreground">
                      {appCount} 个 CLI · 更新{" "}
                      {formatUnixTime(snapshot.updatedAt)}
                      {snapshot.lastAppliedAt
                        ? ` · 最近应用 ${formatUnixTime(snapshot.lastAppliedAt)}`
                        : ""}
                    </p>
                    <p className="mt-0.5 truncate text-[11px] text-muted-foreground">
                      包含：{scopeLabels.join("、")}
                    </p>
                  </div>
                  {busy && <Loader2 className="h-4 w-4 animate-spin" />}
                </div>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  <Button
                    size="sm"
                    variant="outline"
                    className="h-7 px-2 text-xs"
                    disabled={busy}
                    onClick={() => void handlePreview(snapshot.id)}
                  >
                    <GitCompare className="mr-1 h-3.5 w-3.5" />
                    预览并应用
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2 text-xs"
                    disabled={busy || !snapshot.lastAppliedAt}
                    onClick={() => void handleRollback(snapshot.id)}
                  >
                    <RotateCcw className="mr-1 h-3.5 w-3.5" />
                    回滚
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2 text-xs text-destructive"
                    disabled={busy}
                    onClick={() => void handleDelete(snapshot.id)}
                  >
                    <Trash2 className="mr-1 h-3.5 w-3.5" />
                    删除
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {lastReceipt && (
        <div className="rounded-md border border-border/50 bg-muted/20 px-3 py-2 text-xs">
          <div className="flex items-center gap-1.5 font-medium">
            {lastReceipt.verifications.every((item) => item.passed) ? (
              <CheckCircle2 className="h-3.5 w-3.5 text-emerald-500" />
            ) : (
              <XCircle className="h-3.5 w-3.5 text-amber-500" />
            )}
            写后验证 {verificationSummary(lastReceipt)} 通过
          </div>
          {lastReceipt.warnings.length > 0 && (
            <p className="mt-1 text-muted-foreground">
              {lastReceipt.warnings.join("；")}
            </p>
          )}
        </div>
      )}

      <Dialog
        open={preview !== null}
        onOpenChange={(open) => {
          if (!open) setPreview(null);
        }}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>应用前差异预览</DialogTitle>
            <DialogDescription>
              {preview?.snapshot.name} 将按以下差异应用到当前工具环境。
            </DialogDescription>
          </DialogHeader>
          {preview && (
            <div className="rounded-md border border-border/50 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
              <p>
                本次管理：
                {dimensionLabels(
                  preview.snapshot.payload.includedDimensions ?? ALL_DIMENSIONS,
                ).join("、")}
              </p>
              {preview.snapshot.payload.includedDimensions.length <
                ALL_DIMENSIONS.length && (
                <p className="mt-1">
                  保持不变：
                  {dimensionLabels(
                    ALL_DIMENSIONS.filter(
                      (dimension) =>
                        !preview.snapshot.payload.includedDimensions.includes(
                          dimension,
                        ),
                    ),
                  ).join("、")}
                </p>
              )}
            </div>
          )}
          <div className="max-h-[45vh] overflow-auto rounded-md border border-border/60 bg-muted/20 p-3">
            {preview?.diff.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                当前环境已与快照一致。
              </p>
            ) : (
              <div className="space-y-2">
                {preview?.diff.map((item, index) => (
                  <div
                    key={`${item.app}-${item.dimension}-${index}`}
                    className="text-xs"
                  >
                    <p className="font-medium">
                      {item.app} / {dimensionLabel(item.dimension)}
                    </p>
                    <p className="mt-0.5 text-muted-foreground">
                      {formatValues(item.before)} → {formatValues(item.after)}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </div>
          {preview && preview.diff.length > 0 && (
            <pre className="max-h-24 overflow-auto rounded-md bg-background/70 p-2 text-[11px] text-muted-foreground">
              {preview.diff.map(diffLine).join("\n")}
            </pre>
          )}
          <DialogFooter className="gap-2 sm:justify-end">
            <Button variant="outline" onClick={() => setPreview(null)}>
              取消
            </Button>
            <Button
              onClick={() => void handleApply()}
              disabled={busyId !== null}
            >
              <Play className="mr-1 h-3.5 w-3.5" />
              确认应用
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
