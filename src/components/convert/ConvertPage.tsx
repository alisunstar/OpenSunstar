import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  ArrowRight,
  ArrowRightLeft,
  Check,
  AlertTriangle,
  FileText,
  Server,
  Wrench,
  Terminal,
  Bot,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import {
  applyConvert,
  detectConvertSources,
  previewConvert,
  type ConvertSourceItem,
} from "@/lib/api/convert";
import type { BridgePreview } from "@/lib/api/bridge";

const CONVERT_APPS = [
  { id: "claude", label: "Claude Code" },
  { id: "codex", label: "Codex" },
  { id: "gemini", label: "Gemini CLI" },
  { id: "opencode", label: "OpenCode" },
  { id: "hermes", label: "Hermes" },
] as const;

const STEPS = ["source", "previewSource", "previewTarget", "confirm"] as const;
type Step = (typeof STEPS)[number];

function DiffPreview({
  left,
  right,
  leftLabel,
  rightLabel,
}: {
  left: string;
  right: string;
  leftLabel: string;
  rightLabel: string;
}) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-3 min-h-[280px]">
      <div className="flex flex-col min-h-0">
        <div className="text-xs font-medium text-muted-foreground mb-1.5 px-1">
          {leftLabel}
        </div>
        <pre className="flex-1 overflow-auto rounded-lg border border-border/60 bg-muted/30 p-3 text-xs leading-relaxed whitespace-pre-wrap font-mono">
          {left || "—"}
        </pre>
      </div>
      <div className="flex flex-col min-h-0">
        <div className="text-xs font-medium text-muted-foreground mb-1.5 px-1">
          {rightLabel}
        </div>
        <pre className="flex-1 overflow-auto rounded-lg border border-blue-500/30 bg-blue-500/5 p-3 text-xs leading-relaxed whitespace-pre-wrap font-mono">
          {right || "—"}
        </pre>
      </div>
    </div>
  );
}

