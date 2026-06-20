import { CheckCircle2, KeyRound, Layers, Server } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { SC_INNER } from "./ui";

interface ConnectSummaryBarProps {
  supplierLabel: string;
  keyReady: boolean;
  keyHint?: string | null;
  poolEnabled: boolean;
  poolKeyCount: number;
  configuredCliCount?: number;
  totalCliCount?: number;
  currentStep?: number;
}

export function ConnectSummaryBar({
  supplierLabel,
  keyReady,
  keyHint,
  poolEnabled,
  poolKeyCount,
  configuredCliCount = 0,
  totalCliCount = 0,
  currentStep = 1,
}: ConnectSummaryBarProps) {
  const { t } = useTranslation();

  return (
    <div
      className={cn(
        SC_INNER,
        "flex flex-wrap items-center gap-2 px-4 py-3 text-sm",
      )}
    >
      <Badge variant="outline" className="font-normal tabular-nums">
        {t("simpleConnect.summaryStep", {
          step: currentStep,
          defaultValue: "步骤 {{step}}/3",
        })}
      </Badge>
      <div className="flex items-center gap-2 min-w-0">
        <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
        <span className="truncate font-medium">{supplierLabel}</span>
      </div>
      <Badge
        variant={keyReady ? "secondary" : "outline"}
        className={cn(
          "gap-1 font-normal",
          keyReady &&
            "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
        )}
      >
        <KeyRound className="h-3 w-3" />
        {keyReady
          ? keyHint ??
            t("simpleConnect.keySaved", { defaultValue: "Key 已保存" })
          : t("simpleConnect.keyMissing", { defaultValue: "未保存 Key" })}
      </Badge>
      {poolEnabled && (
        <Badge variant="outline" className="gap-1 font-normal">
          <Layers className="h-3 w-3" />
          {t("simpleConnect.poolBadge", {
            count: poolKeyCount,
            defaultValue: "密钥池 ×{{count}}",
          })}
        </Badge>
      )}
      {totalCliCount > 0 && currentStep >= 3 && (
        <Badge variant="outline" className="gap-1 font-normal">
          <CheckCircle2 className="h-3 w-3 text-emerald-500" />
          {t("simpleConnect.toolStatusCount", {
            configured: configuredCliCount,
            total: totalCliCount,
            defaultValue: "{{configured}}/{{total}} 已配置",
          })}
        </Badge>
      )}
    </div>
  );
}
