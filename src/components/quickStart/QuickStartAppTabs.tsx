import { useTranslation } from "react-i18next";
import { ProviderIcon } from "@/components/ProviderIcon";
import { cn } from "@/lib/utils";
import {
  QUICKSTART_APP_IDS,
  type QuickStartAppId,
} from "@/config/quickStartCurated";
import { Monitor, Terminal } from "lucide-react";

const APP_META: Record<
  QuickStartAppId,
  { icon: string; labelKey: string; defaultLabel: string; badge?: typeof Terminal }
> = {
  claude: {
    icon: "claude",
    labelKey: "quickStart.apps.claude",
    defaultLabel: "Claude Code",
  },
  "claude-desktop": {
    icon: "claude",
    labelKey: "quickStart.apps.claudeDesktop",
    defaultLabel: "Claude Desktop",
    badge: Monitor,
  },
  codex: {
    icon: "openai",
    labelKey: "quickStart.apps.codex",
    defaultLabel: "Codex",
  },
  gemini: {
    icon: "gemini",
    labelKey: "quickStart.apps.gemini",
    defaultLabel: "Gemini",
  },
};

interface QuickStartAppTabsProps {
  activeApp: QuickStartAppId;
  onChange: (app: QuickStartAppId) => void;
}

export function QuickStartAppTabs({
  activeApp,
  onChange,
}: QuickStartAppTabsProps) {
  const { t } = useTranslation();

  return (
    <div className="inline-flex flex-wrap gap-1 rounded-xl bg-muted p-1">
      {QUICKSTART_APP_IDS.map((app) => {
        const meta = APP_META[app];
        const isActive = app === activeApp;
        const Badge = meta.badge;
        return (
          <button
            key={app}
            type="button"
            onClick={() => onChange(app)}
            className={cn(
              "inline-flex h-9 items-center gap-2 rounded-lg px-3 text-sm font-medium transition-colors",
              isActive
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:bg-background/60 hover:text-foreground",
            )}
          >
            <ProviderIcon icon={meta.icon} name={meta.defaultLabel} size={18} />
            {Badge && (
              <Badge className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />
            )}
            <span>{t(meta.labelKey, { defaultValue: meta.defaultLabel })}</span>
          </button>
        );
      })}
    </div>
  );
}

export function quickStartAppLabel(
  appId: QuickStartAppId,
  t: (key: string, opts?: { defaultValue?: string }) => string,
): string {
  const meta = APP_META[appId];
  return t(meta.labelKey, { defaultValue: meta.defaultLabel });
}
