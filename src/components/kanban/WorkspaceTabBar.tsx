import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { LayoutDashboard, LayoutGrid } from "lucide-react";
import type { WorkspaceTab } from "@/types/workspace";
import { cn } from "@/lib/utils";

/**
 * 三个 Tab 共用 `KanbanPage` 里那一整段内容区：换的是里面渲染什么，面板本身
 * 不换。所以三个 Tab 的 `aria-controls` 都指向同一个 id，而面板的
 * `aria-labelledby` 指向**当前选中**的那个 Tab —— 读屏器进入面板时念
 * 「今日告警，标签面板」，跟着切换走。
 *
 * id 由这里导出而不是两边各写一遍字符串：`aria-controls` 指向一个不存在的
 * id 是最常见的 a11y bug，而且它静悄悄——读屏器只是找不到面板，界面上什么
 * 都看不出来。让编译器看着这层引用。
 */
export const WORKSPACE_TABPANEL_ID = "workspace-tabpanel";

export const workspaceTabId = (tab: WorkspaceTab) => `workspace-tab-${tab}`;

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
    defaultLabel: "今日告警",
  },
  /**
   * 「项目看板」吸收了原「AI 资产总览」（工作区重构 2026-07-30）：治理面板
   * 与资产矩阵同为项目维度的巡视内容，分两个 Tab 是信息超载。
   */
  {
    id: "board",
    icon: LayoutGrid,
    labelKey: "workspace.tabs.board",
    defaultLabel: "项目看板",
  },
];

export function WorkspaceTabBar({ activeTab, onChange }: WorkspaceTabBarProps) {
  const { t } = useTranslation();
  const btnRefs = useRef<(HTMLButtonElement | null)[]>([]);

  /**
   * `role="tablist"` 向读屏器承诺两件事：存在对应的 tabpanel，以及**方向键
   * 可以在标签之间走**。这套控件原来一件都没做到（审查报告 §7）—— 说到做
   * 不到的 role 比没有 role 更坏。
   *
   * 这里补的是第二件。与看板矩阵那排筛选胶囊的处理正好相反：那排的真身是
   * 互斥筛选开关（切的是同一张表格的行，压根没有第二块内容），所以退回
   * `role="group"` + `aria-pressed`；这里切的是三块彼此独立的内容，是真
   * Tab，该做的是把承诺兑现。
   *
   * 用 APG 的「自动激活」模式：方向键走到哪就切到哪。三个 Tab 的内容都已
   * 在本地算好，切换没有网络代价，不需要「先移焦点、再按 Enter 激活」那套
   * 更啰嗦的手动模式。
   */
  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const idx = TABS.findIndex((tab) => tab.id === activeTab);
    let next: number;
    switch (e.key) {
      case "ArrowRight":
        next = (idx + 1) % TABS.length;
        break;
      case "ArrowLeft":
        next = (idx - 1 + TABS.length) % TABS.length;
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = TABS.length - 1;
        break;
      default:
        return;
    }
    e.preventDefault();
    onChange(TABS[next].id);
    btnRefs.current[next]?.focus();
  };

  return (
    <div
      className="flex flex-wrap gap-1 rounded-lg border border-border/50 bg-muted/20 p-0.5"
      role="tablist"
      aria-label={t("workspace.tabs.label", { defaultValue: "工作区视图" })}
      onKeyDown={handleKeyDown}
    >
      {TABS.map(({ id, icon: Icon, labelKey, defaultLabel }, i) => {
        const selected = activeTab === id;
        return (
          <button
            key={id}
            ref={(el) => {
              btnRefs.current[i] = el;
            }}
            id={workspaceTabId(id)}
            type="button"
            role="tab"
            aria-selected={selected}
            aria-controls={WORKSPACE_TABPANEL_ID}
            /*
             * roving tabindex：整条标签栏在 Tab 键序里只占**一站**，进来之后
             * 用方向键选。否则键盘用户每次路过都要按三下 Tab —— 标签越多越
             * 难受，而这正是 tablist 这个 role 存在的理由。
             */
            tabIndex={selected ? 0 : -1}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              selected
                ? "bg-primary text-primary-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
            onClick={() => onChange(id)}
          >
            <Icon className="h-3.5 w-3.5 shrink-0" />
            {t(labelKey, { defaultValue: defaultLabel })}
          </button>
        );
      })}
    </div>
  );
}
