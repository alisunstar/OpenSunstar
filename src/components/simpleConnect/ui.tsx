import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

/** Simple Connect 统一面板外框 */
export const SC_PANEL =
  "rounded-xl border border-border/60 bg-gradient-to-br from-muted/25 to-muted/5 shadow-sm";

/** 内嵌子面板（运行态、统计卡片区） */
export const SC_INNER =
  "rounded-lg border border-border/50 bg-background/45 backdrop-blur-[1px]";

/** 步骤内容区外框 */
export const SC_STEP =
  "rounded-xl border border-border/60 bg-card/30 p-5 sm:p-6 space-y-5";

interface SectionHeaderProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}

export function SectionHeader({
  icon: Icon,
  title,
  description,
  action,
  className,
}: SectionHeaderProps) {
  return (
    <div className={cn("flex items-start justify-between gap-3", className)}>
      <div className="flex items-start gap-2.5 min-w-0">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Icon className="h-4 w-4" />
        </div>
        <div className="min-w-0">
          <h3 className="text-sm font-semibold leading-tight">{title}</h3>
          {description && (
            <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
          )}
        </div>
      </div>
      {action}
    </div>
  );
}

interface PanelShellProps {
  className?: string;
  children: React.ReactNode;
  padding?: "sm" | "md";
}

export function PanelShell({
  className,
  children,
  padding = "md",
}: PanelShellProps) {
  return (
    <div
      className={cn(
        SC_PANEL,
        padding === "sm" ? "p-4 space-y-3" : "p-5 space-y-4",
        className,
      )}
    >
      {children}
    </div>
  );
}
