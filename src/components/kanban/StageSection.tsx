import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, ArrowUpDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ProjectCard } from "./ProjectCard";
import type { StageKey } from "@/hooks/useProjectStages";
import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";

interface StageSectionProps {
  stage: StageKey;
  projects: Project[];
  stages: Map<string, StageKey>;
  progressMap: Map<string, number>;
  aiSummaryMap?: Map<string, string>;
  aiLoadingMap?: Map<string, boolean>;
  aiHealthMap?: Map<string, number>;
  agentReadinessMap?: Map<string, AgentReadinessBatchEntry>;
  onProjectClick: (project: Project) => void;
  onProjectRemove: (projectId: string) => void;
  onStageChange: (projectId: string, stage: StageKey) => void;
  onOpenFolder?: (path: string) => void;
}

type SortKey = "name" | "date";

const STAGE_CONFIG: Record<
  StageKey,
  { titleKey: string; titleDefault: string; dotClass: string; emptyKey: string }
> = {
  mvp: {
    titleKey: "board.stage.mvp.title",
    titleDefault: "MVP 阶段（未上线）",
    dotClass: "bg-purple-500",
    emptyKey: "kanban.stage.mvp.empty",
  },
  rapid: {
    titleKey: "board.stage.rapid.title",
    titleDefault: "快速迭代阶段（已上线）",
    dotClass: "bg-emerald-500",
    emptyKey: "kanban.stage.rapid.empty",
  },
  stable: {
    titleKey: "board.stage.stable.title",
    titleDefault: "慢迭代阶段（稳定维护）",
    dotClass: "bg-blue-500",
    emptyKey: "kanban.stage.stable.empty",
  },
};

export function StageSection({
  stage,
  projects,
  stages,
  progressMap,
  aiSummaryMap,
  aiLoadingMap,
  aiHealthMap,
  agentReadinessMap,
  onProjectClick,
  onProjectRemove,
  onStageChange,
  onOpenFolder,
}: StageSectionProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(true);
  const [sortBy, setSortBy] = useState<SortKey>("name");
  const cfg = STAGE_CONFIG[stage];

  const sorted = useMemo(() => {
    return [...projects].sort((a, b) => {
      if (sortBy === "name") return a.name.localeCompare(b.name);
      return new Date(b.addedAt).getTime() - new Date(a.addedAt).getTime();
    });
  }, [projects, sortBy]);

  return (
    <section className="space-y-3">
      {/* 阶段头部 */}
      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
          aria-controls={`stage-${stage}`}
          className={cn(
            "flex items-center gap-2.5 px-1 py-2 rounded-lg",
            "hover:bg-muted/30 transition-colors",
          )}
        >
          <span className={cn("w-2.5 h-2.5 rounded-full", cfg.dotClass)} />
          <h2 className="text-sm font-semibold text-foreground">
            {t(cfg.titleKey, { defaultValue: cfg.titleDefault })}
          </h2>
          <span className="text-xs text-muted-foreground tabular-nums">
            {projects.length}
          </span>
          <ChevronDown
            className={cn(
              "h-4 w-4 text-muted-foreground transition-transform duration-200",
              expanded && "rotate-180",
            )}
          />
        </button>

        {/* 排序 */}
        {projects.length > 1 && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs text-muted-foreground gap-1"
              >
                <ArrowUpDown className="h-3 w-3" />
                {sortBy === "name"
                  ? t("kanban.sortByName", { defaultValue: "名称" })
                  : t("kanban.sortByDate", { defaultValue: "日期" })}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-28">
              <DropdownMenuItem onClick={() => setSortBy("name")}>
                {t("kanban.sortByName", { defaultValue: "按名称" })}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setSortBy("date")}>
                {t("kanban.sortByDate", { defaultValue: "按日期" })}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>

      {/* 卡片网格 */}
      <AnimatePresence initial={false}>
        {expanded && (
          <motion.div
            id={`stage-${stage}`}
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeInOut" }}
            className="overflow-hidden"
          >
            {sorted.length === 0 ? (
              <div className="py-8 text-center rounded-xl border border-dashed border-border/50 bg-muted/10">
                <p className="text-xs text-muted-foreground/60">
                  {t(cfg.emptyKey, { defaultValue: "此阶段暂无项目" })}
                </p>
              </div>
            ) : (
              <div className="grid gap-3 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
                {sorted.map((project) => (
                  <ProjectCard
                    key={project.id}
                    project={project}
                    stage={stages.get(project.id) ?? "mvp"}
                    progress={progressMap.get(project.id)}
                    aiSummary={aiSummaryMap?.get(project.id)}
                    aiSummaryLoading={aiLoadingMap?.get(project.id)}
                    healthScore={aiHealthMap?.get(project.id)}
                    agentReadiness={agentReadinessMap?.get(project.id)?.score}
                    agentDriftCount={agentReadinessMap?.get(project.id)?.driftCount}
                    onClick={() => onProjectClick(project)}
                    onRemove={() => onProjectRemove(project.id)}
                    onStageChange={(s) => onStageChange(project.id, s)}
                    onOpenFolder={
                      onOpenFolder
                        ? () => onOpenFolder(project.path)
                        : undefined
                    }
                  />
                ))}
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  );
}
