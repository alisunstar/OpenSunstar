import * as React from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

interface EmptyStateProps extends React.HTMLAttributes<HTMLDivElement> {
  /** 顶部图标（lucide），不传则只显示文字 */
  icon?: LucideIcon;
  /** 主标题，说明这里本该有什么 */
  title: string;
  /** 补充说明，告诉用户下一步可以做什么 */
  description?: string;
  /** 动作槽：通常放一个 <Button> */
  action?: React.ReactNode;
}

/**
 * 统一空状态：收敛各页面散装的「暂无数据」实现。
 * 图标置于 muted 圆形底上，文字层级遵循排版基元。
 */
function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
  ...props
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center gap-2 py-12 text-center",
        className,
      )}
      {...props}
    >
      {Icon && (
        <div className="mb-1 flex h-10 w-10 items-center justify-center rounded-full bg-muted">
          <Icon className="h-5 w-5 text-muted-foreground" aria-hidden />
        </div>
      )}
      <p className="text-sm font-medium text-foreground">{title}</p>
      {description && (
        <p className="max-w-sm text-xs text-muted-foreground">{description}</p>
      )}
      {action && <div className="mt-3">{action}</div>}
    </div>
  );
}

export { EmptyState };
export type { EmptyStateProps };
