import React from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { AppId } from "@/lib/api/types";
import { APP_IDS, APP_ICON_MAP } from "@/config/appConfig";

interface AppToggleGroupProps {
  apps: Partial<Record<AppId, boolean>>;
  onToggle: (app: AppId, enabled: boolean) => void;
  appIds?: AppId[];
  /** 禁用项：展示图标但不可点击，tooltip 显示原因 */
  disabledApps?: Partial<Record<AppId, string>>;
}

export const AppToggleGroup: React.FC<AppToggleGroupProps> = ({
  apps,
  onToggle,
  appIds = APP_IDS,
  disabledApps = {},
}) => {
  return (
    <div className="flex items-center gap-1.5 flex-shrink-0">
      {appIds.map((app) => {
        const { label, icon, activeClass } = APP_ICON_MAP[app];
        const enabled = apps[app];
        const disabledReason = disabledApps[app];
        const isDisabled = Boolean(disabledReason);
        return (
          <Tooltip key={app}>
            <TooltipTrigger asChild>
              <button
                type="button"
                disabled={isDisabled}
                onClick={() => {
                  if (!isDisabled) onToggle(app, !enabled);
                }}
                className={`w-7 h-7 rounded-lg flex items-center justify-center transition-all ${
                  isDisabled
                    ? "opacity-25 cursor-not-allowed"
                    : enabled
                      ? activeClass
                      : "opacity-35 hover:opacity-70"
                }`}
              >
                {icon}
              </button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>
                {label}
                {isDisabled
                  ? ` — ${disabledReason}`
                  : enabled
                    ? " ✓"
                    : ""}
              </p>
            </TooltipContent>
          </Tooltip>
        );
      })}
    </div>
  );
};
