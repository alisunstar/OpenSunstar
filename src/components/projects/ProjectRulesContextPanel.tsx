import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FileText,
  Globe2,
  Layers,
  Link2,
  Plus,
  Puzzle,
  RefreshCw,
  Tag,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { useProjectAssets } from "@/hooks/useProjectAssets";
import {
  projectsApi,
  type ProjectContextFile,
  type ProjectPromptLink,
} from "@/lib/api/projects";
import { promptsApi, type Prompt } from "@/lib/api/prompts";
import type { AppId } from "@/lib/api/types";
import type { AssetHealthPlan } from "@/types/assetHealth";

// ─── Types & Constants ──────────────────────────────────────────────────────

const PROMPT_APPS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

interface EnrichedRule {
  prompt: Prompt;
  appType: string;
  linkEnabled: boolean;
}

interface ProjectRulesContextPanelProps {
  projectId: string;
  onOpenPromptLibrary?: () => void;
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
  onOpenPromptLibrary,
}: ProjectRulesContextPanelProps) {
  const { t } = useTranslation();
  const assets = useProjectAssets(projectId);

  const [promptMap, setPromptMap] = useState<
    Record<string, Record<string, Prompt>>
  >({});
  const [loadingPrompts, setLoadingPrompts] = useState(false);
  const [contextFiles, setContextFiles] = useState<ProjectContextFile[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<"link" | "create" | null>(null);
  const [selectedFragmentKey, setSelectedFragmentKey] = useState("");
  const [ruleApp, setRuleApp] = useState<AppId>("claude");
  const [ruleName, setRuleName] = useState("");
  const [ruleContent, setRuleContent] = useState("");
  const [ruleTargets, setRuleTargets] = useState("claude");
  const [ruleGlobs, setRuleGlobs] = useState("[]");
  const [rulePriority, setRulePriority] = useState("0");
  const [parentPromptId, setParentPromptId] = useState("");
  const [saving, setSaving] = useState(false);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [syncPlan, setSyncPlan] = useState<AssetHealthPlan | null>(null);

  // Load full prompt data for all apps
  const loadPrompts = useCallback(async () => {
    setLoadingPrompts(true);
    setLoadError(null);
    try {
      const results = await Promise.all(
        PROMPT_APPS.map(async (app) => {
          const data = await promptsApi.getPrompts(app);
          return [app, data] as const;
        }),
      );
      setPromptMap(Object.fromEntries(results));
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : String(error));
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

  const linkedRuleFragments = useMemo(
    () => enrichedRules.filter((entry) => entry.prompt.isFragment),
    [enrichedRules],
  );
  const linkedBasePrompts = useMemo(
    () => enrichedRules.filter((entry) => !entry.prompt.isFragment),
    [enrichedRules],
  );
  const fragmentCatalog = useMemo(
    () =>
      PROMPT_APPS.flatMap((app) =>
        Object.values(promptMap[app] ?? {})
          .filter((prompt) => prompt.isFragment)
          .map((prompt) => ({ prompt, app })),
      ).sort((a, b) => a.prompt.name.localeCompare(b.prompt.name)),
    [promptMap],
  );
  const parentPromptOptions = useMemo(
    () =>
      Object.values(promptMap[ruleApp] ?? {})
        .filter((prompt) => !prompt.isFragment)
        .sort((a, b) => a.name.localeCompare(b.name)),
    [promptMap, ruleApp],
  );

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

  const refreshPanel = useCallback(async () => {
    await Promise.all([assets.refresh(), loadPrompts(), loadContextFiles()]);
  }, [assets, loadContextFiles, loadPrompts]);

  const openCreateRule = useCallback(() => {
    setRuleApp("claude");
    setRuleName("");
    setRuleContent("");
    setRuleTargets("claude");
    setRuleGlobs("[]");
    setRulePriority("0");
    setParentPromptId("");
    setDialog("create");
  }, []);

  const linkSelectedRule = useCallback(async () => {
    const [app, promptId] = selectedFragmentKey.split(":", 2) as [
      AppId,
      string,
    ];
    if (!app || !promptId) return;
    setSaving(true);
    try {
      await projectsApi.linkPrompt(projectId, promptId, app, true);
      setActionMessage(
        "规则已关联到当前项目。下一步可预览同步，将规则写入项目上下文文件。",
      );
      setDialog(null);
      await refreshPanel();
    } finally {
      setSaving(false);
    }
  }, [projectId, refreshPanel, selectedFragmentKey]);

  const createAndLinkRule = useCallback(async () => {
    const name = ruleName.trim();
    if (!name || !ruleContent.trim()) return;
    setSaving(true);
    try {
      const timestamp = Math.floor(Date.now() / 1000);
      let parentId = parentPromptId;
      if (!parentId) {
        parentId = `rule-set-${Date.now()}`;
        await promptsApi.upsertPrompt(ruleApp, parentId, {
          id: parentId,
          name: `${name} 规则包`,
          description: "由项目规则向导创建，用于归类规则片段。",
          content: "",
          enabled: false,
          targets: JSON.stringify([ruleApp]),
          globs: "[]",
          priority: 0,
          isFragment: false,
          parentPromptId: null,
          createdAt: timestamp,
          updatedAt: timestamp,
        });
      }
      const ruleId = `rule-${Date.now()}`;
      const targets = ruleTargets
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean);
      await promptsApi.upsertPrompt(ruleApp, ruleId, {
        id: ruleId,
        name,
        description: `关联到项目 ${projectId} 的规则片段。`,
        content: ruleContent.trim(),
        enabled: false,
        targets: JSON.stringify(targets.length > 0 ? targets : [ruleApp]),
        globs: ruleGlobs.trim() || "[]",
        priority: Number.parseInt(rulePriority, 10) || 0,
        isFragment: true,
        parentPromptId: parentId,
        createdAt: timestamp,
        updatedAt: timestamp,
      });
      await projectsApi.linkPrompt(projectId, ruleId, ruleApp, true);
      setActionMessage(
        "规则包已创建，并已自动关联到当前项目。下一步可预览同步。 ",
      );
      setDialog(null);
      await refreshPanel();
    } finally {
      setSaving(false);
    }
  }, [
    parentPromptId,
    projectId,
    refreshPanel,
    ruleApp,
    ruleContent,
    ruleGlobs,
    ruleName,
    rulePriority,
    ruleTargets,
  ]);

  const previewSync = useCallback(async () => {
    setSaving(true);
    try {
      setSyncPlan(await projectsApi.planAssetHealth(projectId));
    } finally {
      setSaving(false);
    }
  }, [projectId]);

  const applySyncPlan = useCallback(async () => {
    if (!syncPlan) return;
    setSaving(true);
    try {
      await projectsApi.applyAssetHealthPlan(projectId, syncPlan.planSha256);
      setSyncPlan(null);
      setActionMessage("项目上下文已同步，并已生成可追溯回执。");
      await refreshPanel();
    } finally {
      setSaving(false);
    }
  }, [projectId, refreshPanel, syncPlan]);

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
              {linkedRuleFragments.length}
            </Badge>
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2"
              onClick={() => void refreshPanel()}
              disabled={isLoading}
              aria-label="刷新项目规则"
              title="刷新项目规则"
            >
              <RefreshCw
                className={cn("w-3.5 h-3.5", isLoading && "animate-spin")}
              />
            </Button>
          </div>
        </div>

        <p className="text-xs text-muted-foreground">
          {t("rulesContext.hint", {
            defaultValue:
              "已关联到当前项目的规则片段。带 Glob 的规则仅在匹配文件存在时生效。",
          })}
        </p>

        {linkedBasePrompts.length > 0 && (
          <p className="text-[11px] text-muted-foreground">
            另有 {linkedBasePrompts.length} 个基础 Prompt 已关联；它们在“项目
            Prompt”中管理，不计入规则片段数量。
          </p>
        )}

        {loadError ? (
          <div className="rounded-lg border border-amber-500/40 bg-amber-500/5 p-4 text-sm">
            <p className="font-medium text-foreground">规则库读取异常</p>
            <p className="mt-1 text-xs text-muted-foreground break-all">
              {loadError}
            </p>
            <Button
              size="sm"
              variant="outline"
              className="mt-3"
              onClick={() => void refreshPanel()}
            >
              重新加载
            </Button>
          </div>
        ) : linkedRuleFragments.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border/60 p-6 text-center">
            <Puzzle className="w-6 h-6 text-muted-foreground/40 mx-auto mb-2" />
            <p className="text-sm font-medium text-foreground">
              当前项目尚未关联规则
            </p>
            <p className="mx-auto mt-1 max-w-xl text-xs text-muted-foreground">
              规则用于约束当前项目中的 AI
              工具行为。可关联已有规则，或创建规则包后自动关联。
            </p>
            <div className="mt-4 flex flex-wrap justify-center gap-2">
              <Button size="sm" onClick={openCreateRule}>
                <Plus className="mr-1.5 h-3.5 w-3.5" />
                {parentPromptOptions.length === 0
                  ? "新建规则包"
                  : "新建规则片段"}
              </Button>
              <Button
                size="sm"
                variant="outline"
                onClick={() => setDialog("link")}
                disabled={fragmentCatalog.length === 0}
              >
                <Link2 className="mr-1.5 h-3.5 w-3.5" />
                关联已有规则
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={onOpenPromptLibrary}
                disabled={!onOpenPromptLibrary}
              >
                管理 Prompt & Rules
              </Button>
            </div>
            {fragmentCatalog.length === 0 && (
              <p className="mt-3 text-[11px] text-muted-foreground">
                全局规则库尚无规则片段；“新建规则包”会同时创建归属 Prompt
                与第一条规则。
              </p>
            )}
          </div>
        ) : (
          <div className="space-y-1.5">
            <div className="flex justify-end gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={() => setDialog("link")}
              >
                <Link2 className="mr-1.5 h-3.5 w-3.5" />
                关联规则
              </Button>
              <Button size="sm" onClick={openCreateRule}>
                <Plus className="mr-1.5 h-3.5 w-3.5" />
                新建规则片段
              </Button>
            </div>
            {linkedRuleFragments.map((rule) => {
              const targets = parseJsonArray(rule.prompt.targets);
              const globs = parseJsonArray(rule.prompt.globs);
              return (
                <div
                  key={`${rule.appType}-${rule.prompt.id}`}
                  className="flex items-start gap-3 px-3 py-2.5 rounded-md border border-border/40 bg-card/30"
                >
                  <div className="shrink-0 mt-0.5">
                    <Puzzle className="w-4 h-4 text-violet-500" />
                  </div>
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
                      <Badge
                        className="text-[9px] h-4 px-1 shrink-0 text-violet-600 border-violet-500/30 dark:text-violet-400"
                        variant="outline"
                      >
                        {t("rulesContext.fragment", { defaultValue: "片段" })}
                      </Badge>
                    </div>
                    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                      <span className="inline-flex items-center gap-1">
                        <Tag className="w-3 h-3" />
                        {targets.length === 0 || targets.includes("*")
                          ? t("rulesContext.allTargets", {
                              defaultValue: "所有工具",
                            })
                          : targets.join(", ")}
                      </span>
                      <span className="inline-flex items-center gap-1">
                        <Globe2 className="w-3 h-3" />
                        {globs.length === 0
                          ? t("rulesContext.universal", {
                              defaultValue: "全局",
                            })
                          : globs.join(", ")}
                      </span>
                      {rule.prompt.priority != null && (
                        <span>P{rule.prompt.priority}</span>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {actionMessage && (
        <div className="rounded-lg border border-primary/30 bg-primary/5 px-4 py-3">
          <p className="text-xs text-foreground">{actionMessage}</p>
          <div className="mt-2 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => void previewSync()}
              disabled={saving}
            >
              预览同步
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setActionMessage(null)}
            >
              稍后处理
            </Button>
          </div>
        </div>
      )}

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

      <Dialog
        open={dialog === "link"}
        onOpenChange={(open) => !open && setDialog(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>关联已有规则</DialogTitle>
            <DialogDescription>
              仅展示规则片段。关联后规则会按目标工具和 Glob
              范围参与当前项目同步。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 px-6 py-5">
            {fragmentCatalog.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                规则库尚无规则片段，请先新建规则包。
              </p>
            ) : (
              <label className="block space-y-1.5 text-sm font-medium">
                选择规则片段
                <select
                  className="h-9 w-full rounded-md border border-input bg-background px-2 text-sm font-normal"
                  value={selectedFragmentKey}
                  onChange={(event) =>
                    setSelectedFragmentKey(event.target.value)
                  }
                >
                  <option value="">请选择规则片段</option>
                  {fragmentCatalog.map(({ app, prompt }) => (
                    <option
                      key={`${app}:${prompt.id}`}
                      value={`${app}:${prompt.id}`}
                    >
                      {prompt.name} · {app}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialog(null)}>
              取消
            </Button>
            <Button
              onClick={() => void linkSelectedRule()}
              disabled={!selectedFragmentKey || saving}
            >
              {saving ? "关联中…" : "关联到当前项目"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={dialog === "create"}
        onOpenChange={(open) => !open && setDialog(null)}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>
              {parentPromptOptions.length === 0 ? "新建规则包" : "新建规则片段"}
            </DialogTitle>
            <DialogDescription>
              保存后会自动关联到当前项目；未选择归属 Prompt 时会同步创建规则包。
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 overflow-y-auto px-6 py-5 text-sm">
            <label className="grid gap-1.5 font-medium">
              规则名称
              <input
                className="h-9 rounded-md border border-input bg-background px-2 text-sm font-normal"
                value={ruleName}
                onChange={(event) => setRuleName(event.target.value)}
                placeholder="例如：前端组件约束"
              />
            </label>
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="grid gap-1.5 font-medium">
                规则库工具
                <select
                  className="h-9 rounded-md border border-input bg-background px-2 text-sm font-normal"
                  value={ruleApp}
                  onChange={(event) => {
                    const app = event.target.value as AppId;
                    setRuleApp(app);
                    setRuleTargets(app);
                    setParentPromptId("");
                  }}
                >
                  {PROMPT_APPS.map((app) => (
                    <option key={app} value={app}>
                      {app}
                    </option>
                  ))}
                </select>
              </label>
              <label className="grid gap-1.5 font-medium">
                归属 Prompt
                <select
                  className="h-9 rounded-md border border-input bg-background px-2 text-sm font-normal"
                  value={parentPromptId}
                  onChange={(event) => setParentPromptId(event.target.value)}
                >
                  <option value="">新建规则包</option>
                  {parentPromptOptions.map((prompt) => (
                    <option key={prompt.id} value={prompt.id}>
                      {prompt.name}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="grid gap-4 sm:grid-cols-3">
              <label className="grid gap-1.5 font-medium">
                目标工具
                <input
                  className="h-9 rounded-md border border-input bg-background px-2 text-sm font-normal"
                  value={ruleTargets}
                  onChange={(event) => setRuleTargets(event.target.value)}
                  placeholder="claude, codex 或 *"
                />
              </label>
              <label className="grid gap-1.5 font-medium">
                优先级
                <input
                  className="h-9 rounded-md border border-input bg-background px-2 text-sm font-normal"
                  type="number"
                  value={rulePriority}
                  onChange={(event) => setRulePriority(event.target.value)}
                />
              </label>
              <label className="grid gap-1.5 font-medium">
                文件 Glob
                <input
                  className="h-9 rounded-md border border-input bg-background px-2 text-sm font-normal"
                  value={ruleGlobs}
                  onChange={(event) => setRuleGlobs(event.target.value)}
                  placeholder='["src/**/*.tsx"]'
                />
              </label>
            </div>
            <label className="grid gap-1.5 font-medium">
              规则内容
              <textarea
                className="min-h-40 rounded-md border border-input bg-background p-2 text-sm font-normal"
                value={ruleContent}
                onChange={(event) => setRuleContent(event.target.value)}
                placeholder="描述在本项目中需要持续遵守的开发规则。"
              />
            </label>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialog(null)}>
              取消
            </Button>
            <Button
              onClick={() => void createAndLinkRule()}
              disabled={!ruleName.trim() || !ruleContent.trim() || saving}
            >
              {saving ? "创建中…" : "创建并关联"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={syncPlan !== null}
        onOpenChange={(open) => !open && setSyncPlan(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>项目资产同步预览</DialogTitle>
            <DialogDescription>
              下列计划尚未写入。确认后系统将按照受保护文件策略同步项目资产。
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-64 space-y-2 overflow-y-auto px-6 py-5 text-xs">
            {syncPlan?.steps.map((step) => (
              <div
                key={step.expectationId}
                className="rounded-md border border-border/60 bg-muted/20 px-3 py-2"
              >
                <p className="font-medium">
                  {step.assetType} · {step.targetApp}
                </p>
                <p className="mt-0.5 text-muted-foreground">
                  {step.action} ·{" "}
                  {step.managedPaths.join(", ") || "由适配器决定路径"}
                </p>
              </div>
            ))}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSyncPlan(null)}>
              取消
            </Button>
            <Button onClick={() => void applySyncPlan()} disabled={saving}>
              {saving ? "同步中…" : "确认同步"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