export function ConvertPage() {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("source");
  const [sourceApp, setSourceApp] = useState("claude");
  const [targetApp, setTargetApp] = useState("");
  const [sources, setSources] = useState<ConvertSourceItem[]>([]);
  const [loadingSources, setLoadingSources] = useState(false);
  const [selectedType, setSelectedType] = useState<string>("prompt");
  const [preview, setPreview] = useState<BridgePreview | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [overwrite, setOverwrite] = useState(false);
  const [applying, setApplying] = useState(false);

  const stepIndex = STEPS.indexOf(step);

  const selectedSource = useMemo(
    () => sources.find((s) => s.contentType === selectedType),
    [sources, selectedType],
  );

  const sourceContent = selectedSource?.content ?? "";

  const targetOptions = useMemo(
    () => CONVERT_APPS.filter((a) => a.id !== sourceApp),
    [sourceApp],
  );

  const loadSources = useCallback(async (app: string) => {
    setLoadingSources(true);
    try {
      const items = await detectConvertSources(app);
      setSources(items);
      const first = items.find((i) => i.exists) ?? items[0];
      if (first) setSelectedType(first.contentType);
    } catch (e) {
      toast.error(
        t("convert.loadFailed", {
          defaultValue: "检测源配置失败",
        }),
      );
      setSources([]);
    } finally {
      setLoadingSources(false);
    }
  }, [t]);

  useEffect(() => {
    void loadSources(sourceApp);
  }, [sourceApp, loadSources]);

  useEffect(() => {
    if (!targetApp || !sourceContent) {
      setPreview(null);
      return;
    }
    let cancelled = false;
    setPreviewing(true);
    previewConvert(sourceApp, targetApp, sourceContent, selectedType)
      .then((p) => {
        if (!cancelled) setPreview(p);
      })
      .catch(() => {
        if (!cancelled) setPreview(null);
      })
      .finally(() => {
        if (!cancelled) setPreviewing(false);
      });
    return () => {
      cancelled = true;
    };
  }, [sourceApp, targetApp, sourceContent, selectedType]);

  const canNextFromSource =
    !loadingSources && selectedSource?.exists && !!selectedSource.content;

  const handleApply = async () => {
    if (!preview || !targetApp) return;
    setApplying(true);
    try {
      const result = await applyConvert({
        sourceApp,
        targetApp,
        contentType: selectedType,
        content: sourceContent,
        overwrite,
      });
      toast.success(
        t("convert.applySuccess", { defaultValue: "配置已写入目标工具" }),
      );
      if (result.warnings.length > 0) {
        result.warnings.forEach((w) => toast.warning(w));
      }
      setStep("source");
      setTargetApp("");
      setPreview(null);
      void loadSources(sourceApp);
    } catch (e) {
      toast.error(
        String(e) ||
          t("convert.applyFailed", { defaultValue: "写入失败，已尝试保留备份" }),
      );
    } finally {
      setApplying(false);
    }
  };

  return (
    <div className="flex flex-col flex-1 min-h-0 px-6 pb-8">
      <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6">
        <div className="flex items-center gap-3 mb-3">
          <div className="w-10 h-10 rounded-lg bg-violet-500/10 flex items-center justify-center">
            <ArrowRightLeft className="w-5 h-5 text-violet-500" />
          </div>
          <div>
            <h2 className="text-base font-semibold">
              {t("convert.title", { defaultValue: "配置转换向导" })}
            </h2>
            <p className="text-sm text-muted-foreground">
              {t("convert.subtitle", {
                defaultValue:
                  "在 Claude / Codex / Gemini 等工具间迁移 Prompt、MCP、Skill 与 Command 配置",
              })}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs">
          {STEPS.map((s, i) => (
            <div key={s} className="flex items-center gap-2">
              <span
                className={cn(
                  "flex items-center justify-center w-6 h-6 rounded-full border text-[11px] font-medium",
                  i <= stepIndex
                    ? "bg-violet-500 text-white border-violet-500"
                    : "border-border text-muted-foreground",
                )}
              >
                {i < stepIndex ? <Check className="w-3.5 h-3.5" /> : i + 1}
              </span>
              <span
                className={cn(
                  i === stepIndex
                    ? "text-foreground font-medium"
                    : "text-muted-foreground",
                )}
              >
                {t(`convert.steps.${s}`, {
                  defaultValue: [
                    "选择源",
                    "预览源",
                    "预览转换",
                    "确认写入",
                  ][i],
                })}
              </span>
              {i < STEPS.length - 1 && (
                <ArrowRight className="w-3.5 h-3.5 text-muted-foreground/50 mx-1" />
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {step === "source" && (
          <div className="space-y-5 max-w-xl">
            <div>
              <label className="text-sm font-medium mb-2 block">
                {t("convert.sourceApp", { defaultValue: "源工具" })}
              </label>
              <Select value={sourceApp} onValueChange={setSourceApp}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CONVERT_APPS.map((app) => (
                    <SelectItem key={app.id} value={app.id}>
                      {app.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div>
              <label className="text-sm font-medium mb-2 block">
                {t("convert.contentType", { defaultValue: "配置类型" })}
              </label>
              {loadingSources ? (
                <div className="text-sm text-muted-foreground py-4">
                  {t("common.loading")}
                </div>
              ) : (
                <div className="space-y-2">
                  {sources.map((item) => (
                    <button
                      key={item.contentType}
                      type="button"
                      disabled={!item.exists}
                      onClick={() => setSelectedType(item.contentType)}
                      className={cn(
                        "w-full flex items-start gap-3 p-3 rounded-lg border text-left transition-colors",
                        selectedType === item.contentType
                          ? "border-violet-500/50 bg-violet-500/5"
                          : "border-border/60 hover:bg-muted/30",
                        !item.exists && "opacity-50 cursor-not-allowed",
                      )}
                    >
                      {item.contentType === "mcp" ? (
                        <Server className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                      ) : item.contentType === "skill" ? (
                        <Wrench className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                      ) : item.contentType === "command" ? (
                        <Terminal className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                      ) : item.contentType === "agent" ? (
                        <Bot className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                      ) : (
                        <FileText className="w-4 h-4 mt-0.5 shrink-0 text-muted-foreground" />
                      )}
                      <div className="min-w-0 flex-1">
                        <div className="text-sm font-medium">{item.label}</div>
                        <div className="text-xs text-muted-foreground truncate">
                          {item.path}
                        </div>
                        {!item.exists && (
                          <div className="text-xs text-amber-600 dark:text-amber-400 mt-1">
                            {t("convert.notFound", { defaultValue: "文件不存在" })}
                          </div>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {step === "previewSource" && selectedSource && (
          <div className="space-y-3">
            <div className="text-sm text-muted-foreground">
              {selectedSource.path}
            </div>
            <pre className="rounded-lg border border-border/60 bg-muted/30 p-4 text-xs leading-relaxed whitespace-pre-wrap font-mono max-h-[480px] overflow-auto">
              {sourceContent}
            </pre>
          </div>
        )}

        {step === "previewTarget" && (
          <div className="space-y-4">
            <div className="max-w-xs">
              <label className="text-sm font-medium mb-2 block">
                {t("convert.targetApp", { defaultValue: "目标工具" })}
              </label>
              <Select value={targetApp} onValueChange={setTargetApp}>
                <SelectTrigger>
                  <SelectValue
                    placeholder={t("convert.selectTarget", {
                      defaultValue: "选择目标工具",
                    })}
                  />
                </SelectTrigger>
                <SelectContent>
                  {targetOptions.map((app) => (
                    <SelectItem key={app.id} value={app.id}>
                      {app.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {previewing && (
              <div className="text-sm text-muted-foreground">
                {t("convert.previewing", { defaultValue: "生成预览…" })}
              </div>
            )}

            {preview && targetApp && (
              <>
                {(preview.unmappedSections.length > 0 ||
                  preview.warnings.length > 0) && (
                  <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 space-y-1">
                    <div className="flex items-center gap-2 text-sm font-medium text-amber-700 dark:text-amber-300">
                      <AlertTriangle className="w-4 h-4" />
                      {t("convert.warnings", { defaultValue: "转换提示" })}
                    </div>
                    {preview.warnings.map((w) => (
                      <p key={w} className="text-xs text-muted-foreground">
                        {w}
                      </p>
                    ))}
                    {preview.unmappedSections.map((s) => (
                      <p key={s} className="text-xs text-muted-foreground">
                        {t("convert.unmapped", {
                          defaultValue: "未映射章节: {{section}}",
                          section: s,
                        })}
                      </p>
                    ))}
                  </div>
                )}
                <DiffPreview
                  left={sourceContent}
                  right={preview.convertedContent}
                  leftLabel={t("convert.sourcePreview", {
                    defaultValue: "源内容",
                  })}
                  rightLabel={t("convert.convertedPreview", {
                    defaultValue: "转换后",
                  })}
                />
              </>
            )}
          </div>
        )}

        {step === "confirm" && preview && targetApp && (
          <div className="space-y-4 max-w-2xl">
            <div className="rounded-lg border border-border/60 p-4 space-y-2 text-sm">
              <div className="flex justify-between gap-4">
                <span className="text-muted-foreground">
                  {t("convert.sourceApp", { defaultValue: "源工具" })}
                </span>
                <span className="font-medium">
                  {CONVERT_APPS.find((a) => a.id === sourceApp)?.label}
                </span>
              </div>
              <div className="flex justify-between gap-4">
                <span className="text-muted-foreground">
                  {t("convert.targetApp", { defaultValue: "目标工具" })}
                </span>
                <span className="font-medium">
                  {CONVERT_APPS.find((a) => a.id === targetApp)?.label}
                </span>
              </div>
              <div className="flex justify-between gap-4">
                <span className="text-muted-foreground">
                  {t("convert.contentType", { defaultValue: "配置类型" })}
                </span>
                <span className="font-medium">{selectedSource?.label}</span>
              </div>
            </div>

            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <Checkbox
                checked={overwrite}
                onCheckedChange={(v) => setOverwrite(v === true)}
              />
              {t("convert.overwrite", {
                defaultValue: "覆盖已存在的目标文件（写入前会自动备份 .bak.opensunstar）",
              })}
            </label>

            <pre className="rounded-lg border border-border/60 bg-muted/20 p-3 text-xs max-h-48 overflow-auto whitespace-pre-wrap font-mono">
              {preview.convertedContent.slice(0, 2000)}
              {preview.convertedContent.length > 2000 ? "\n…" : ""}
            </pre>
          </div>
        )}
      </div>

      <div className="flex-shrink-0 flex justify-between pt-4 mt-4 border-t border-border/40">
        <Button
          variant="ghost"
          disabled={stepIndex === 0 || applying}
          onClick={() => setStep(STEPS[stepIndex - 1])}
        >
          {t("common.back", { defaultValue: "上一步" })}
        </Button>
        <div className="flex gap-2">
          {step !== "confirm" ? (
            <Button
              disabled={
                (step === "source" && !canNextFromSource) ||
                (step === "previewTarget" && (!targetApp || !preview || previewing))
              }
              onClick={() => setStep(STEPS[stepIndex + 1])}
            >
              {t("common.next", { defaultValue: "下一步" })}
            </Button>
          ) : (
            <Button onClick={() => void handleApply()} disabled={applying}>
              {applying
                ? t("common.loading")
                : t("convert.apply", { defaultValue: "确认写入" })}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
