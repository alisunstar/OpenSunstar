import { useMemo } from "react";

import { useTranslation } from "react-i18next";

import {

  AlertTriangle,

  ArrowRight,

  Clock,

  Loader2,

  Shield,

  Sparkles,

  TrendingUp,

} from "lucide-react";

import { Button } from "@/components/ui/button";

import type { Project } from "@/types/project";

import type { StageKey } from "@/hooks/useProjectStages";

import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";

import { SummaryCard } from "./SummaryCard";

import { cn } from "@/lib/utils";



import {
  AGENT_READINESS_MAX,
  isReadinessOk,
  readinessScoreTone,
} from "@/lib/readinessConstants";

const MVP_PROGRESS_WARN = 50;



export interface TodayWorkspaceProps {

  projects: Project[];

  getStage: (projectId: string) => StageKey;

  progressMap: Map<string, number>;

  agentReadinessMap: Map<string, number>;

  assetMap: Map<string, ProjectAssetCounts>;

  commits7dMap: Map<string, number>;

  overviewWindowDays: number;

  lastUpdatedAt?: number | null;

  isRefreshing?: boolean;

  onOpenProject: (project: Project, options?: { assetsTab?: boolean }) => void;

}



interface AttentionItem {

  project: Project;

  reasons: string[];

  readiness: number | null;

}



function formatDashboardUpdatedAt(

  timestampMs: number,

  t: (key: string, options?: Record<string, unknown>) => string,

): string {

  const diff = Date.now() - timestampMs;

  const minutes = Math.floor(diff / 60_000);

  const hours = Math.floor(diff / 3_600_000);



  if (minutes < 1) {

    return t("workspace.dashboard.updatedJustNow", { defaultValue: "刚刚更新" });

  }

  if (minutes < 60) {

    return t("workspace.dashboard.updatedMinutesAgo", {

      count: minutes,

      defaultValue: `${minutes} 分钟前更新`,

    });

  }

  if (hours < 24) {

    return t("workspace.dashboard.updatedHoursAgo", {

      count: hours,

      defaultValue: `${hours} 小时前更新`,

    });

  }

  return t("workspace.dashboard.updatedAt", {

    time: new Date(timestampMs).toLocaleString(),

    defaultValue: `更新于 ${new Date(timestampMs).toLocaleString()}`,

  });

}



