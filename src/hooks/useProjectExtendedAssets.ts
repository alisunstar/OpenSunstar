import { useCallback, useEffect, useState } from "react";
import { projectsApi } from "@/lib/api/projects";
import type { ExtendedProjectAssetType } from "@/types/projectAsset";
import type { ProjectAssetLink } from "@/types/projectAsset";

const EXTENDED_TYPES: ExtendedProjectAssetType[] = [
  "command",
  "hook",
  "ignore",
  "permission",
  "subagent",
];

/**
 * 扩展 5 类项目资产关联（Commands / Hooks / Ignore / Permissions / Subagents）
 * MCP / Skills / Prompts 仍使用 useProjectConfig + 旧三表。
 */
export function useProjectExtendedAssets(projectId: string | null) {
  const [linksByType, setLinksByType] = useState<
    Record<ExtendedProjectAssetType, ProjectAssetLink[]>
  >({
    command: [],
    hook: [],
    ignore: [],
    permission: [],
    subagent: [],
  });
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setLinksByType({
        command: [],
        hook: [],
        ignore: [],
        permission: [],
        subagent: [],
      });
      return;
    }
    setLoading(true);
    try {
      const results = await Promise.all(
        EXTENDED_TYPES.map((type) =>
          projectsApi.getAssetLinks(projectId, type).then((links) => ({
            type,
            links,
          })),
        ),
      );
      const next: Record<ExtendedProjectAssetType, ProjectAssetLink[]> = {
        command: [],
        hook: [],
        ignore: [],
        permission: [],
        subagent: [],
      };
      for (const { type, links } of results) {
        next[type] = links;
      }
      setLinksByType(next);
    } catch (err) {
      console.warn("[useProjectExtendedAssets] load failed", err);
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const link = useCallback(
    async (assetType: ExtendedProjectAssetType, assetId: string) => {
      if (!projectId) return;
      await projectsApi.linkAsset(projectId, assetType, assetId, true);
      await refresh();
    },
    [projectId, refresh],
  );

  const unlink = useCallback(
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
    loading,
    linksByType,
    refresh,
    link,
    unlink,
    isLinked,
    enabledCount,
  };
}
