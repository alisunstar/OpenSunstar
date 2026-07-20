import { Globe2, FolderOpen } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export type PageScope = "global" | "project";

interface PageScopeBadgeProps {
  scope: PageScope;
  projectName?: string;
  className?: string;
}

/**
 * 作用域标记组件，用于在全局面板或项目面板标题旁显示当前操作的作用域。
 * - global: 蓝色地球图标 + "全局"
 * - project: 绿色文件夹图标 + 项目名称
 */
export function PageScopeBadge({ scope, projectName, className }: PageScopeBadgeProps) {
  if (scope === "global") {
    return (
      <Badge
        variant="outline"
        className={cn(
          "text-[10px] font-medium gap-1 px-2 py-0 h-5",
          "text-blue-600 border-blue-500/30 bg-blue-500/5 dark:text-blue-400",
          className,
        )}
      >
        <Globe2 className="h-3 w-3" />
        全局
      </Badge>
    );
  }

  return (
    <Badge
      variant="outline"
      className={cn(
        "text-[10px] font-medium gap-1 px-2 py-0 h-5",
        "text-emerald-600 border-emerald-500/30 bg-emerald-500/5 dark:text-emerald-400",
        className,
      )}
    >
      <FolderOpen className="h-3 w-3" />
      {projectName ?? "项目"}
    </Badge>
  );
}

/**
 * 全局配置面板的副标题，解释作用域语义。
 */
export function GlobalScopeSubtitle({ className }: { className?: string }) {
  return (
    <p className={cn("text-xs text-muted-foreground mt-1", className)}>
      对所有项目生效的全局配置。按项目定制请在项目详情中关联。
    </p>
  );
}
