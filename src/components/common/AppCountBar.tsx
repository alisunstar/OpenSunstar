import React from "react";
import { Badge } from "@/components/ui/badge";
import type { AppId } from "@/lib/api/types";
import { APP_IDS, APP_ICON_MAP } from "@/config/appConfig";

interface AppCountBarProps {
  totalLabel: string;
  counts: Partial<Record<AppId, number>>;
  appIds?: AppId[];
  /** When provided, badges become clickable filter buttons */
  onAppClick?: (app: AppId | null) => void;
  /** Currently active app filter (null = show all) */
  activeApp?: AppId | null;
}

export const AppCountBar: React.FC<AppCountBarProps> = ({
  totalLabel,
  counts,
  appIds = APP_IDS,
  onAppClick,
  activeApp = null,
}) => {
  const clickable = !!onAppClick;

  return (
    <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6 flex items-center justify-between gap-4">
      <Badge
        variant="outline"
        className={`h-7 px-3 ${clickable ? "cursor-pointer hover:bg-background/80" : ""} ${activeApp === null && clickable ? "ring-1 ring-primary/50 bg-primary/10" : "bg-background/50"}`}
        onClick={clickable ? () => onAppClick!(null) : undefined}
      >
        {totalLabel}
      </Badge>
      <div className="flex items-center gap-2 overflow-x-auto no-scrollbar">
        {appIds.map((app) => {
          const isActive = activeApp === app;
          return (
            <Badge
              key={app}
              variant="secondary"
              className={`${APP_ICON_MAP[app].badgeClass} ${clickable ? "cursor-pointer" : ""} ${isActive ? "ring-2 ring-primary/60 font-bold" : ""}`}
              onClick={clickable ? () => onAppClick!(app) : undefined}
            >
              <span className="opacity-75">{APP_ICON_MAP[app].label}:</span>
              <span className="font-bold ml-1">{counts[app] ?? 0}</span>
            </Badge>
          );
        })}
      </div>
    </div>
  );
};
