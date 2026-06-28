import { useTranslation } from "react-i18next";
import { ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ResolvedQuickStartPreset } from "@/lib/quickStart/types";

interface QuickStartOfficialPanelProps {
  preset: ResolvedQuickStartPreset;
  onBack: () => void;
  onOpenSettings?: () => void;
}

export function QuickStartOfficialPanel({
  preset,
  onBack,
  onOpenSettings,
}: QuickStartOfficialPanelProps) {
  const { t } = useTranslation();
  const displayName = preset.nameKey
    ? String(t(preset.nameKey))
    : preset.name;

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-border bg-card p-4 space-y-3">
        <h3 className="font-semibold">{displayName}</h3>
        <p className="text-sm text-muted-foreground">
          {t("quickStart.officialDesc", {
            defaultValue:
              "官方供应商需通过浏览器登录或订阅授权，无法在快速接入中仅填 Key 完成。请前往供应商管理完成 OAuth / 官方配置。",
          })}
        </p>
        {preset.websiteUrl && (
          <a
            href={preset.websiteUrl}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 text-xs text-primary hover:underline"
          >
            {preset.websiteUrl}
            <ExternalLink className="h-3 w-3" />
          </a>
        )}
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={onBack}>
          {t("common.back", { defaultValue: "返回" })}
        </Button>
        {onOpenSettings && (
          <Button onClick={onOpenSettings}>
            {t("quickStart.openSettings", { defaultValue: "前往供应商管理" })}
          </Button>
        )}
      </div>
    </div>
  );
}
