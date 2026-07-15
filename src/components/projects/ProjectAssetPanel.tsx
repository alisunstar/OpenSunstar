import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Server,
  Sparkles,
  MessageSquare,
  Loader2,
  ExternalLink,
  Terminal,
  Webhook,
  EyeOff,
  Shield,
  Bot,
  Info,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useProjectAssets } from "@/hooks/useProjectAssets";
import { mcpApi } from "@/lib/api/mcp";
import { skillsApi, type InstalledSkill } from "@/lib/api/skills";
import { promptsApi } from "@/lib/api/prompts";
import type { AppId } from "@/lib/api/types";
import { commandsApi, type Command } from "@/lib/api/commands";
import { hooksApi, type Hook } from "@/lib/api/hooks";
import { ignoreApi, type IgnoreRule } from "@/lib/api/ignore";
import { permissionsApi, type ToolPermission } from "@/lib/api/permissions";
import { agentsApi, type Agent } from "@/lib/api/agents";
import type { McpServersMap } from "@/types";
import type { PageView } from "@/App";
import type { ProjectAssetSection } from "@/types/projectAsset";
import type { ExtendedProjectAssetType } from "@/types/projectAsset";
import {
  ProjectAssetAppSupportChips,
  ProjectAssetEnableSwitch,
  ProjectAssetSupportTooltipProvider,
} from "./ProjectAssetSupport";
import { ProjectAssetHealthSummary } from "./ProjectAssetHealthSummary";
import {
  summarizeAssetSupport,
  PROMPT_SYNC_APP_IDS,
} from "@/lib/projectAssets/assetAppSupport";
import { ProjectEnvironmentSnapshotPanel } from "./ProjectEnvironmentSnapshotPanel";

interface ProjectAssetPanelProps {
  projectId: string;
  scrollToSection?: ProjectAssetSection | null;
  onConfigChanged?: () => void;
  onNavigateToGlobal?: (view: PageView) => void;
}

const SECTION_ICON: Record<ProjectAssetSection, typeof Server> = {
  mcp: Server,
  skill: Sparkles,
  prompt: MessageSquare,
  command: Terminal,
  hook: Webhook,
  ignore: EyeOff,
  permission: Shield,
  subagent: Bot,
};

const GLOBAL_VIEW: Partial<Record<ProjectAssetSection, PageView>> = {
  mcp: "mcp",
  skill: "skills",
  prompt: "prompts",
  command: "commands",
  hook: "hooks",
  ignore: "ignore",
  permission: "permissions",
  subagent: "agents",
};

