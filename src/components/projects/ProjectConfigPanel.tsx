import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Server, Sparkles, MessageSquare, Loader2, ExternalLink } from "lucide-react";
import { toast } from "sonner";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { useProjectConfig } from "@/hooks/useProjectConfig";
import { mcpApi } from "@/lib/api/mcp";
import { skillsApi, type InstalledSkill } from "@/lib/api/skills";
import { promptsApi, type Prompt } from "@/lib/api/prompts";
import type { McpServersMap } from "@/types";
import type { PageView } from "@/App";
import type { ProjectAssetSection } from "@/lib/readinessActions";

interface ProjectConfigPanelProps {
  projectId: string;
  scrollToSection?: ProjectAssetSection | null;
  onConfigChanged?: () => void;
  onNavigateToGlobal?: (view: PageView) => void;
}

/**
 * 项目级配置面板
 *
 * 允许用户为特定项目选择性地启用/禁用 MCP 服务器、Skills 和 Prompts。
 */
export function ProjectConfigPanel({
  projectId,
  scrollToSection,
  onConfigChanged,
  onNavigateToGlobal,
}: ProjectConfigPanelProps) {
  const { t } = useTranslation();
  const { loading, mcp, skills, prompts } = useProjectConfig(projectId);
  const mcpRef = useRef<HTMLElement>(null);
  const skillsRef = useRef<HTMLElement>(null);
  const promptsRef = useRef<HTMLElement>(null);

  // 全局可用列表
  const [allMcp, setAllMcp] = useState<McpServersMap>({});
  const [allSkills, setAllSkills] = useState<InstalledSkill[]>([]);
  const [allPrompts, setAllPrompts] = useState<Record<string, Prompt>>({});
  const [globalLoading, setGlobalLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setGlobalLoading(true);
      try {
        const [mcpData, skillsData, promptsData] = await Promise.all([
          mcpApi.getAllServers(),
          skillsApi.getInstalled(),
          promptsApi.getPrompts("claude"),
        ]);
        if (!cancelled) {
          setAllMcp(mcpData);
          setAllSkills(skillsData);
          setAllPrompts(promptsData);
        }
      } catch (err) {
        console.error("Failed to load global config:", err);
      } finally {
        if (!cancelled) setGlobalLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    if (!scrollToSection) return;
    const target =
      scrollToSection === "mcp"
        ? mcpRef.current
        : scrollToSection === "skills"
          ? skillsRef.current
          : promptsRef.current;
    target?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [scrollToSection, loading, globalLoading]);

  const notifyChanged = () => {
    onConfigChanged?.();
  };

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

  const mcpEntries = Object.entries(allMcp);
  const promptEntries = Object.entries(allPrompts);

  const handleMcpToggle = async (serverId: string, checked: boolean) => {
    try {
      if (checked) {
        await mcp.link(serverId);
      } else {
        await mcp.unlink(serverId);
      }
      notifyChanged();
    } catch {
      toast.error("操作失败");
    }
  };

  const handleSkillToggle = async (skillId: string, checked: boolean) => {
    try {
      if (checked) {
        await skills.link(skillId);
      } else {
        await skills.unlink(skillId);
      }
      notifyChanged();
    } catch {
      toast.error("操作失败");
    }
  };

  const handlePromptToggle = async (
    promptId: string,
    appType: string,
    checked: boolean,
  ) => {
    try {
      if (checked) {
        await prompts.link(promptId, appType);
      } else {
        await prompts.unlink(promptId, appType);
      }
      notifyChanged();
    } catch {
      toast.error("操作失败");
    }
  };

  return (
    <div className="space-y-6">
      <p className="text-xs text-muted-foreground leading-relaxed">
        {t("projectConfig.hint", {
          defaultValue:
            "为当前项目勾选要启用的 MCP、Skills 与 Prompts。全局资产库在侧栏 Agent 配置中维护。",
        })}
      </p>

      {/* MCP Servers */}
      <section ref={mcpRef}>
        <h3 className="flex items-center gap-2 text-sm font-medium mb-3">
          <Server className="w-4 h-4 text-blue-500" />
          {t("projectConfig.mcpServers", {
            defaultValue: "MCP 服务器",
          })}
          <span className="text-xs text-muted-foreground ml-auto">
            {mcp.links.length}/{mcpEntries.length}
          </span>
        </h3>
        {mcpEntries.length === 0 ? (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              {t("projectConfig.noMcp", {
                defaultValue: "暂无 MCP 服务器",
              })}
            </p>
            {onNavigateToGlobal && (
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs"
                onClick={() => onNavigateToGlobal("mcp")}
              >
                <ExternalLink className="w-3 h-3 mr-1" />
                {t("projectConfig.goToMcp", {
                  defaultValue: "前往 MCP 管理",
                })}
              </Button>
            )}
          </div>
        ) : (
          <div className="space-y-2">
            {mcpEntries.map(([id, server]) => (
              <div
                key={id}
                className="flex items-center justify-between px-3 py-2 rounded-md border border-border/50 bg-card/50"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{server.name}</p>
                  {server.description && (
                    <p className="text-xs text-muted-foreground truncate">
                      {server.description}
                    </p>
                  )}
                </div>
                <Switch
                  checked={mcpLinkedIds.has(id)}
                  onCheckedChange={(checked) => handleMcpToggle(id, checked)}
                />
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Skills */}
      <section ref={skillsRef}>
        <h3 className="flex items-center gap-2 text-sm font-medium mb-3">
          <Sparkles className="w-4 h-4 text-amber-500" />
          {t("projectConfig.skills", { defaultValue: "Skills" })}
          <span className="text-xs text-muted-foreground ml-auto">
            {skills.links.length}/{allSkills.length}
          </span>
        </h3>
        {allSkills.length === 0 ? (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              {t("projectConfig.noSkills", {
                defaultValue: "暂无已安装的 Skills",
              })}
            </p>
            {onNavigateToGlobal && (
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs"
                onClick={() => onNavigateToGlobal("skills")}
              >
                <ExternalLink className="w-3 h-3 mr-1" />
                {t("projectConfig.goToSkills", {
                  defaultValue: "前往 Skills 管理",
                })}
              </Button>
            )}
          </div>
        ) : (
          <div className="space-y-2">
            {allSkills.map((skill) => (
              <div
                key={skill.id}
                className="flex items-center justify-between px-3 py-2 rounded-md border border-border/50 bg-card/50"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{skill.name}</p>
                  {skill.description && (
                    <p className="text-xs text-muted-foreground truncate">
                      {skill.description}
                    </p>
                  )}
                </div>
                <Switch
                  checked={skillLinkedIds.has(skill.id)}
                  onCheckedChange={(checked) =>
                    handleSkillToggle(skill.id, checked)
                  }
                />
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Prompts */}
      <section ref={promptsRef}>
        <h3 className="flex items-center gap-2 text-sm font-medium mb-3">
          <MessageSquare className="w-4 h-4 text-green-500" />
          {t("projectConfig.prompts", { defaultValue: "Prompts" })}
          <span className="text-xs text-muted-foreground ml-auto">
            {prompts.links.length}/{promptEntries.length}
          </span>
        </h3>
        {promptEntries.length === 0 ? (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              {t("projectConfig.noPrompts", {
                defaultValue: "暂无 Prompts",
              })}
            </p>
            {onNavigateToGlobal && (
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs"
                onClick={() => onNavigateToGlobal("prompts")}
              >
                <ExternalLink className="w-3 h-3 mr-1" />
                {t("projectConfig.goToPrompts", {
                  defaultValue: "前往 Prompts 管理",
                })}
              </Button>
            )}
          </div>
        ) : (
          <div className="space-y-2">
            {promptEntries.map(([id, prompt]) => (
              <div
                key={id}
                className="flex items-center justify-between px-3 py-2 rounded-md border border-border/50 bg-card/50"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{prompt.name}</p>
                  {prompt.description && (
                    <p className="text-xs text-muted-foreground truncate">
                      {prompt.description}
                    </p>
                  )}
                </div>
                <Switch
                  checked={promptLinkedIds.has(`${id}:claude`)}
                  onCheckedChange={(checked) =>
                    handlePromptToggle(id, "claude", checked)
                  }
                />
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
