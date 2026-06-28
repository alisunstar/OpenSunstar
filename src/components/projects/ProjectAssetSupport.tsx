import { useTranslation } from "react-i18next";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import {
  ASSET_APP_SUPPORT,
  isAssetLinkable,
  type AssetAppSupport,
} from "@/lib/projectAssets/assetAppSupport";
import type { AppId } from "@/lib/api";
import type { ProjectAssetType } from "@/types/projectAsset";

const SUPPORT_APPS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

interface ProjectAssetAppSupportChipsProps {
  assetType: ProjectAssetType;
  className?: string;
}

/** 展示该资产类型在各 CLI 上的同步能力（supported / partial / unsupported） */
export function ProjectAssetAppSupportChips({
  assetType,
  className,
}: ProjectAssetAppSupportChipsProps) {
  const { t } = useTranslation();

  return (
    <div className={cn("flex flex-wrap gap-1", className)}>
      {SUPPORT_APPS.map((appId) => {
        const support = ASSET_APP_SUPPORT[assetType][appId];
        const label =
          appId === "claude"
            ? "Claude"
            : appId === "codex"
              ? "Codex"
              : appId === "gemini"
                ? "Gemini"
                : appId === "opencode"
                  ? "OpenCode"
                  : "Hermes";
        const tone =
          support.status === "supported"
            ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border-emerald-500/20"
            : support.status === "partial"
              ? "bg-amber-500/10 text-amber-700 dark:text-amber-400 border-amber-500/20"
              : "bg-muted/40 text-muted-foreground/60 border-border/40 line-through";

        const chip = (
          <span
            key={appId}
            className={cn(
              "inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium border",
              tone,
            )}
          >
            {label}
          </span>
        );

        if (support.status === "unsupported" || support.status === "partial") {
          return (
            <Tooltip key={appId}>
              <TooltipTrigger asChild>{chip}</TooltipTrigger>
              <TooltipContent side="bottom" className="max-w-xs text-xs">
                {t(support.reasonKey ?? "", {
                  defaultValue:
                    support.reasonDefault ?? "当前应用不支持此资产同步",
                })}
              </TooltipContent>
            </Tooltip>
          );
        }
        return chip;
      })}
    </div>
  );
}

interface ProjectAssetEnableSwitchProps {
  assetType: ProjectAssetType;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** 项目级启用开关：资产类型全局不可 link 时置灰并说明原因 */
export function ProjectAssetEnableSwitch({
  assetType,
  checked,
  onCheckedChange,
  disabled,
}: ProjectAssetEnableSwitchProps) {
  const { t } = useTranslation();
  const linkable = isAssetLinkable(assetType);

  const switchEl = (
    <Switch
      checked={checked}
      onCheckedChange={onCheckedChange}
      disabled={disabled || !linkable}
    />
  );

  if (linkable) return switchEl;

  const firstUnsupported = Object.values(ASSET_APP_SUPPORT[assetType]).find(
    (s) => s.status === "unsupported",
  ) as AssetAppSupport | undefined;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className="inline-flex">{switchEl}</span>
      </TooltipTrigger>
      <TooltipContent side="left" className="max-w-xs text-xs">
        {t("projectAssets.switchAllUnsupported", {
          defaultValue: "当前资产类型在所有目标 CLI 上均不支持同步，无法为项目启用",
        })}
        {firstUnsupported?.reasonDefault && (
          <span className="block mt-1 text-muted-foreground">
            {firstUnsupported.reasonDefault}
          </span>
        )}
      </TooltipContent>
    </Tooltip>
  );
}

export function ProjectAssetSupportTooltipProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  return <TooltipProvider delayDuration={200}>{children}</TooltipProvider>;
}
