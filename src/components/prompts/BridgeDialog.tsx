import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ArrowRightLeft, Link2, AlertTriangle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  bridgePrompt,
  previewBridge,
  type BridgePreview,
} from "@/lib/api/bridge";

interface BridgeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  promptId: string;
  promptName: string;
  sourceApp: string;
  content: string;
  onBridged?: () => void;
}

const TARGET_APPS = [
  { id: "claude", label: "Claude Code" },
  { id: "codex", label: "Codex" },
  { id: "gemini", label: "Gemini CLI" },
  { id: "opencode", label: "OpenCode" },
  { id: "hermes", label: "Hermes" },
  { id: "openclaw", label: "OpenClaw" },
];

export function BridgeDialog({
  open,
  onOpenChange,
  promptId,
  promptName,
  sourceApp,
  content,
  onBridged,
}: BridgeDialogProps) {
  const { t } = useTranslation();
  const [targetApp, setTargetApp] = useState("");
  const [preview, setPreview] = useState<BridgePreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [autoPush, setAutoPush] = useState(false);

  const availableTargets = TARGET_APPS.filter((app) => app.id !== sourceApp);

  useEffect(() => {
    if (open) {
      invoke<boolean>("get_bridge_auto_push").then(setAutoPush).catch(() => {});
    } else {
      setTargetApp("");
      setPreview(null);
    }
  }, [open]);

  useEffect(() => {
    if (!targetApp || !content) {
      setPreview(null);
      return;
    }
    let cancelled = false;
    setPreviewing(true);
    previewBridge(sourceApp, targetApp, content)
      .then((result) => {
        if (!cancelled) setPreview(result);
      })
      .catch((e) => console.error("Preview failed:", e))
      .finally(() => {
        if (!cancelled) setPreviewing(false);
      });
    return () => {
      cancelled = true;
    };
  }, [targetApp, sourceApp, content]);

  const handleBridge = async () => {
    if (!targetApp) return;
    setLoading(true);
    try {
      const result = (await bridgePrompt(sourceApp, targetApp, promptId)) as {
        id: string;
        app_type: string;
        warnings: string[];
      };
      if (result.warnings.length > 0) {
        toast.warning(
          t("bridge.createdWithWarnings", {
            defaultValue: "桥接已创建，但有警告",
          }),
          {
            description: result.warnings.join("; "),
          },
        );
      } else {
        const label =
          TARGET_APPS.find((a) => a.id === targetApp)?.label || targetApp;
        toast.success(
          t("bridge.createdSuccess", {
            label,
            defaultValue: "Prompt 已桥接到 {{label}}",
          }),
        );
      }
      onBridged?.();
      onOpenChange(false);
    } catch (e) {
      toast.error(
        t("bridge.failed", {
          error: String(e),
          defaultValue: "桥接失败：{{error}}",
        }),
      );
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[520px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ArrowRightLeft className="w-5 h-5" />
            {t("bridge.title", { defaultValue: "桥接 Prompt" })}
          </DialogTitle>
          <DialogDescription>
            {t("bridge.description", {
              name: promptName,
              defaultValue:
                "将「{{name}}」桥接到其他 AI 工具，内容将自动转换格式。",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4 px-6">
          <div className="space-y-2">
            <label className="text-sm font-medium">
              {t("bridge.targetTool", { defaultValue: "目标工具" })}
            </label>
            <Select value={targetApp} onValueChange={setTargetApp}>
              <SelectTrigger>
                <SelectValue
                  placeholder={t("bridge.selectTarget", {
                    defaultValue: "选择目标工具...",
                  })}
                />
              </SelectTrigger>
              <SelectContent>
                {availableTargets.map((app) => (
                  <SelectItem key={app.id} value={app.id}>
                    {app.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {previewing && (
            <p className="text-sm text-muted-foreground animate-pulse">
              {t("bridge.generatingPreview", {
                defaultValue: "正在生成预览...",
              })}
            </p>
          )}

          {preview && (
            <div className="space-y-3">
              {preview.warnings.length > 0 && (
                <div className="rounded-lg border border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-950/30 p-3 space-y-1">
                  {preview.warnings.map((w, i) => (
                    <p
                      key={i}
                      className="text-xs text-amber-700 dark:text-amber-300 flex gap-2"
                    >
                      <AlertTriangle className="w-3 h-3 shrink-0 mt-0.5" />
                      {w}
                    </p>
                  ))}
                </div>
              )}

              <div className="space-y-1">
                <p className="text-xs font-medium text-muted-foreground">
                  {t("bridge.previewLabel", {
                    defaultValue: "预览（前 500 字符）",
                  })}
                </p>
                <pre className="text-xs bg-muted/50 rounded-md p-3 max-h-[200px] overflow-auto whitespace-pre-wrap break-words border">
                  {preview.convertedContent.slice(0, 500)}
                  {preview.convertedContent.length > 500 && "..."}
                </pre>
              </div>
            </div>
          )}
        </div>

        <div className="px-6 pb-2">
          <div className="flex items-center justify-between rounded-lg border p-3">
            <div className="space-y-0.5">
              <p className="text-sm font-medium">
                {t("bridge.autoSyncOnEdit", { defaultValue: "编辑时自动同步" })}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("bridge.autoSyncDescription", {
                  defaultValue: "保存源 Prompt 时自动推送变更到已桥接的目标",
                })}
              </p>
            </div>
            <Switch
              checked={autoPush}
              onCheckedChange={(checked) => {
                setAutoPush(checked);
                invoke("set_bridge_auto_push", { enabled: checked }).catch(
                  (e) =>
                    toast.error(
                      t("bridge.saveSettingFailed", {
                        error: String(e),
                        defaultValue: "保存设置失败：{{error}}",
                      }),
                    ),
                );
              }}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button onClick={handleBridge} disabled={!targetApp || loading}>
            {loading ? (
              t("bridge.bridging", { defaultValue: "桥接中..." })
            ) : (
              <>
                <Link2 className="w-4 h-4 mr-1" />
                {t("bridge.createBridge", { defaultValue: "创建桥接" })}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
