import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FileText,
  Globe2,
  Layers,
  Puzzle,
  RefreshCw,
  Tag,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useProjectAssets } from "@/hooks/useProjectAssets";
import {
  projectsApi,
  type ProjectContextFile,
  type ProjectPromptLink,
} from "@/lib/api/projects";
import { promptsApi, type Prompt } from "@/lib/api/prompts";
import type { AppId } from "@/lib/api/types";

// ─── Types & Constants ──────────────────────────────────────────────────────

const PROMPT_APPS: AppId[] = ["claude", "codex", "gemini", "opencode", "hermes"];

interface EnrichedRule {
  prompt: Prompt;
  appType: string;
  linkEnabled: boolean;
}

// ─── Helpers ────────────────────────────────────────────────────────────────

function parseJsonArray(raw: string | undefined): string[] {
  if (!raw) return [];
  try {
    const arr = JSON.parse(raw);
    return Array.isArray(arr) ? arr : [];
  } catch {
    return [];
  }
}

// ─── Component ──────────────────────────────────────────────────────────────

export function ProjectRulesContextPanel({
  projectId,
}: {
  projectId: string;
}) {
  const { t } = useTranslation();
  const assets = useProjectAssets(projectId);

  const [promptMap, setPromptMap] = useState<
    Record<string, Record<string, Prompt>>
  >({});
  const [loadingPrompts, setLoadingPrompts] = useState(false);
  const [contextFiles, setContextFiles] = useState<ProjectContextFile[]>([]);

  // Load full prompt data for all apps
  const loadPrompts = useCallback(async () => {
    setLoadingPrompts(true);
    try {
      const results = await Promise.all(
        PROMPT_APPS.map(async (app) => {
          const data = await promptsApi.getPrompts(app);
          return [app, data] as const;
        }),
      );
      setPromptMap(Object.fromEntries(results));
    } catch {
      // silently fail — panel shows empty state
    } finally {
      setLoadingPrompts(false);
    }
  }, []);

  // Load context file status from backend
  const loadContextFiles = useCallback(async () => {
    try {
      const files = await projectsApi.getContextFiles(projectId);
      setContextFiles(files);
    } catch {
      // silently fail
    }
  }, [projectId]);

  useEffect(() => {
    void loadPrompts();
    void loadContextFiles();
  }, [loadPrompts, loadContextFiles]);

  // Enrich linked prompts with full data
  const enrichedRules: EnrichedRule[] = useMemo(() => {
    const links: ProjectPromptLink[] = assets.prompts?.links ?? [];
    return links
      .filter((l) => l.enabled)
      .map((link) => {
        const appPrompts = promptMap[link.prompt_app_type] ?? {};
        const prompt = appPrompts[link.prompt_id];
        return {
          prompt: prompt ?? {
            id: link.prompt_id,
            name: link.prompt_id,
            content: "",
            enabled: true,
          },
          appType: link.prompt_app_type,
          linkEnabled: link.enabled,
        };
      })
      .sort((a, b) => {
        // Fragments after parents; then by name
        const aFrag = a.prompt.isFragment ? 1 : 0;
        const bFrag = b.prompt.isFragment ? 1 : 0;
        if (aFrag !== bFrag) return aFrag - bFrag;
        return a.prompt.name.localeCompare(b.prompt.name);
      });
  }, [assets.prompts?.links, promptMap]);

  // Context files per app — which apps have linked prompts
  const appHasLinked = useMemo(() => {
    const map: Record<string, boolean> = {};
    for (const app of PROMPT_APPS) map[app] = false;
    for (const rule of enrichedRules) {
      map[rule.appType] = true;
    }
    return map;
  }, [enrichedRules]);

  const isLoading = assets.loading || loadingPrompts;

  return (
    <div className="space-y-6">
      {/* ── Section 1: Linked Rules ─────────────────────────────── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Layers className="w-4 h-4 text-primary" />
            <h3 className="text-sm font-semibold">
              {t("rulesContext.title", { defaultValue: "项目规则" })}
            </h3>
            <Badge variant="secondary" className="text-[10px] h-4 px-1.5">
              {enrichedRules.length}
            </Badge>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2"
            onClick={() => {
              void assets.refresh();
              void loadPrompts();
              void loadContextFiles();
            }}
            disabled={isLoading}
          >
            <RefreshCw
              className={cn("w-3.5 h-3.5", isLoading && "animate-spin")}
            />
          </Button>
        </div>

        <p className="text-xs text-muted-foreground">
          {t("rulesContext.hint", {
            defaultValue:
              "已关联到当前项目的规则片段。带 Glob 的规则仅在匹配文件存在时生效。",
          })}
        </p>

        {enrichedRules.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border/60 p-6 text-center">
            <Puzzle className="w-6 h-6 text-muted-foreground/40 mx-auto mb-2" />
            <p className="text-xs text-muted-foreground">
              {t("rulesContext.empty", {
                defaultValue:
                  "暂无已关联的规则。在 Agent 配置 > Prompts 中创建规则片段，然后在项目详情中关联。",
              })}
            </p>
          </div>
        ) : (
          <div className="space-y-1.5">
            {enrichedRules.map((rule) => {
              const targets = parseJsonArray(rule.prompt.targets);
              const globs = parseJsonArray(rule.prompt.globs);
              const isFragment = !!rule.prompt.isFragment;

              return (
                <div
                  key={`${rule.appType}-${rule.prompt.id}`}
                  className="flex items-start gap-3 px-3 py-2.5 rounded-md border border-border/40 bg-card/30"
                >
                  {/* Icon + type badge */}
                  <div className="shrink-0 mt-0.5">
                    {isFragment ? (
                      <Puzzle className="w-4 h-4 text-violet-500" />
                    ) : (
                      <FileText className="w-4 h-4 text-blue-500" />
                    )}
                  </div>

                  {/* Main content */}
                  <div className="flex-1 min-w-0 space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium truncate">
                        {rule.prompt.name}
                      </span>
                      <Badge
                        variant="outline"
                        className="text-[9px] h-4 px-1 shrink-0"
                      >
                        {rule.appType}
                      </Badge>
                      {isFragment && (
                        <Badge
                          variant="outline"
                          className="text-[9px] h-4 px-1 shrink-0 text-violet-600 border-violet-500/30 dark:text-violet-400"
                        >
                          {t("rulesContext.fragment", {
                            defaultValue: "片段",
                          })}
                        </Badge>
                      )}
                    </div>

                    {/* Metadata row */}
                    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                      {/* Targets */}
                      <span className="inline-flex items-center gap-1">
                        <Tag className="w-3 h-3" />
                        {targets.length === 0 || targets.includes("*")
                          ? t("rulesContext.allTargets", {
                              defaultValue: "所有工具",
                            })
                          : targets.join(", ")}
                      </span>

                      {/* Globs */}
                      <span className="inline-flex items-center gap-1">
                        <Globe2 className="w-3 h-3" />
                        {globs.length === 0
                          ? t("rulesContext.universal", {
                              defaultValue: "全局",
                            })
                          : globs.join(", ")}
                      </span>

                      {/* Priority (only for fragments) */}
                      {isFragment && rule.prompt.priority != null && (
                        <span>
                          P{rule.prompt.priority}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* ── Section 2: Context Files ────────────────────────────── */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <FileText className="w-4 h-4 text-primary" />
          <h3 className="text-sm font-semibold">
            {t("rulesContext.contextFiles", {
              defaultValue: "上下文文件",
            })}
          </h3>
        </div>

        <p className="text-xs text-muted-foreground">
          {t("rulesContext.contextFilesHint", {
            defaultValue:
              "项目同步时为各 AI 工具生成的上下文文件。仅关联了 Prompt 的工具会生成文件。",
          })}
        </p>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {contextFiles.length > 0
            ? contextFiles.map((cf) => {
                const hasLinked = appHasLinked[cf.app];
                return (
                  <div
                    key={cf.app}
                    className={cn(
                      "flex items-center gap-3 px-3 py-2.5 rounded-md border",
                      cf.exists
                        ? "border-emerald-500/30 bg-emerald-500/5"
                        : hasLinked
                          ? "border-amber-500/30 bg-amber-500/5"
                          : "border-border/40 bg-card/30 opacity-60",
                    )}
                  >
                    <div
                      className={cn(
                        "w-2 h-2 rounded-full shrink-0",
                        cf.exists
                          ? "bg-emerald-500"
                          : hasLinked
                            ? "bg-amber-500"
                            : "bg-muted-foreground/40",
                      )}
                    />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate capitalize">
                        {cf.app}
                      </p>
                      <p className="text-[11px] text-muted-foreground font-mono">
                        {cf.filename}
                      </p>
                    </div>
                    {cf.exists ? (
                      <Badge
                        variant="outline"
                        className={cn(
                          "text-[9px] h-4 px-1 shrink-0",
                          cf.managed
                            ? "text-emerald-600 border-emerald-500/30 dark:text-emerald-400"
                            : "text-amber-600 border-amber-500/30 dark:text-amber-400",
                        )}
                      >
                        {cf.managed
                          ? t("rulesContext.managed", {
                              defaultValue: "已托管",
                            })
                          : t("rulesContext.userCreated", {
                              defaultValue: "用户自建",
                            })}
                      </Badge>
                    ) : hasLinked ? (
                      <Badge
                        variant="outline"
                        className="text-[9px] h-4 px-1 shrink-0 text-amber-600 border-amber-500/30 dark:text-amber-400"
                      >
                        {t("rulesContext.pendingSync", {
                          defaultValue: "待同步",
                        })}
                      </Badge>
                    ) : (
                      <Badge
                        variant="outline"
                        className="text-[9px] h-4 px-1 shrink-0"
                      >
                        {t("rulesContext.notLinked", {
                          defaultValue: "未关联",
                        })}
                      </Badge>
                    )}
                  </div>
                );
              })
            : // Fallback when API hasn't loaded yet
              PROMPT_APPS.map((app) => (
                <div
                  key={app}
                  className="flex items-center gap-3 px-3 py-2.5 rounded-md border border-border/40 bg-card/30 opacity-60"
                >
                  <div className="w-2 h-2 rounded-full shrink-0 bg-muted-foreground/40" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate capitalize">
                      {app}
                    </p>
                    <p className="text-[11px] text-muted-foreground font-mono">
                      —
                    </p>
                  </div>
                  <Badge
                    variant="outline"
                    className="text-[9px] h-4 px-1 shrink-0"
                  >
                    {t("rulesContext.notLinked", {
                      defaultValue: "未关联",
                    })}
                  </Badge>
                </div>
              ))}
        </div>
      </div>
    </div>
  );
}
