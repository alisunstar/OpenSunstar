import { useCallback, useEffect, useState } from "react";
import {
  projectsApi,
  type Project,
  type ProjectConfigLink,
  type ProjectPromptLink,
} from "@/lib/api/projects";
import type {
  ExtendedProjectAssetType,
  ProjectAssetLink,
} from "@/types/projectAsset";

const EXTENDED_TYPES: ExtendedProjectAssetType[] = [
  "command",
  "hook",
  "ignore",
  "permission",
  "subagent",
];

const EMPTY_EXTENDED: Record<ExtendedProjectAssetType, ProjectAssetLink[]> = {
  command: [],
  hook: [],
  ignore: [],
  permission: [],
  subagent: [],
};

/**
 * 项目 8 类 AI 资产关联（SSOT：`project_asset_links`）
 *
 * 统一加载 MCP / Skills / Prompts 与扩展 5 类资产，单次 refresh、单一 loading。
 */
export function useProjectAssets(projectId: string | null) {
  const [project, setProject] = useState<Project | null>(null);
  const [mcpLinks, setMcpLinks] = useState<ProjectConfigLink[]>([]);
  const [skillLinks, setSkillLinks] = useState<ProjectConfigLink[]>([]);
  const [promptLinks, setPromptLinks] = useState<ProjectPromptLink[]>([]);
  const [linksByType, setLinksByType] =
    useState<Record<ExtendedProjectAssetType, ProjectAssetLink[]>>(EMPTY_EXTENDED);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setProject(null);
      setMcpLinks([]);
      setSkillLinks([]);
      setPromptLinks([]);
      setLinksByType(EMPTY_EXTENDED);
      return;
    }

    setLoading(true);
    try {
      const [proj, mcp, skills, prompts, ...extendedResults] = await Promise.all([
        projectsApi.getById(projectId),
        projectsApi.getMcpServers(projectId),
        projectsApi.getSkills(projectId),
        projectsApi.getPrompts(projectId),
        ...EXTENDED_TYPES.map((type) => projectsApi.getAssetLinks(projectId, type)),
      ]);

      setProject(proj);
      setMcpLinks(mcp);
      setSkillLinks(skills);
      setPromptLinks(prompts);

      const next: Record<ExtendedProjectAssetType, ProjectAssetLink[]> = {
        ...EMPTY_EXTENDED,
      };
      EXTENDED_TYPES.forEach((type, index) => {
        next[type] = extendedResults[index] ?? [];
      });
      setLinksByType(next);
    } catch (err) {
      console.error("[useProjectAssets] load failed", err);
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

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

  const linkExtended = useCallback(
    async (assetType: ExtendedProjectAssetType, assetId: string) => {
      if (!projectId) return;
      await projectsApi.linkAsset(projectId, assetType, assetId, true);
      await refresh();
    },
    [projectId, refresh],
  );

  const unlinkExtended = useCallback(
    async (assetType: ExtendedProjectAssetType, assetId: string) => {
      if (!projectId) return;
      await projectsApi.unlinkAsset(projectId, assetType, assetId);
      await refresh();
    },
    [projectId, refresh],
  );

  const isLinked = useCallback(
    (assetType: ExtendedProjectAssetType, assetId: string) =>
      linksByType[assetType].some((l) => l.asset_id === assetId && l.enabled),
    [linksByType],
  );

  const enabledCount = useCallback(
    (assetType: ExtendedProjectAssetType) =>
      linksByType[assetType].filter((l) => l.enabled).length,
    [linksByType],
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
    extended: {
      linksByType,
      link: linkExtended,
      unlink: unlinkExtended,
      isLinked,
      enabledCount,
    },
  };
}