export function TodayWorkspace({

  projects,

  getStage,

  progressMap,

  agentReadinessMap,

  assetMap,

  commits7dMap,

  overviewWindowDays,

  lastUpdatedAt,

  isRefreshing,

  onOpenProject,

}: TodayWorkspaceProps) {

  const { t } = useTranslation();



  const stats = useMemo(() => {

    let readinessSum = 0;

    let readinessCount = 0;

    let lowReadiness = 0;

    let missingMcp = 0;

    let activeProjects = 0;

    let mvpBehind = 0;



    const attention: AttentionItem[] = [];



    for (const project of projects) {

      const readiness = agentReadinessMap.get(project.id);

      if (typeof readiness === "number") {

        readinessSum += readiness;

        readinessCount += 1;

        if (!isReadinessOk(readiness)) lowReadiness += 1;

      }



      const assets = assetMap.get(project.id);

      if (!assets || assets.mcp === 0) missingMcp += 1;



      const commits = commits7dMap.get(project.id) ?? 0;

      if (commits > 0) activeProjects += 1;



      const stage = getStage(project.id);

      const progress = progressMap.get(project.id);

      if (

        stage === "mvp" &&

        typeof progress === "number" &&

        progress < MVP_PROGRESS_WARN

      ) {

        mvpBehind += 1;

      }



      const reasons: string[] = [];

      if (typeof readiness === "number" && !isReadinessOk(readiness)) {

        reasons.push(

          t("workspace.dashboard.reasonReadiness", {

            score: readiness,

            max: AGENT_READINESS_MAX,

            defaultValue: `就绪分 ${readiness}/${AGENT_READINESS_MAX}`,

          }),

        );

      }

      if (!assets || assets.mcp === 0) {

        reasons.push(

          t("workspace.dashboard.reasonMcp", {

            defaultValue: "未关联 MCP",

          }),

        );

      }

      if (!assets || assets.skills === 0) {

        reasons.push(

          t("workspace.dashboard.reasonSkills", {

            defaultValue: "未配置 Skills",

          }),

        );

      }

      if (!assets || assets.prompts === 0) {

        reasons.push(

          t("workspace.dashboard.reasonPrompts", {

            defaultValue: "未关联 Prompts",

          }),

        );

      }

      if (

        stage === "mvp" &&

        typeof progress === "number" &&

        progress < MVP_PROGRESS_WARN

      ) {

        reasons.push(

          t("workspace.dashboard.reasonProgress", {

            progress,

            defaultValue: `MVP 进度 ${progress}%`,

          }),

        );

      }



      if (reasons.length > 0) {

        attention.push({

          project,

          reasons,

          readiness: readiness ?? null,

        });

      }

    }



    attention.sort((a, b) => {

      const ar = a.readiness ?? 999;

      const br = b.readiness ?? 999;

      return ar - br;

    });



    return {

      avgReadiness:

        readinessCount > 0

          ? Math.round(readinessSum / readinessCount)

          : null,

      lowReadiness,

      missingMcp,

      activeProjects,

      mvpBehind,

      attention: attention.slice(0, 8),

    };

  }, [

    projects,

    agentReadinessMap,

    assetMap,

    commits7dMap,

    progressMap,

    getStage,

    t,

  ]);



  const showLoadingPlaceholder =

    Boolean(isRefreshing) &&

    agentReadinessMap.size === 0 &&

    assetMap.size === 0;



  return (

    <div className="space-y-4">

      <div className="rounded-xl border border-primary/20 bg-gradient-to-br from-primary/5 via-card/40 to-card/20 p-4">

        <div className="flex flex-wrap items-start justify-between gap-3">

          <div>

            <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">

              <TrendingUp className="h-4 w-4 text-primary" />

              {t("workspace.dashboard.title", { defaultValue: "今日工作台" })}

            </h3>

            <p className="text-xs text-muted-foreground mt-1 max-w-xl">

              {t("workspace.dashboard.subtitle", {

                count: projects.length,

                days: overviewWindowDays,

                defaultValue: `共 ${projects.length} 个项目 · 优先关注进度与 AI 资产配置`,

              })}

            </p>

          </div>

          <div className="flex items-center gap-2 text-[11px] text-muted-foreground shrink-0">

            {isRefreshing ? (

              <>

                <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />

                <span>

                  {t("workspace.dashboard.refreshing", {

                    defaultValue: "正在刷新…",

                  })}

                </span>

              </>

            ) : lastUpdatedAt ? (

              <>

                <Clock className="h-3.5 w-3.5" />

                <span title={new Date(lastUpdatedAt).toLocaleString()}>

                  {formatDashboardUpdatedAt(lastUpdatedAt, t)}

                </span>

              </>

            ) : null}

          </div>

        </div>

      </div>



      {showLoadingPlaceholder ? (

        <div className="flex items-center justify-center py-10 text-sm text-muted-foreground rounded-xl border border-border/50 bg-card/20">

          <Loader2 className="w-4 h-4 animate-spin mr-2" />

          {t("workspace.dashboard.loading", {

            defaultValue: "正在汇总项目就绪与资产数据…",

          })}

        </div>

      ) : (

        <>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">

            <SummaryCard

              label={t("workspace.dashboard.attentionCount", {

                defaultValue: "待关注项目",

              })}

              value={String(stats.attention.length)}

              color={

                stats.attention.length > 0

                  ? "text-amber-500"

                  : "text-emerald-500"

              }

              sub={t("workspace.dashboard.attentionHint", {

                defaultValue: "就绪不足或资产缺失",

              })}

            />

            <SummaryCard

              label={t("workspace.dashboard.avgReadiness", {

                defaultValue: "平均就绪分",

              })}

              value={

                stats.avgReadiness !== null ? String(stats.avgReadiness) : "—"

              }

              unit={stats.avgReadiness !== null ? `/${AGENT_READINESS_MAX}` : undefined}

              color={

                stats.avgReadiness !== null &&

                isReadinessOk(stats.avgReadiness)

                  ? "text-emerald-500"

                  : "text-amber-500"

              }

            />

            <SummaryCard

              label={t("workspace.dashboard.missingMcp", {

                defaultValue: "缺 MCP 项目",

              })}

              value={String(stats.missingMcp)}

              color={

                stats.missingMcp > 0 ? "text-amber-500" : "text-foreground"

              }

            />

            <SummaryCard

              label={t("workspace.dashboard.activeProjects", {

                days: overviewWindowDays,

                defaultValue: `近 ${overviewWindowDays} 天活跃`,

              })}

              value={String(stats.activeProjects)}

              sub={t("workspace.dashboard.activeHint", {

                defaultValue: "有 Git 提交",

              })}

            />

          </div>



          {stats.mvpBehind > 0 && (

            <p className="text-[11px] text-amber-600/90 dark:text-amber-400/90 px-1">

              {t("workspace.dashboard.mvpBehindHint", {

                count: stats.mvpBehind,

                defaultValue: `${stats.mvpBehind} 个 MVP 项目进度低于 50%，建议优先推进。`,

              })}

            </p>

          )}



          <div className="rounded-xl border border-border/60 bg-card/30 p-4">

            <div className="flex items-center gap-2 mb-3">

              <AlertTriangle className="h-4 w-4 text-amber-500" />

              <h4 className="text-sm font-semibold text-foreground">

                {t("workspace.dashboard.queueTitle", {

                  defaultValue: "建议优先处理",

                })}

              </h4>

            </div>



            {stats.attention.length === 0 ? (

              <p className="text-xs text-muted-foreground py-4 text-center">

                {t("workspace.dashboard.allGood", {

                  defaultValue: "各项目配置状态良好，可直接进入迭代。",

                })}

              </p>

            ) : (

              <div className="space-y-2">

                {stats.attention.map(({ project, reasons, readiness }) => (

                  <div

                    key={project.id}

                    className="group flex flex-wrap items-center gap-2 rounded-lg border border-border/50 bg-background/50 px-3 py-2.5 transition-colors hover:border-primary/30 hover:bg-primary/5"

                  >

                    <div className="flex-1 min-w-[180px]">

                      <p className="text-sm font-medium text-foreground truncate group-hover:text-primary transition-colors">

                        {project.name}

                      </p>

                      <p className="text-[11px] text-muted-foreground mt-0.5">

                        {reasons.join(" · ")}

                      </p>

                    </div>

                    {typeof readiness === "number" && (

                      <span

                        className={cn(

                          "inline-flex items-center gap-1 text-[10px] font-semibold tabular-nums shrink-0",

                          readinessScoreTone(readiness),

                        )}

                      >

                        <Shield className="h-3 w-3" />

                        {readiness}/{AGENT_READINESS_MAX}

                      </span>

                    )}

                    <Button

                      variant="outline"

                      size="sm"

                      className="h-7 text-xs shrink-0 opacity-90 group-hover:opacity-100"

                      onClick={() =>

                        onOpenProject(project, { assetsTab: true })

                      }

                    >

                      <Sparkles className="h-3 w-3 mr-1" />

                      {t("workspace.dashboard.configureAssets", {

                        defaultValue: "配置资产",

                      })}

                      <ArrowRight className="h-3 w-3 ml-1" />

                    </Button>

                  </div>

                ))}

              </div>

            )}

          </div>

        </>

      )}

    </div>

  );

}

