import { useTranslation } from "react-i18next";
import { Loader2, Shield } from "lucide-react";
import type { Project } from "@/types/project";
import type { StageKey } from "@/hooks/useProjectStages";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import { readinessScoreTone } from "@/lib/readinessConstants";
import { cn } from "@/lib/utils";

const STAGE_LABEL: Record<StageKey, string> = {
  mvp: "MVP",
  rapid: "迭代",
  stable: "稳定",
};

const EMPTY_COUNTS: ProjectAssetCounts = {
  mcp: 0,
  skills: 0,
  prompts: 0,
  commands: 0,
  hooks: 0,
  ignore: 0,
  permissions: 0,
  subagents: 0,
};

const COUNT_COLUMNS: {
  key: keyof ProjectAssetCounts;
  label: string;
  width: string;
}[] = [
  { key: "mcp", label: "MCP", width: "w-12" },
  { key: "skills", label: "Skills", width: "w-12" },
  { key: "prompts", label: "Prompts", width: "w-14" },
  { key: "commands", label: "Cmd", width: "w-12" },
  { key: "hooks", label: "Hooks", width: "w-12" },
  { key: "ignore", label: "Ign", width: "w-12" },
  { key: "permissions", label: "Perm", width: "w-12" },
  { key: "subagents", label: "Sub", width: "w-12" },
];

export interface ProjectAssetsMatrixProps {
  projects: Project[];
  getStage: (projectId: string) => StageKey;
  progressMap: Map<string, number>;
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  assetMap: Map<string, ProjectAssetCounts>;
  loading?: boolean;
  onOpenProject: (project: Project, options?: { assetsTab?: boolean }) => void;
}

function cellTone(count: number): string {
  if (count > 0) return "text-emerald-600 dark:text-emerald-400";
  return "text-amber-600/80 dark:text-amber-400/80 font-semibold";
}

function cellBg(count: number): string {
  if (count > 0) return "";
  return "bg-amber-500/5";
}

export function ProjectAssetsMatrix({
  projects,
  getStage,
  progressMap,
  agentReadinessMap,
  assetMap,
  loading,
  onOpenProject,
}: ProjectAssetsMatrixProps) {
  const { t } = useTranslation();

  if (loading && projects.length > 0 && assetMap.size === 0) {
    return (
      <div className="flex items-center justify-center py-16 text-muted-foreground text-sm">
        <Loader2 className="w-4 h-4 animate-spin mr-2" />
        {t("workspace.assetsMatrix.loading", {
          defaultValue: "正在汇总各项目 AI 资产…",
        })}
      </div>
    );
  }

  if (projects.length === 0) return null;

  return (
    <div className="rounded-xl border border-border/60 bg-card/30 overflow-hidden">
      <div className="px-4 py-3 border-b border-border/40 flex items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {t("workspace.assetsMatrix.title", {
              defaultValue: "AI 资产总览",
            })}
          </h3>
          <p className="text-[11px] text-muted-foreground mt-0.5">
            {t("workspace.assetsMatrix.subtitle8", {
              defaultValue:
                "按项目查看 8 类 AI 资产关联数量与就绪状态（项目级启用子集）",
            })}
          </p>
        </div>
        {loading && assetMap.size > 0 && (
          <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground shrink-0">
            <Loader2 className="h-3 w-3 animate-spin" />
            {t("workspace.assetsMatrix.refreshing", { defaultValue: "更新中" })}
          </span>
        )}
      </div>

      <div className="overflow-x-auto">
        <table className="w-full min-w-[960px] text-xs">
          <thead>
            <tr className="border-b border-border/40 bg-muted/20 text-muted-foreground">
              <th className="text-left font-medium px-4 py-2.5 sticky left-0 bg-muted/20 z-10">
                {t("workspace.assetsMatrix.project", {
                  defaultValue: "项目",
                })}
              </th>
              <th className="text-center font-medium px-2 py-2.5 w-14">
                {t("workspace.assetsMatrix.stage", { defaultValue: "阶段" })}
              </th>
              {COUNT_COLUMNS.map((col) => (
                <th
                  key={col.key}
                  className={cn(
                    "text-center font-medium px-1.5 py-2.5",
                    col.width,
                  )}
                  title={col.label}
                >
                  {col.label}
                </th>
              ))}
              <th className="text-center font-medium px-2 py-2.5 w-14">
                {t("workspace.assetsMatrix.readiness", {
                  defaultValue: "就绪",
                })}
              </th>
              <th className="text-center font-medium px-2 py-2.5 w-14">
                {t("workspace.assetsMatrix.progress", {
                  defaultValue: "进度",
                })}
              </th>
            </tr>
          </thead>
          <tbody>
            {projects.map((project) => {
              const stage = getStage(project.id);
              const assets = assetMap.get(project.id) ?? EMPTY_COUNTS;
              const readiness = agentReadinessMap.get(project.id)?.score;
              const progress = progressMap.get(project.id);

              return (
                <tr
                  key={project.id}
                  className="border-b border-border/30 hover:bg-muted/20 cursor-pointer transition-colors"
                  onClick={() => onOpenProject(project, { assetsTab: true })}
                >
                  <td className="px-4 py-2.5 sticky left-0 bg-card/95 z-10">
                    <p className="font-medium text-foreground truncate max-w-[160px]">
                      {project.name}
                    </p>
                  </td>
                  <td className="text-center px-2 py-2.5 text-muted-foreground">
                    {STAGE_LABEL[stage]}
                  </td>
                  {COUNT_COLUMNS.map((col) => {
                    const count = assets[col.key];
                    return (
                      <td
                        key={col.key}
                        className={cn(
                          "text-center px-1.5 py-2.5 font-semibold tabular-nums",
                          cellTone(count),
                          cellBg(count),
                        )}
                      >
                        {count}
                      </td>
                    );
                  })}
                  <td className="text-center px-2 py-2.5">
                    {typeof readiness === "number" ? (
                      <span
                        className={cn(
                          "inline-flex items-center gap-0.5 font-semibold tabular-nums",
                          readinessScoreTone(readiness),
                        )}
                      >
                        <Shield className="h-3 w-3" />
                        {readiness}
                      </span>
                    ) : (
                      <span className="text-muted-foreground/40">—</span>
                    )}
                  </td>
                  <td className="text-center px-2 py-2.5 tabular-nums text-muted-foreground">
                    {stage === "mvp" && typeof progress === "number"
                      ? `${progress}%`
                      : "—"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