export function ProjectAssetPanel({
  projectId,
  scrollToSection,
  onConfigChanged,
  onNavigateToGlobal,
}: ProjectAssetPanelProps) {
  const { t } = useTranslation();
  const { loading, project, mcp, skills, prompts, extended } =
    useProjectAssets(projectId);

  const sectionRefs = useRef<Partial<Record<ProjectAssetSection, HTMLElement>>>(
    {},
  );

  const [allMcp, setAllMcp] = useState<McpServersMap>({});
  const [allSkills, setAllSkills] = useState<InstalledSkill[]>([]);
  const [promptCatalog, setPromptCatalog] = useState<
    { id: string; name: string; appType: AppId }[]
  >([]);
  const [allCommands, setAllCommands] = useState<Command[]>([]);
  const [allHooks, setAllHooks] = useState<Hook[]>([]);
  const [allIgnore, setAllIgnore] = useState<IgnoreRule[]>([]);
  const [allPermissions, setAllPermissions] = useState<ToolPermission[]>([]);
  const [allAgents, setAllAgents] = useState<Agent[]>([]);
  const [globalLoading, setGlobalLoading] = useState(true);
  const [mcpRuntimeStatus, setMcpRuntimeStatus] = useState<string | null>(null);
  const [probingMcp, setProbingMcp] = useState(false);

  const probeMcpRuntime = async (app: "claude" | "gemini") => {
    setProbingMcp(true);
    try {
      const result = await mcpApi.probeProjectRuntime(projectId, app);
      setMcpRuntimeStatus(
        `${app === "claude" ? "Claude Code" : "Gemini CLI"}：${result.summary}`,
      );
    } catch (error) {
      setMcpRuntimeStatus(String(error));
    } finally {
      setProbingMcp(false);
    }
  };

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setGlobalLoading(true);
      try {
        const [
          mcpData,
          skillsData,
          commandsData,
          hooksData,
          ignoreData,
          permsData,
          agentsData,
          ...promptBundles
        ] = await Promise.all([
          mcpApi.getAllServers(),
          skillsApi.getInstalled(),
          commandsApi.getAll(),
          hooksApi.getAll(),
          ignoreApi.getAll(),
          permissionsApi.getAll(),
          agentsApi.getAll(),
          ...PROMPT_SYNC_APP_IDS.map(async (app) => {
            const data = await promptsApi.getPrompts(app);
            return Object.entries(data).map(([id, prompt]) => ({
              id,
              name: prompt.name,
              appType: app,
            }));
          }),
        ]);
        const promptItems = promptBundles.flat();
        if (!cancelled) {
          setAllMcp(mcpData);
          setAllSkills(skillsData);
          setPromptCatalog(promptItems);
          setAllCommands(Object.values(commandsData));
          setAllHooks(hooksData);
          setAllIgnore(ignoreData);
          setAllPermissions(permsData);
          setAllAgents(Object.values(agentsData));
        }
      } catch (err) {
        console.error("Failed to load global assets:", err);
      } finally {
        if (!cancelled) setGlobalLoading(false);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!scrollToSection) return;
    sectionRefs.current[scrollToSection]?.scrollIntoView({
      behavior: "smooth",
      block: "start",
    });
  }, [scrollToSection, loading, globalLoading]);

  const promptRows = useMemo(() => {
    const byKey = new Map<
      string,
      { id: string; name: string; appType: AppId }
    >();
    for (const item of promptCatalog) {
      byKey.set(`${item.appType}:${item.id}`, item);
    }
    for (const link of prompts.links) {
      const key = `${link.prompt_app_type}:${link.prompt_id}`;
      if (!byKey.has(key)) {
        byKey.set(key, {
          id: link.prompt_id,
          name: link.prompt_id,
          appType: link.prompt_app_type as AppId,
        });
      }
    }
    const rows = Array.from(byKey.values());
    const target = project?.target_app;
    if (target) {
      rows.sort((a, b) => {
        if (a.appType === target && b.appType !== target) return -1;
        if (b.appType === target && a.appType !== target) return 1;
        return (
          a.appType.localeCompare(b.appType) || a.name.localeCompare(b.name)
        );
      });
    } else {
      rows.sort(
        (a, b) =>
          a.appType.localeCompare(b.appType) || a.name.localeCompare(b.name),
      );
    }
    return rows;
  }, [promptCatalog, prompts.links, project?.target_app]);

  const notifyChanged = () => onConfigChanged?.();

  if (loading || globalLoading) {
    return (
      <div className="flex items-center justify-center py-8 text-muted-foreground">
        <Loader2 className="w-4 h-4 animate-spin mr-2" />
        {t("common.loading", { defaultValue: "加载中..." })}
      </div>
    );
  }

  const mcpLinkedIds = new Set(mcp.links.map((l) => l.config_id));
  const skillLinkedIds = new Set(skills.links.map((l) => l.config_id));
  const promptLinkedIds = new Set(
    prompts.links.map((l) => `${l.prompt_id}:${l.prompt_app_type}`),
  );

  const setSectionRef =
    (section: ProjectAssetSection) => (el: HTMLElement | null) => {
      if (el) sectionRefs.current[section] = el;
    };

  const renderSectionHeader = (
    section: ProjectAssetSection,
    title: string,
    linked: number,
    total: number,
  ) => {
    const Icon = SECTION_ICON[section];
    const support = summarizeAssetSupport(section);
    return (
      <div className="mb-3 space-y-2">
        <h3 className="flex items-center gap-2 text-sm font-medium">
          <Icon className="w-4 h-4 shrink-0 text-primary" />
          {title}
          <span className="text-xs text-muted-foreground ml-auto tabular-nums">
            {linked}/{total}
          </span>
        </h3>
        <ProjectAssetAppSupportChips assetType={section} />
        {support.hasPartial && (
          <p className="text-[11px] text-amber-600/90 dark:text-amber-400/90">
            {t("projectAssets.partialHint", {
              defaultValue: "部分 CLI 支持有限制，悬停标签查看说明",
            })}
          </p>
        )}
      </div>
    );
  };

  const renderGlobalBaseline = (
    _section: "ignore" | "permission",
    globalCount: number,
    projectCount: number,
  ) => {
    if (projectCount > 0 || globalCount === 0) return null;
    return (
      <div className="mb-3 rounded-lg border border-blue-500/20 bg-blue-500/5 px-3 py-2 text-[11px] text-muted-foreground leading-relaxed">
        <Info className="inline h-3.5 w-3.5 mr-1 text-blue-500 align-text-bottom" />
        {t("projectAssets.globalBaselineHint", {
          count: globalCount,
          defaultValue: `全局库已有 ${globalCount} 条规则作为安全基线；可为当前项目单独勾选启用子集。`,
        })}
      </div>
    );
  };

  const handleExtendedToggle = async (
    type: ExtendedProjectAssetType,
    id: string,
    checked: boolean,
  ) => {
    try {
      if (checked) await extended.link(type, id);
      else await extended.unlink(type, id);
      notifyChanged();
    } catch {
      toast.error(t("common.error", { defaultValue: "操作失败" }));
    }
  };

  const renderExtendedList = (
    section: ExtendedProjectAssetType,
    items: { id: string; title: string; subtitle?: string }[],
    emptyKey: string,
    emptyDefault: string,
    goToKey: string,
    goToDefault: string,
  ) => (
    <section ref={setSectionRef(section)}>
      {renderSectionHeader(
        section,
        t(
          `projectConfig.${section === "subagent" ? "subagents" : section + "s"}`,
          {
            defaultValue:
              section === "command"
                ? "Commands"
                : section === "hook"
                  ? "Hooks"
                  : section === "ignore"
                    ? "Ignore"
                    : section === "permission"
                      ? "Permissions"
                      : "Subagents",
          },
        ),
        extended.enabledCount(section),
        items.length,
      )}
      {(section === "ignore" || section === "permission") &&
        renderGlobalBaseline(
          section,
          section === "ignore" ? allIgnore.length : allPermissions.length,
          extended.enabledCount(section),
        )}
      {items.length === 0 ? (
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground">
            {t(emptyKey, { defaultValue: emptyDefault })}
          </p>
          {onNavigateToGlobal && GLOBAL_VIEW[section] && (
            <Button
              variant="outline"
              size="sm"
              className="h-8 text-xs"
              onClick={() => onNavigateToGlobal(GLOBAL_VIEW[section]!)}
            >
              <ExternalLink className="w-3 h-3 mr-1" />
              {t(goToKey, { defaultValue: goToDefault })}
            </Button>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          {items.map((item) => (
            <div
              key={item.id}
              className="flex items-center justify-between gap-2 px-3 py-2 rounded-md border border-border/50 bg-card/50"
            >
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium truncate">{item.title}</p>
                {item.subtitle && (
                  <p className="text-xs text-muted-foreground truncate">
                    {item.subtitle}
                  </p>
                )}
              </div>
              <ProjectAssetEnableSwitch
                assetType={section}
                checked={extended.isLinked(section, item.id)}
                onCheckedChange={(checked) =>
                  void handleExtendedToggle(section, item.id, checked)
                }
              />
            </div>
          ))}
        </div>
      )}
    </section>
  );

  return (
    <ProjectAssetSupportTooltipProvider>
      <div className="space-y-6">
        <ProjectAssetHealthSummary projectId={projectId} />
        <ProjectEnvironmentSnapshotPanel
          projectId={projectId}
          onApplied={notifyChanged}
        />
        <div className="rounded-lg border border-border/50 bg-muted/15 px-3 py-2.5 space-y-1.5">
          <p className="text-xs text-muted-foreground leading-relaxed">
            {t("projectAssets.hint", {
              defaultValue:
                "为当前项目勾选要启用的 AI 资产子集。关联仅写入 OpenSunstar 本地库，不会自动修改项目仓库文件；写入各 CLI 仍由侧栏全局资产管理。",
            })}
          </p>
        </div>

        {/* MCP */}
        <section ref={setSectionRef("mcp")}>
          {renderSectionHeader(
            "mcp",
            t("projectConfig.mcpServers", { defaultValue: "MCP 服务器" }),
            mcp.links.length,
            Object.keys(allMcp).length,
          )}
          {Object.keys(allMcp).length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {t("projectConfig.noMcp", { defaultValue: "暂无 MCP 服务器" })}
            </p>
          ) : (
            <div className="space-y-2">
              <div className="flex flex-wrap items-center gap-2 rounded-md bg-muted/30 px-3 py-2">
                <span className="text-xs text-muted-foreground">
                  运行时读取验证：
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={probingMcp}
                  onClick={() => void probeMcpRuntime("claude")}
                >
                  验证 Claude Code
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={probingMcp}
                  onClick={() => void probeMcpRuntime("gemini")}
                >
                  验证 Gemini CLI
                </Button>
                {mcpRuntimeStatus && (
                  <span className="text-xs text-muted-foreground">
                    {mcpRuntimeStatus}
                  </span>
                )}
              </div>
              {Object.entries(allMcp).map(([id, server]) => (
                <div
                  key={id}
                  className="flex items-center justify-between px-3 py-2 rounded-md border border-border/50 bg-card/50"
                >
                  <div className="min-w-0">
                    <p className="text-sm font-medium truncate">
                      {server.name}
                    </p>
                  </div>
                  <ProjectAssetEnableSwitch
                    assetType="mcp"
                    checked={mcpLinkedIds.has(id)}
                    onCheckedChange={(checked) => {
                      void (async () => {
                        try {
                          if (checked) await mcp.link(id);
                          else await mcp.unlink(id);
                          notifyChanged();
                        } catch {
                          toast.error("操作失败");
                        }
                      })();
                    }}
                  />
                </div>
              ))}
            </div>
          )}
        </section>

        {/* Skills */}
        <section ref={setSectionRef("skill")}>
          {renderSectionHeader(
            "skill",
            t("projectConfig.skills", { defaultValue: "Skills" }),
            skills.links.length,
            allSkills.length,
          )}
          {allSkills.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {t("projectConfig.noSkills", { defaultValue: "暂无 Skills" })}
            </p>
          ) : (
            <div className="space-y-2">
              {allSkills.map((skill) => (
                <div
                  key={skill.id}
                  className="flex items-center justify-between px-3 py-2 rounded-md border border-border/50 bg-card/50"
                >
                  <p className="text-sm font-medium truncate">{skill.name}</p>
                  <ProjectAssetEnableSwitch
                    assetType="skill"
                    checked={skillLinkedIds.has(skill.id)}
                    onCheckedChange={(checked) => {
                      void (async () => {
                        try {
                          if (checked) await skills.link(skill.id);
                          else await skills.unlink(skill.id);
                          notifyChanged();
                        } catch {
                          toast.error("操作失败");
                        }
                      })();
                    }}
                  />
                </div>
              ))}
            </div>
          )}
        </section>

        {/* Prompts */}
        <section ref={setSectionRef("prompt")}>
          {renderSectionHeader(
            "prompt",
            t("projectConfig.prompts", { defaultValue: "Prompts" }),
            prompts.links.filter((l) => l.enabled).length,
            promptRows.length,
          )}
          {promptRows.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {t("projectConfig.noPrompts", { defaultValue: "暂无 Prompts" })}
            </p>
          ) : (
            <div className="space-y-2">
              {promptRows.map((item) => (
                <div
                  key={`${item.appType}:${item.id}`}
                  className="flex items-center justify-between px-3 py-2 rounded-md border border-border/50 bg-card/50"
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-medium truncate">{item.name}</p>
                    <p className="text-[11px] text-muted-foreground truncate">
                      {item.appType}
                    </p>
                  </div>
                  <ProjectAssetEnableSwitch
                    assetType="prompt"
                    checked={promptLinkedIds.has(`${item.id}:${item.appType}`)}
                    onCheckedChange={(checked) => {
                      void (async () => {
                        try {
                          if (checked)
                            await prompts.link(item.id, item.appType);
                          else await prompts.unlink(item.id, item.appType);
                          notifyChanged();
                        } catch {
                          toast.error("操作失败");
                        }
                      })();
                    }}
                  />
                </div>
              ))}
            </div>
          )}
        </section>

        {renderExtendedList(
          "command",
          allCommands.map((c) => ({
            id: c.id,
            title: c.name,
            subtitle: c.description,
          })),
          "projectConfig.noCommands",
          "暂无 Commands",
          "projectConfig.goToCommands",
          "前往 Commands 管理",
        )}

        {renderExtendedList(
          "hook",
          allHooks.map((h) => ({
            id: h.id,
            title: h.eventType,
            subtitle: h.hookCommand,
          })),
          "projectConfig.noHooks",
          "暂无 Hooks",
          "projectConfig.goToHooks",
          "前往 Hooks 管理",
        )}

        {renderExtendedList(
          "ignore",
          allIgnore.map((r) => ({
            id: r.id,
            title: r.pattern,
            subtitle: r.description,
          })),
          "projectConfig.noIgnore",
          "暂无 Ignore 规则",
          "projectConfig.goToIgnore",
          "前往 Ignore 管理",
        )}

        {renderExtendedList(
          "permission",
          allPermissions.map((p) => ({
            id: p.id,
            title: p.toolPattern,
            subtitle: p.description,
          })),
          "projectConfig.noPermissions",
          "暂无 Permissions",
          "projectConfig.goToPermissions",
          "前往 Permissions 管理",
        )}

        {renderExtendedList(
          "subagent",
          allAgents.map((a) => ({
            id: a.id,
            title: a.name,
            subtitle: a.description,
          })),
          "projectConfig.noSubagents",
          "暂无 Subagents",
          "projectConfig.goToSubagents",
          "前往 Subagents 管理",
        )}
      </div>
    </ProjectAssetSupportTooltipProvider>
  );
}

/** @deprecated 使用 ProjectAssetPanel */
export const ProjectConfigPanel = ProjectAssetPanel;
