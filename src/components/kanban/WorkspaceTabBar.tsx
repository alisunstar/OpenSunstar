import { useTranslation } from "react-i18next";
import { LayoutDashboard, LayoutGrid, Table2 } from "lucide-react";
import type { WorkspaceTab } from "@/types/workspace";
import { cn } from "@/lib/utils";

interface WorkspaceTabBarProps {
  activeTab: WorkspaceTab;
  onChange: (tab: WorkspaceTab) => void;
}

const TABS: {
  id: WorkspaceTab;
  icon: typeof LayoutDashboard;
  labelKey: string;
  defaultLabel: string;
}[] = [
  {
    id: "dashboard",
    icon: LayoutDashboard,
    labelKey: "workspace.tabs.dashboard",
    defaultLabel: "今日工作台",
  },
  {
    id: "board",
    icon: LayoutGrid,
    labelKey: "workspace.tabs.board",
    defaultLabel: "项目看板",
  },
  {
    id: "assetsMatrix",
    icon: Table2,
    labelKey: "workspace.tabs.assetsMatrix",
    defaultLabel: "AI 资产总览",
  },
];

export function WorkspaceTabBar({ activeTab, onChange }: WorkspaceTabBarProps) {
  const { t } = useTranslation();

  return (
    <div
      className="flex flex-wrap gap-1 rounded-lg border border-border/50 bg-muted/20 p-0.5"
      role="tablist"
      aria-label={t("workspace.tabs.label", { defaultValue: "工作区视图" })}
    >
      {TABS.map(({ id, icon: Icon, labelKey, defaultLabel }) => (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={activeTab === id}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
            activeTab === id
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
          onClick={() => onChange(id)}
        >
          <Icon className="h-3.5 w-3.5 shrink-0" />
          {t(labelKey, { defaultValue: defaultLabel })}
        </button>
      ))}
    </div>
  );
}
