import { useState, useEffect, useCallback } from "react";
import {
  projectsApi,
  type Project,
  type ProjectConfigLink,
  type ProjectPromptLink,
} from "@/lib/api/projects";

/**
 * 项目级配置管理 Hook
 *
 * 管理选定项目与 MCP/Skills/Prompts 的关联关系。
 */
export function useProjectConfig(projectId: string | null) {
  const [project, setProject] = useState<Project | null>(null);
  const [mcpLinks, setMcpLinks] = useState<ProjectConfigLink[]>([]);
  const [skillLinks, setSkillLinks] = useState<ProjectConfigLink[]>([]);
  const [promptLinks, setPromptLinks] = useState<ProjectPromptLink[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setProject(null);
      setMcpLinks([]);
      setSkillLinks([]);
      setPromptLinks([]);
      return;
    }

    setLoading(true);
    try {
      const [proj, mcp, skills, prompts] = await Promise.all([
        projectsApi.getById(projectId),
        projectsApi.getMcpServers(projectId),
        projectsApi.getSkills(projectId),
        projectsApi.getPrompts(projectId),
      ]);
      setProject(proj);
      setMcpLinks(mcp);
      setSkillLinks(skills);
      setPromptLinks(prompts);
    } catch (err) {
      console.error("Failed to load project config:", err);
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // MCP operations
  const linkMcp = useCallback(
    async (mcpServerId: string, enabled = true) => {
      if (!projectId) return;
      await projectsApi.linkMcpServer(projectId, mcpServerId, enabled);
      await refresh();
    },
    [projectId, refresh],
  );

  const unlinkMcp = useCallback(
    async (mcpServerId: string) => {
      if (!projectId) return;
      await projectsApi.unlinkMcpServer(projectId, mcpServerId);
      await refresh();
    },
    [projectId, refresh],
  );

  const setMcpServers = useCallback(
    async (ids: string[]) => {
      if (!projectId) return;
      await projectsApi.setMcpServers(projectId, ids);
      await refresh();
    },
    [projectId, refresh],
  );

  // Skills operations
  const linkSkill = useCallback(
    async (skillId: string, enabled = true) => {
      if (!projectId) return;
      await projectsApi.linkSkill(projectId, skillId, enabled);
      await refresh();
    },
    [projectId, refresh],
  );

  const unlinkSkill = useCallback(
    async (skillId: string) => {
      if (!projectId) return;
      await projectsApi.unlinkSkill(projectId, skillId);
      await refresh();
    },
    [projectId, refresh],
  );

  const setSkills = useCallback(
    async (ids: string[]) => {
      if (!projectId) return;
      await projectsApi.setSkills(projectId, ids);
      await refresh();
    },
    [projectId, refresh],
  );

  // Prompts operations
  const linkPrompt = useCallback(
    async (promptId: string, appType: string, enabled = true) => {
      if (!projectId) return;
      await projectsApi.linkPrompt(projectId, promptId, appType, enabled);
      await refresh();
    },
    [projectId, refresh],
  );

  const unlinkPrompt = useCallback(
    async (promptId: string, appType: string) => {
      if (!projectId) return;
      await projectsApi.unlinkPrompt(projectId, promptId, appType);
      await refresh();
    },
    [projectId, refresh],
  );

  const setPrompts = useCallback(
    async (prompts: [string, string][]) => {
      if (!projectId) return;
      await projectsApi.setPrompts(projectId, prompts);
      await refresh();
    },
    [projectId, refresh],
  );

  return {
    project,
    loading,
    refresh,
    mcp: {
      links: mcpLinks,
      link: linkMcp,
      unlink: unlinkMcp,
      setAll: setMcpServers,
    },
    skills: {
      links: skillLinks,
      link: linkSkill,
      unlink: unlinkSkill,
      setAll: setSkills,
    },
    prompts: {
      links: promptLinks,
      link: linkPrompt,
      unlink: unlinkPrompt,
      setAll: setPrompts,
    },
  };
}
