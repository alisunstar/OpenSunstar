import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { createUsageScript, type Provider, type UsageScript } from "@/types";

interface QuickStartUsageDialogProps {
  provider: Provider | null;
  open: boolean;
  isSaving?: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (script: UsageScript) => Promise<void>;
}

export function QuickStartUsageDialog({
  provider,
  open,
  isSaving = false,
  onOpenChange,
  onSave,
}: QuickStartUsageDialogProps) {
  const { t } = useTranslation();
  const [enabled, setEnabled] = useState(false);
  const [code, setCode] = useState("");
  const [autoQueryInterval, setAutoQueryInterval] = useState("5");

  useEffect(() => {
    if (!open) return;
    const script = createUsageScript(provider?.meta?.usage_script);
    setEnabled(script.enabled);
    setCode(script.code);
    setAutoQueryInterval(String(script.autoQueryInterval ?? 5));
  }, [open, provider]);

  const handleSave = async () => {
    if (!provider) return;

    await onSave(
      createUsageScript({
        ...provider.meta?.usage_script,
        enabled,
        code,
        autoQueryInterval: Math.max(0, Number(autoQueryInterval) || 0),
      }),
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent zIndex="nested" className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("provider.configureUsage", { defaultValue: "配置用量查询" })}
          </DialogTitle>
          <DialogDescription>
            {t("quickStart.usageConfigDescription", {
              defaultValue:
                "为 {{name}} 保存查询脚本。默认复用该供应商已安全保存的连接凭据，不在此处重复录入密钥。",
              name: provider?.name ?? "",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 px-6 py-5">
          <label className="flex items-center gap-2 text-sm font-medium">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(event) => setEnabled(event.target.checked)}
            />
            {t("quickStart.enableUsage", { defaultValue: "启用用量自动查询" })}
          </label>

          <div className="space-y-2">
            <label
              htmlFor="quick-start-usage-code"
              className="text-sm font-medium"
            >
              {t("quickStart.usageScript", { defaultValue: "查询脚本" })}
            </label>
            <Textarea
              id="quick-start-usage-code"
              value={code}
              onChange={(event) => setCode(event.target.value)}
              className="min-h-52 font-mono text-xs"
              placeholder="return { success: true, data: [] };"
            />
          </div>

          <div className="max-w-48 space-y-2">
            <label
              htmlFor="quick-start-usage-interval"
              className="text-sm font-medium"
            >
              {t("quickStart.usageInterval", {
                defaultValue: "自动查询间隔（分钟，0 为关闭）",
              })}
            </label>
            <Input
              id="quick-start-usage-interval"
              type="number"
              min="0"
              value={autoQueryInterval}
              onChange={(event) => setAutoQueryInterval(event.target.value)}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button
            onClick={() => void handleSave()}
            disabled={isSaving || (enabled && !code.trim())}
          >
            {t("quickStart.saveUsage", { defaultValue: "保存用量配置" })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
