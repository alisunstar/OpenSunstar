import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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
        toast.warning("Bridge created with warnings", {
          description: result.warnings.join("; "),
        });
      } else {
        const label =
          TARGET_APPS.find((a) => a.id === targetApp)?.label || targetApp;
        toast.success(`Prompt bridged to ${label}`);
      }
      onBridged?.();
      onOpenChange(false);
    } catch (e) {
      toast.error(`Bridge failed: ${e}`);
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
            Bridge Prompt
          </DialogTitle>
          <DialogDescription>
            Bridge &ldquo;{promptName}&rdquo; to another AI tool. Content will be
            automatically transformed.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4 px-6">
          <div className="space-y-2">
            <label className="text-sm font-medium">Target Tool</label>
            <Select value={targetApp} onValueChange={setTargetApp}>
              <SelectTrigger>
                <SelectValue placeholder="Select target tool..." />
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
              Generating preview...
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
                  Preview (first 500 chars)
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
              <p className="text-sm font-medium">Auto-sync on edit</p>
              <p className="text-xs text-muted-foreground">
                Automatically push changes to bridged targets when source is saved
              </p>
            </div>
            <Switch
              checked={autoPush}
              onCheckedChange={(checked) => {
                setAutoPush(checked);
                invoke("set_bridge_auto_push", { enabled: checked }).catch((e) =>
                  toast.error(`Failed to save setting: ${e}`)
                );
              }}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleBridge} disabled={!targetApp || loading}>
            {loading ? (
              "Bridging..."
            ) : (
              <>
                <Link2 className="w-4 h-4 mr-1" />
                Create Bridge
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
