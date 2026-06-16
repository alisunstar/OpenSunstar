import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface SidebarItemProps {
  icon: React.ReactNode;
  label: string;
  active?: boolean;
  onClick: () => void;
  badge?: React.ReactNode;
  indent?: boolean;
  /** 是否显示左侧 accent 指示条（默认 true） */
  accent?: boolean;
  /** 折叠模式：仅显示图标，悬停显示 tooltip */
  collapsed?: boolean;
  /** Tooltip 文本（折叠模式下使用，默认用 label） */
  title?: string;
}

export function SidebarItem({
  icon,
  label,
  active,
  onClick,
  badge,
  indent,
  accent = true,
  collapsed = false,
  title,
}: SidebarItemProps) {
  const button = (
    <Button
      variant="ghost"
      size="sm"
      onClick={onClick}
      title={collapsed ? (title ?? label) : undefined}
      className={cn(
        "relative rounded-lg text-sm font-normal transition-all duration-150 ease-out",
        // 折叠模式：居中图标
        collapsed
          ? "w-full justify-center h-9 px-0"
          : ["w-full justify-start gap-3 h-9", indent ? "pl-10" : "pl-3", "pr-3"],
        // 状态样式
        active
          ? [
              "bg-primary/10 text-primary hover:bg-primary/15",
              accent &&
                "before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2",
              accent &&
                "before:w-[3px] before:h-4 before:rounded-full before:bg-primary",
              accent && "before:transition-all before:duration-200",
            ]
          : "text-muted-foreground hover:bg-muted/50 hover:text-foreground",
        collapsed && active && "before:left-0",
      )}
    >
      <span
        className={cn(
          "shrink-0 flex items-center justify-center w-4 h-4 transition-colors duration-150",
          active ? "text-primary" : "text-muted-foreground",
        )}
      >
        {icon}
      </span>

      {!collapsed && (
        <>
          <span className="flex-1 text-left truncate">{label}</span>
          {badge}
        </>
      )}
    </Button>
  );

  // 折叠模式下包裹 Tooltip
  if (collapsed && (title || label)) {
    return (
      <Tooltip delayDuration={300}>
        <TooltipTrigger asChild>{button}</TooltipTrigger>
        <TooltipContent side="right" sideOffset={8}>
          {title ?? label}
        </TooltipContent>
      </Tooltip>
    );
  }

  return button;
}
