import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useDryRunMode } from "@/hooks/useDryRunMode";

export function DryRunSettings() {
  const { t } = useTranslation();
  const { enabled, loading, setMode } = useDryRunMode();

  return (
    <div className="flex items-center justify-between gap-4">
      <div className="space-y-1">
        <Label>{t("dryRun.title", { defaultValue: "预览模式（Dry Run）" })}</Label>
        <p className="text-sm text-muted-foreground">
          {t("dryRun.description", {
            defaultValue:
              "开启后，Prompt 激活等写入操作将先显示 Diff 预览，确认后才实际写入",
          })}
        </p>
      </div>
      <Switch
        checked={enabled}
        disabled={loading}
        onCheckedChange={(v) => void setMode(v)}
      />
    </div>
  );
}
