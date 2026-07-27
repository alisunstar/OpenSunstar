import { useTranslation } from "react-i18next";
import {
  FolderOpen,
  Trash2,
  MoreVertical,
  ChevronDown,
  Shield,
} from "lucide-react";
import { readinessScoreTone } from "@/lib/readinessConstants";
import { projectScoreTitle } from "@/lib/kanban/projectScores";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { StageKey } from "@/hooks/useProjectStages";
import type { ProjectView } from "@/types/projectView";

interface ProjectCardProps {
  /**
   * 这个项目在组合视图里的那一行（审查报告 §6.1）。
   *
   * 此前是 8 个平行 prop（project / stage / progress / aiSummary /
   * aiSummaryLoading / healthScore / agentReadiness / agentDriftCount），由
   * `StageSection` 逐个 `xxxMap.get(project.id)` 现拼 —— 同一个 id 写八遍，
   * 写错一遍就是一张张冠李戴的卡片，而且类型完全查不出来。
   */
  view: ProjectView;
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

function relativeTime(
  iso: string,
  t: (k: string, d: { defaultValue: string }) => string,
): string {
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
  view,
  onClick,
  onRemove,
  onOpenFolder,
  onStageChange,
}: ProjectCardProps) {
  const { t } = useTranslation();
  const {
    project,
    stage,
    progress,
    aiSummary,
    aiSummaryLoading,
    aiHealthScore: healthScore,
    readiness,
  } = view;
  // 两个分数在同一个对象里各占一个字段，看得见它们不是一回事（§5.2）。
  const agentReadiness = readiness?.score;
  const agentDriftCount = readiness?.driftCount ?? 0;
  const style = STAGE_STYLE[stage];
  const folderName = project.path.split(/[/\\]/).pop() || project.path;
  const dirPath =
    project.path.split(/[/\\]/).slice(0, -1).join("/") || project.path;

  const showProgress =
    stage === "mvp" && typeof progress === "number" && progress > 0;

  return (
    <article
      className={cn(
        "group relative rounded-xl border border-border/60 bg-card/50",
        "hover:border-primary/25 hover:shadow-md hover:shadow-primary/5",
        "transition-all duration-200 cursor-pointer",
        "border-l-[3px]",
        style.border,
      )}
      role="button"
      aria-label={`查看 ${project.name} 详情`}
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

        {/* ── AI 摘要 ─────────────────────── */}
        {aiSummary && (
          <p className="pl-6 mt-1.5 text-[11px] text-primary/70 leading-relaxed line-clamp-1">
            <span className="inline-block px-1 py-0 rounded bg-primary/10 text-primary/60 text-[9px] font-semibold mr-1 align-middle">
              AI
            </span>
            {aiSummary}
          </p>
        )}
        {aiSummaryLoading && !aiSummary && (
          <div className="pl-6 mt-1.5 flex items-center gap-1.5">
            <span className="inline-block px-1 py-0 rounded bg-primary/10 text-primary/60 text-[9px] font-semibold">
              AI
            </span>
            <div className="h-2.5 w-3/4 rounded bg-muted/40 animate-pulse" />
          </div>
        )}

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

        {/* ── 底部：时间 + 路径 + 健康度 ──────── */}
        <div
          className={cn(
            "flex items-center gap-3 mt-2.5 pl-6",
            !project.description &&
              !showProgress &&
              !aiSummary &&
              !aiSummaryLoading &&
              "mt-0",
          )}
        >
          <span className="text-[10px] text-muted-foreground/50">
            {relativeTime(project.addedAt, t)}
          </span>
          <span className="text-[10px] text-muted-foreground/30">·</span>
          <span className="text-[10px] text-muted-foreground/50 font-mono truncate flex-1">
            {dirPath}
          </span>
          {/*
            并排的两个 0-100 分数来源毫不相干：左边是 AI 给的工程健康度，
            右边是配置扫描出来的就绪分。名字统一由 `projectScoreTitle` 出
            （审查报告 §5.2），别在这里手写 —— 手写过一次，就写成了
            「健康评分: 88/100」和「Agent 配置就绪 42/100」两种格式。
          */}
          {typeof healthScore === "number" && (
            <span
              className={cn(
                "shrink-0 inline-flex items-center gap-0.5 text-[10px] font-semibold tabular-nums",
                healthScore >= 80
                  ? "text-emerald-500"
                  : healthScore >= 60
                    ? "text-amber-500"
                    : "text-red-400",
              )}
              title={projectScoreTitle("aiHealth", healthScore, t)}
            >
              <span
                className={cn(
                  "w-1.5 h-1.5 rounded-full",
                  healthScore >= 80
                    ? "bg-emerald-500"
                    : healthScore >= 60
                      ? "bg-amber-500"
                      : "bg-red-400",
                )}
              />
              {healthScore}
            </span>
          )}
          {typeof agentReadiness === "number" && (
            <span
              className={cn(
                "shrink-0 inline-flex items-center gap-0.5 text-[10px] font-semibold tabular-nums",
                readinessScoreTone(agentReadiness),
              )}
              title={projectScoreTitle("agentReadiness", agentReadiness, t, {
                hint: t("kanban.readiness.badgeHint", {
                  defaultValue: "点击查看详情",
                }),
              })}
            >
              <Shield className="h-3 w-3" />
              {agentReadiness}
            </span>
          )}
          {agentDriftCount > 0 && (
            <span
              className="shrink-0 inline-flex items-center rounded-md bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-medium text-amber-700 dark:text-amber-400"
              title={t("kanban.readiness.driftBadge", {
                count: agentDriftCount,
                defaultValue: `${agentDriftCount} 项配置不一致`,
              })}
            >
              {t("kanban.readiness.driftShort", {
                count: agentDriftCount,
                defaultValue: `待处理 ${agentDriftCount}`,
              })}
            </span>
          )}
        </div>
      </div>
    </article>
  );
}
