import { useTranslation } from "react-i18next";
import {
  FolderOpen,
  Trash2,
  MoreVertical,
  GripVertical,
  ChevronDown,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { StageKey } from "@/hooks/useProjectStages";
import type { Project } from "@/types/project";

interface ProjectCardProps {
  project: Project;
  stage: StageKey;
  progress?: number; // undefined=未设置, 仅 MVP 阶段显示
  onClick: () => void;
  onRemove: () => void;
  onOpenFolder?: () => void;
  onStageChange?: (stage: StageKey) => void;
}

// ── 阶段配置 ────────────────────────────────────

const STAGE_STYLE: Record<
  StageKey,
  { border: string; dot: string; label: string }
> = {
  mvp: { border: "border-l-purple-500", dot: "bg-purple-500", label: "MVP" },
  rapid: {
    border: "border-l-emerald-500",
    dot: "bg-emerald-500",
    label: "已上线",
  },
  stable: {
    border: "border-l-blue-500",
    dot: "bg-blue-500",
    label: "稳定维护",
  },
};

const STAGE_OPTIONS: { key: StageKey; label: string }[] = [
  { key: "mvp", label: "MVP 阶段" },
  { key: "rapid", label: "快速迭代" },
  { key: "stable", label: "稳定维护" },
];

// ── 工具函数 ────────────────────────────────────

function relativeTime(iso: string, t: (k: string, d: { defaultValue: string }) => string): string {
  const now = Date.now();
  const then = new Date(iso).getTime();
  const diffMs = now - then;
  const minutes = Math.floor(diffMs / 60000);
  const hours = Math.floor(diffMs / 3600000);
  const days = Math.floor(diffMs / 86400000);

  if (minutes < 1) return t("time.justNow", { defaultValue: "刚刚" });
  if (minutes < 60)
    return t("time.minutesAgo", { defaultValue: "{{n}} 分钟前" }).replace(
      "{{n}}",
      String(minutes),
    );
  if (hours < 24)
    return t("time.hoursAgo", { defaultValue: "{{n}} 小时前" }).replace(
      "{{n}}",
      String(hours),
    );
  if (days < 30)
    return t("time.daysAgo", { defaultValue: "{{n}} 天前" }).replace(
      "{{n}}",
      String(days),
    );
  return new Date(iso).toLocaleDateString();
}

// ── 组件 ──────────────────────────────────────────

export function ProjectCard({
  project,
  stage,
  progress,
  onClick,
  onRemove,
  onOpenFolder,
  onStageChange,
}: ProjectCardProps) {
  const { t } = useTranslation();
  const style = STAGE_STYLE[stage];
  const folderName = project.path.split(/[/\\]/).pop() || project.path;
  const dirPath =
    project.path.split(/[/\\]/).slice(0, -1).join("/") || project.path;

  const showProgress = stage === "mvp" && typeof progress === "number" && progress > 0;

  return (
    <article
      className={cn(
        "group relative rounded-xl border border-border/60 bg-card/50",
        "hover:border-primary/25 hover:shadow-md hover:shadow-primary/5",
        "transition-all duration-200 cursor-pointer",
        "border-l-[3px]",
        style.border,
      )}
      onClick={onClick}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
    >
      <div className="px-4 pt-3.5 pb-3">
        {/* ── 第一行：名称 + 操作 ────────────── */}
        <div className="flex items-start gap-2">
          <GripVertical className="h-4 w-4 text-muted-foreground/30 shrink-0 mt-0.5 opacity-0 group-hover:opacity-100 transition-opacity" />

          <div className="min-w-0 flex-1">
            <h3 className="text-sm font-semibold text-foreground truncate leading-tight">
              {project.name}
            </h3>
            <p
              className="text-[11px] text-muted-foreground/60 mt-0.5 truncate font-mono"
              title={project.path}
            >
              {folderName}
            </p>
          </div>

          <div className="flex items-center gap-0.5 shrink-0">
            {/* 阶段快捷切换 */}
            {onStageChange && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    className={cn(
                      "h-6 px-2 rounded-md text-[11px] font-medium gap-1",
                      "opacity-0 group-hover:opacity-100 transition-all",
                      style.dot.replace("bg-", "text-"),
                    )}
                    onClick={(e) => e.stopPropagation()}
                  >
                    <span
                      className={cn("w-1.5 h-1.5 rounded-full", style.dot)}
                    />
                    {style.label}
                    <ChevronDown className="h-3 w-3" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  align="end"
                  className="w-36"
                  onClick={(e) => e.stopPropagation()}
                >
                  {STAGE_OPTIONS.map((opt) => (
                    <DropdownMenuItem
                      key={opt.key}
                      onClick={() => onStageChange(opt.key)}
                      disabled={opt.key === stage}
                    >
                      <span
                        className={cn(
                          "w-1.5 h-1.5 rounded-full mr-2",
                          STAGE_STYLE[opt.key].dot,
                        )}
                      />
                      {opt.label}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            )}

            {/* 更多菜单 */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                  onClick={(e) => e.stopPropagation()}
                >
                  <MoreVertical className="h-3.5 w-3.5" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent
                align="end"
                className="w-40"
                onClick={(e) => e.stopPropagation()}
              >
                {onOpenFolder && (
                  <DropdownMenuItem onClick={onOpenFolder}>
                    <FolderOpen className="h-3.5 w-3.5 mr-2" />
                    {t("kanban.openFolder", { defaultValue: "打开目录" })}
                  </DropdownMenuItem>
                )}
                <DropdownMenuItem onClick={onClick}>
                  <MoreVertical className="h-3.5 w-3.5 mr-2" />
                  {t("kanban.viewDetail", { defaultValue: "查看详情" })}
                </DropdownMenuItem>
                <DropdownMenuItem
                  className="text-destructive focus:text-destructive"
                  onClick={onRemove}
                >
                  <Trash2 className="h-3.5 w-3.5 mr-2" />
                  {t("kanban.remove", { defaultValue: "移除项目" })}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* ── 第二行：描述 ──────────────────── */}
        {project.description ? (
          <p className="pl-6 mt-1.5 text-[12px] text-muted-foreground/80 leading-relaxed line-clamp-2">
            {project.description}
          </p>
        ) : null}

        {/* ── 第三行：进度条（仅 MVP）───────── */}
        {showProgress && (
          <div className="pl-6 mt-2.5">
            <div className="flex items-center justify-between mb-1">
              <span className="text-[10px] text-muted-foreground/60">
                {t("kanban.progress", { defaultValue: "进度" })}
              </span>
              <span className="text-[10px] font-semibold text-foreground/80 tabular-nums">
                {progress}%
              </span>
            </div>
            <div className="h-1.5 rounded-full bg-muted/50 overflow-hidden">
              <div
                className="h-full rounded-full bg-purple-500 transition-all duration-500 ease-out"
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
        )}

        {/* ── 底部：时间 + 路径 ─────────────── */}
        <div
          className={cn(
            "flex items-center gap-3 mt-2.5 pl-6",
            !project.description && !showProgress && "mt-0",
          )}
        >
          <span className="text-[10px] text-muted-foreground/50">
            {relativeTime(project.addedAt, t)}
          </span>
          <span className="text-[10px] text-muted-foreground/30">·</span>
          <span className="text-[10px] text-muted-foreground/50 font-mono truncate">
            {dirPath}
          </span>
        </div>
      </div>
    </article>
  );
}
