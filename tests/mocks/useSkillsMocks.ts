import { vi } from "vitest";
import type { SkillsShSearchResult } from "@/lib/api/skills";

type SkillsHookMock = Record<string, unknown>;

const emptyQueryResult = () => ({
  data: undefined,
  isLoading: false,
  isFetching: false,
  refetch: vi.fn(),
});

const emptyListQueryResult = () => ({
  data: [] as unknown[],
  isLoading: false,
  isFetching: false,
  refetch: vi.fn(),
});

const mutationResult = () => ({
  mutateAsync: vi.fn(),
  isPending: false,
});

/** Baseline stubs for every hook exported from `@/hooks/useSkills`. */
export function createSkillsHooksMock(
  overrides: SkillsHookMock = {},
): SkillsHookMock {
  return {
    useInstalledSkills: () => ({ data: [], isLoading: false }),
    useSkillBackups: () => emptyListQueryResult(),
    useDeleteSkillBackup: () => mutationResult(),
    useDiscoverableSkills: () => emptyListQueryResult(),
    useInstallSkill: () => mutationResult(),
    useInstallClawHubSkill: () => mutationResult(),
    useUninstallSkill: () => mutationResult(),
    useRestoreSkillBackup: () => mutationResult(),
    useToggleSkillApp: () => mutationResult(),
    useBatchToggleSkillApp: () => mutationResult(),
    useScanUnmanagedSkills: () => ({
      data: [],
      refetch: vi.fn(),
      isLoading: false,
      isFetching: false,
    }),
    useImportSkillsFromApps: () => mutationResult(),
    useSkillRepos: () => ({ data: [], refetch: vi.fn() }),
    useAddSkillRepo: () => mutationResult(),
    useRemoveSkillRepo: () => mutationResult(),
    useToggleSkillRepo: () => mutationResult(),
    useInstallSkillsFromZip: () => mutationResult(),
    useCheckSkillUpdates: () => emptyListQueryResult(),
    useUpdateSkill: () => mutationResult(),
    useSearchSkillsSh: () => emptyQueryResult(),
    useSearchClawHub: () => emptyQueryResult(),
    useSearchModelScope: () => emptyQueryResult(),
    ...overrides,
  };
}

export type SkillsShSearchCache = Map<
  string,
  { data: SkillsShSearchResult | undefined; isLoading: boolean; isFetching: boolean }
>;

/** Stable skills.sh search mock — avoids infinite re-render loops in SkillsPage. */
export function createSearchSkillsShMock(cache: SkillsShSearchCache) {
  return (query: string, _limit: number, offset: number) => {
    const cached = cache.get(`${query}:${offset}`);
    if (cached) return cached;
    return { data: undefined, isLoading: false, isFetching: false };
  };
}

export function setSkillsShSearchResult(
  cache: SkillsShSearchCache,
  query: string,
  offset: number,
  result: SkillsShSearchResult | undefined,
) {
  cache.set(`${query}:${offset}`, {
    data: result,
    isLoading: false,
    isFetching: false,
  });
}
