import {
  useState,
  useMemo,
  useEffect,
  forwardRef,
  useImperativeHandle,
} from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { RefreshCw, Search, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { SkillCard, type SkillSource } from "./SkillCard";
import { RepoManagerPanel } from "./RepoManagerPanel";
import {
  useDiscoverableSkills,
  useInstalledSkills,
  useInstallSkill,
  useSkillRepos,
  useAddSkillRepo,
  useRemoveSkillRepo,
  useToggleSkillRepo,
  useSearchSkillsSh,
  useSearchClawHub,
  useInstallClawHubSkill,
} from "@/hooks/useSkills";
import type { AppId } from "@/lib/api/types";
import type {
  DiscoverableSkill,
  SkillRepo,
  SkillsShDiscoverableSkill,
  ClawHubSkillStats,
} from "@/lib/api/skills";
import { skillsApi } from "@/lib/api/skills";
import { formatSkillError } from "@/lib/errors/skillErrorParser";

interface SkillsPageProps {
  initialApp?: AppId;
}

export interface SkillsPageHandle {
  refresh: () => void;
  openRepoManager: () => void;
}

type SearchSource = "all" | "repos" | "skillssh" | "clawhub";

const PAGE_SIZE = 20;
/** 初始发现页默认搜索词，用于预取 skills.sh / ClawHub 结果 */
const INITIAL_DISCOVERY_QUERY = "agent";

interface UnifiedDisplaySkill {
  key: string;
  skill: DiscoverableSkill & { installed: boolean };
  source: SkillSource;
  installs?: number;
  stars?: number;
}

/**
 * Skills 发现面板
 * 支持多源并发搜索：GitHub 仓库 / skills.sh / ClawHub
 */
export const SkillsPage = forwardRef<SkillsPageHandle, SkillsPageProps>(
  ({ initialApp = "claude" }, ref) => {
    const { t } = useTranslation();
    const [repoManagerOpen, setRepoManagerOpen] = useState(false);
    const [searchSource, setSearchSource] = useState<SearchSource>("all");
    const [searchInput, setSearchInput] = useState("");
    const [searchQuery, setSearchQuery] = useState("");

    // skills.sh 分页（仅 skillssh 标签页使用）
    const [skillsShOffset, setSkillsShOffset] = useState(0);
    const [accumulatedResults, setAccumulatedResults] = useState<
      SkillsShDiscoverableSkill[]
    >([]);

    // 仓库筛选
    const [filterRepo, setFilterRepo] = useState<string>("all");
    const [filterStatus, setFilterStatus] = useState<
      "all" | "installed" | "uninstalled"
    >("all");

    const currentApp = initialApp;

    // ===== 数据源 Hooks =====
    const {
      data: discoverableSkills,
      isLoading: loadingDiscoverable,
      isFetching: fetchingDiscoverable,
      refetch: refetchDiscoverable,
    } = useDiscoverableSkills();
    const { data: installedSkills } = useInstalledSkills();
    const { data: repos = [], refetch: refetchRepos } = useSkillRepos();

    // skills.sh 搜索（"all" 和 "skillssh" 标签页共用）
    // "all" 标签页无用户搜索时，使用默认发现查询预取结果
    const userHasQuery = searchQuery.trim().length >= 2;
    const effectiveSkillsShQuery =
      searchSource === "all"
        ? userHasQuery
          ? searchQuery
          : INITIAL_DISCOVERY_QUERY
        : searchSource === "skillssh"
          ? searchQuery
          : "";
    const {
      data: skillsShResult,
      isLoading: loadingSkillsSh,
      isFetching: fetchingSkillsSh,
    } = useSearchSkillsSh(
      effectiveSkillsShQuery,
      PAGE_SIZE,
      searchSource === "skillssh" ? skillsShOffset : 0,
    );

    // ClawHub 搜索（"all" 和 "clawhub" 标签页共用）
    const effectiveClawHubQuery =
      searchSource === "all"
        ? userHasQuery
          ? searchQuery
          : INITIAL_DISCOVERY_QUERY
        : searchSource === "clawhub"
          ? searchQuery
          : "";
    const {
      data: clawHubResult,
      isLoading: loadingClawHub,
      isFetching: fetchingClawHub,
    } = useSearchClawHub(effectiveClawHubQuery, PAGE_SIZE);

    // ClawHub 星标/下载量统计缓存
    const [clawHubStats, setClawHubStats] = useState<
      Record<string, ClawHubSkillStats>
    >({});

    // 当 ClawHub 搜索结果变化时，批量获取星标/下载量
    useEffect(() => {
      if (!clawHubResult || clawHubResult.skills.length === 0) return;

      const slugs = clawHubResult.skills
        .map((s) => s.slug)
        .filter((slug) => !clawHubStats[slug]); // 仅获取缺失的

      if (slugs.length === 0) return;

      let cancelled = false;
      skillsApi
        .batchGetClawHubStats(slugs)
        .then((stats) => {
          if (cancelled) return;
          setClawHubStats((prev) => {
            const next = { ...prev };
            stats.forEach((s) => {
              if (s.slug) next[s.slug] = s;
            });
            return next;
          });
        })
        .catch(() => {
          /* 静默失败 */
        });

      return () => {
        cancelled = true;
      };
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [clawHubResult]);

    // skills.sh 累积结果（仅 skillssh 标签页分页用）
    useEffect(() => {
      if (skillsShResult && searchSource === "skillssh") {
        if (skillsShOffset === 0) {
          setAccumulatedResults(skillsShResult.skills);
        } else {
          setAccumulatedResults((prev) => [
            ...prev,
            ...skillsShResult.skills,
          ]);
        }
      }
    }, [skillsShResult, skillsShOffset, searchSource]);

    // 切换标签页时重置状态
    useEffect(() => {
      setSkillsShOffset(0);
      setAccumulatedResults([]);
    }, [searchSource]);

    // ===== 搜索操作 =====
    const handleSearch = () => {
      const trimmed = searchInput.trim();
      if (
        trimmed.length < 2 &&
        searchSource !== "repos" &&
        searchSource !== "all"
      )
        return;
      setSkillsShOffset(0);
      setAccumulatedResults([]);
      setSearchQuery(trimmed);
    };

    // ===== Mutations =====
    const installMutation = useInstallSkill();
    const installClawHubMutation = useInstallClawHubSkill();
    const addRepoMutation = useAddSkillRepo();
    const removeRepoMutation = useRemoveSkillRepo();
    const toggleRepoMutation = useToggleSkillRepo();

    // 已安装 skill 的唯一 key 集合
    const installedKeys = useMemo(() => {
      if (!installedSkills) return new Set<string>();
      return new Set(
        installedSkills.map((s) => {
          const owner = s.repoOwner?.toLowerCase() || "";
          const name = s.repoName?.toLowerCase() || "";
          return `${s.directory.toLowerCase()}:${owner}:${name}`;
        }),
      );
    }, [installedSkills]);

    type DiscoverableSkillItem = DiscoverableSkill & { installed: boolean };

    // 仓库选项（筛选下拉框）
    const repoOptions = useMemo(() => {
      if (!discoverableSkills) return [];
      const repoSet = new Set<string>();
      discoverableSkills.forEach((s) => {
        if (s.repoOwner && s.repoName) {
          repoSet.add(`${s.repoOwner}/${s.repoName}`);
        }
      });
      return Array.from(repoSet).sort();
    }, [discoverableSkills]);

    // 仓库发现列表 + installed 状态
    const repoSkills: DiscoverableSkillItem[] = useMemo(() => {
      if (!discoverableSkills) return [];
      return discoverableSkills.map((d) => {
        const installName =
          d.directory.split(/[/\\]/).pop()?.toLowerCase() ||
          d.directory.toLowerCase();
        const key = `${installName}:${d.repoOwner.toLowerCase()}:${d.repoName.toLowerCase()}`;
        return { ...d, installed: installedKeys.has(key) };
      });
    }, [discoverableSkills, installedKeys]);

    const isSkillsShInstalled = (
      skill: SkillsShDiscoverableSkill,
    ): boolean => {
      const key = `${skill.directory.toLowerCase()}:${skill.repoOwner.toLowerCase()}:${skill.repoName.toLowerCase()}`;
      return installedKeys.has(key);
    };

    // 仓库模式本地过滤
    const filteredRepoSkills = useMemo(() => {
      const byRepo = repoSkills.filter((skill) => {
        if (filterRepo === "all") return true;
        return `${skill.repoOwner}/${skill.repoName}` === filterRepo;
      });
      const byStatus = byRepo.filter((skill) => {
        if (filterStatus === "installed") return skill.installed;
        if (filterStatus === "uninstalled") return !skill.installed;
        return true;
      });
      if (!searchQuery.trim()) return byStatus;
      const q = searchQuery.toLowerCase();
      return byStatus.filter((skill) => {
        const name = skill.name?.toLowerCase() || "";
        const repo =
          skill.repoOwner && skill.repoName
            ? `${skill.repoOwner}/${skill.repoName}`.toLowerCase()
            : "";
        return name.includes(q) || repo.includes(q);
      });
    }, [repoSkills, searchQuery, filterRepo, filterStatus]);

    // ===== 合并 + 去重（"全部" 标签页） =====
    const mergedResults: UnifiedDisplaySkill[] = useMemo(() => {
      if (searchSource !== "all") return [];

      const seen = new Set<string>();
      const results: UnifiedDisplaySkill[] = [];
      const normalize = (name: string) =>
        name.toLowerCase().replace(/[\s_-]+/g, "");

      // 仓库结果优先
      filteredRepoSkills.forEach((s) => {
        const norm = normalize(s.name);
        if (!seen.has(norm)) {
          seen.add(norm);
          results.push({ key: s.key, skill: s, source: "repos" });
        }
      });

      // skills.sh 结果
      if (skillsShResult) {
        skillsShResult.skills.forEach((s) => {
          const norm = normalize(s.name);
          if (!seen.has(norm)) {
            seen.add(norm);
            const d: DiscoverableSkill = {
              key: s.key,
              name: s.name,
              description: "",
              directory: s.directory,
              repoOwner: s.repoOwner,
              repoName: s.repoName,
              repoBranch: s.repoBranch,
              readmeUrl: s.readmeUrl,
            };
            results.push({
              key: s.key,
              skill: { ...d, installed: isSkillsShInstalled(s) },
              source: "skillssh",
              installs: s.installs,
            });
          }
        });
      }

      // ClawHub 结果
      if (clawHubResult) {
        clawHubResult.skills.forEach((s) => {
          const norm = normalize(s.displayName);
          if (!seen.has(norm)) {
            seen.add(norm);
            const d: DiscoverableSkill = {
              key: `clawhub:${s.slug}`,
              name: s.displayName,
              description: s.summary,
              directory: s.slug,
              repoOwner: "",
              repoName: "",
              repoBranch: "main",
              readmeUrl: `https://clawhub.ai/skills/${s.slug}`,
            };
            const stats = clawHubStats[s.slug];
            results.push({
              key: `clawhub:${s.slug}`,
              skill: {
                ...d,
                installed: installedKeys.has(`clawhub:${s.slug}`),
              },
              source: "clawhub",
              stars: stats?.stars ?? s.stars,
              installs: stats?.installs ?? stats?.downloads,
            });
          }
        });
      }

      return results;
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [
      searchSource,
      filteredRepoSkills,
      skillsShResult,
      clawHubResult,
      installedKeys,
      clawHubStats,
    ]);

    // ===== 加载状态 =====
    const isSearchLoading =
      searchSource === "repos"
        ? loadingDiscoverable || fetchingDiscoverable
        : searchSource === "all"
          ? loadingSkillsSh || loadingClawHub
          : searchSource === "skillssh"
            ? loadingSkillsSh && accumulatedResults.length === 0
            : loadingClawHub;

    const isSearchFetching =
      searchSource === "all"
        ? fetchingSkillsSh || fetchingClawHub
        : searchSource === "skillssh"
          ? fetchingSkillsSh
          : searchSource === "clawhub"
            ? fetchingClawHub
            : false;

    const hasQuery = userHasQuery || searchSource === "all";

    useImperativeHandle(ref, () => ({
      refresh: () => {
        refetchDiscoverable();
        refetchRepos();
      },
      openRepoManager: () => setRepoManagerOpen(true),
    }));

    // skills.sh → DiscoverableSkill 转换
    const toDiscoverableSkill = (
      s: SkillsShDiscoverableSkill,
    ): DiscoverableSkill => ({
      key: s.key,
      name: s.name,
      description: "",
      directory: s.directory,
      repoOwner: s.repoOwner,
      repoName: s.repoName,
      repoBranch: s.repoBranch,
      readmeUrl: s.readmeUrl,
    });

    // ===== 安装/卸载 =====
    const handleInstall = async (key: string) => {
      // ClawHub 技能：静默执行 CLI 安装
      if (key.startsWith("clawhub:")) {
        const slug = key.replace("clawhub:", "");
        try {
          await installClawHubMutation.mutateAsync(slug);
          toast.success(
            t("skills.clawhub.installSuccess", {
              slug,
              defaultValue: `${slug} 安装成功`,
            }),
          );
        } catch (error) {
          toast.error(
            t("skills.clawhub.installFailed", {
              defaultValue: "ClawHub 技能安装失败",
            }),
            { description: String(error) },
          );
        }
        return;
      }

      let skill: DiscoverableSkill | undefined;

      if (searchSource === "skillssh") {
        const found = accumulatedResults.find((s) => s.key === key);
        if (found) skill = toDiscoverableSkill(found);
      } else if (searchSource === "all") {
        const found = mergedResults.find((r) => r.key === key);
        if (found) skill = found.skill;
      } else {
        skill = discoverableSkills?.find((s) => s.key === key);
      }

      if (!skill) {
        toast.error(t("skills.notFound"));
        return;
      }

      try {
        await installMutation.mutateAsync({ skill, currentApp });
        toast.success(t("skills.installSuccess", { name: skill.name }), {
          closeButton: true,
        });
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        const { title, description } = formatSkillError(
          errorMessage,
          t,
          "skills.installFailed",
        );
        toast.error(title, { description, duration: 10000 });
        console.error("Install skill failed:", error);
      }
    };

    const handleUninstall = async (_directory: string) => {
      toast.info(t("skills.uninstallInMainPanel"));
    };

    const handleAddRepo = async (repo: SkillRepo) => {
      try {
        await addRepoMutation.mutateAsync(repo);
        const { data: freshSkills } = await refetchDiscoverable();
        const count =
          freshSkills?.filter(
            (s) =>
              s.repoOwner === repo.owner &&
              s.repoName === repo.name &&
              (s.repoBranch || "main") === (repo.branch || "main"),
          ).length ?? 0;
        toast.success(
          t("skills.repo.addSuccess", {
            owner: repo.owner,
            name: repo.name,
            count,
          }),
          { closeButton: true },
        );
      } catch (error) {
        toast.error(t("common.error"), { description: String(error) });
      }
    };

    const handleRemoveRepo = async (owner: string, name: string) => {
      try {
        await removeRepoMutation.mutateAsync({ owner, name });
        toast.success(t("skills.repo.removeSuccess", { owner, name }), {
          closeButton: true,
        });
      } catch (error) {
        toast.error(t("common.error"), { description: String(error) });
      }
    };

    const handleToggleRepo = async (
      owner: string,
      name: string,
      enabled: boolean,
    ) => {
      try {
        await toggleRepoMutation.mutateAsync({ owner, name, enabled });
        toast.success(
          enabled
            ? t("skills.repo.enableSuccess", {
                owner,
                name,
                defaultValue: `已启用 ${owner}/${name}`,
              })
            : t("skills.repo.disableSuccess", {
                owner,
                name,
                defaultValue: `已禁用 ${owner}/${name}`,
              }),
          { closeButton: true },
        );
      } catch (error) {
        toast.error(t("common.error"), { description: String(error) });
      }
    };

    // skills.sh 是否有更多（仅 skillssh 标签页）
    const hasMoreSkillsSh =
      searchSource === "skillssh" &&
      skillsShResult &&
      accumulatedResults.length < skillsShResult.totalCount;

    // 搜索框 placeholder
    const searchPlaceholder =
      searchSource === "repos"
        ? t("skills.searchPlaceholder")
        : searchSource === "skillssh"
          ? t("skills.skillssh.searchPlaceholder")
          : searchSource === "clawhub"
            ? t("skills.clawhub.searchPlaceholder")
            : t("skills.all.searchPlaceholder");

    // Tab 定义
    const tabs: { id: SearchSource; label: string; mw: string }[] = [
      { id: "all", label: t("skills.searchSource.all"), mw: "min-w-[48px]" },
      {
        id: "repos",
        label: t("skills.searchSource.repos"),
        mw: "min-w-[48px]",
      },
      { id: "skillssh", label: "skills.sh", mw: "min-w-[64px]" },
      { id: "clawhub", label: "ClawHub", mw: "min-w-[64px]" },
    ];

    // 来源计数（"全部" 标签页）
    const sourceCounts = useMemo(() => {
      if (searchSource !== "all" || !hasQuery) return null;
      return {
        repos: filteredRepoSkills.length,
        skillssh: skillsShResult?.skills.length ?? 0,
        clawhub: clawHubResult?.skills.length ?? 0,
      };
    }, [
      searchSource,
      hasQuery,
      filteredRepoSkills,
      skillsShResult,
      clawHubResult,
    ]);

    return (
      <div className="px-6 flex flex-col flex-1 min-h-0 overflow-hidden bg-background/50">
        <div className="flex-1 overflow-y-auto overflow-x-hidden animate-fade-in">
          <div className="py-4">
            {/* 标签栏 + 搜索框 */}
            <div className="mb-6 flex flex-col gap-3">
              {/* 标签栏 */}
              <div className="flex items-center gap-3">
                <div className="inline-flex gap-1 rounded-md border border-border-default bg-background p-1 shrink-0">
                  {tabs.map((tab) => (
                    <Button
                      key={tab.id}
                      type="button"
                      size="sm"
                      variant={searchSource === tab.id ? "default" : "ghost"}
                      className={
                        searchSource === tab.id
                          ? `shadow-sm ${tab.mw}`
                          : `text-muted-foreground hover:text-foreground hover:bg-muted ${tab.mw}`
                      }
                      onClick={() => setSearchSource(tab.id)}
                    >
                      {tab.label}
                    </Button>
                  ))}
                </div>
                {isSearchFetching && !isSearchLoading && (
                  <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                )}
              </div>

              {/* 搜索 + 筛选行 */}
              <div className="flex flex-col gap-3 md:flex-row md:items-center">
                <div className="relative flex-1 min-w-0">
                  <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    type="text"
                    placeholder={searchPlaceholder}
                    value={searchInput}
                    onChange={(e) => setSearchInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleSearch();
                    }}
                    className="pl-9 pr-3"
                  />
                </div>

                {/* 仓库模式筛选 */}
                {searchSource === "repos" && (
                  <>
                    <div className="w-full md:w-56">
                      <Select value={filterRepo} onValueChange={setFilterRepo}>
                        <SelectTrigger className="bg-card border shadow-sm text-foreground">
                          <SelectValue
                            placeholder={t("skills.filter.repo")}
                            className="text-left truncate"
                          />
                        </SelectTrigger>
                        <SelectContent className="bg-card text-foreground shadow-lg max-h-64 min-w-[var(--radix-select-trigger-width)]">
                          <SelectItem
                            value="all"
                            className="text-left pr-3 [&[data-state=checked]>span:first-child]:hidden"
                          >
                            {t("skills.filter.allRepos")}
                          </SelectItem>
                          {repoOptions.map((repo) => (
                            <SelectItem
                              key={repo}
                              value={repo}
                              className="text-left pr-3 [&[data-state=checked]>span:first-child]:hidden"
                              title={repo}
                            >
                              <span className="truncate block max-w-[200px]">
                                {repo}
                              </span>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="w-full md:w-36">
                      <Select
                        value={filterStatus}
                        onValueChange={(val) =>
                          setFilterStatus(
                            val as "all" | "installed" | "uninstalled",
                          )
                        }
                      >
                        <SelectTrigger className="bg-card border shadow-sm text-foreground">
                          <SelectValue
                            placeholder={t("skills.filter.placeholder")}
                            className="text-left"
                          />
                        </SelectTrigger>
                        <SelectContent className="bg-card text-foreground shadow-lg">
                          <SelectItem
                            value="all"
                            className="text-left pr-3 [&[data-state=checked]>span:first-child]:hidden"
                          >
                            {t("skills.filter.all")}
                          </SelectItem>
                          <SelectItem
                            value="installed"
                            className="text-left pr-3 [&[data-state=checked]>span:first-child]:hidden"
                          >
                            {t("skills.filter.installed")}
                          </SelectItem>
                          <SelectItem
                            value="uninstalled"
                            className="text-left pr-3 [&[data-state=checked]>span:first-child]:hidden"
                          >
                            {t("skills.filter.uninstalled")}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </>
                )}

                {/* skills.sh / ClawHub 搜索按钮 */}
                {(searchSource === "skillssh" ||
                  searchSource === "clawhub") && (
                  <Button
                    size="sm"
                    onClick={handleSearch}
                    disabled={searchInput.trim().length < 2 || isSearchFetching}
                    className="shrink-0"
                  >
                    {isSearchFetching ? (
                      <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                    ) : (
                      <Search className="h-3.5 w-3.5 mr-1.5" />
                    )}
                    {t("skills.search")}
                  </Button>
                )}
              </div>
            </div>

            {/* ===== 内容区域 ===== */}

            {searchSource === "all" && (
              /* ===== "全部" 标签页：多源合并 ===== */
              <>
                {isSearchLoading ? (
                  <div className="flex items-center justify-center h-64">
                    <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                    <span className="ml-3 text-sm text-muted-foreground">
                      {t("skills.all.searching")}
                    </span>
                  </div>
                ) : !hasQuery ? (
                  // 无搜索关键词 → 展示仓库技能 + 引导搜索
                  loadingDiscoverable || fetchingDiscoverable ? (
                    <div className="flex items-center justify-center h-64">
                      <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
                    </div>
                  ) : repoSkills.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-64 text-center">
                      <Search className="h-12 w-12 text-muted-foreground/30 mb-4" />
                      <p className="text-sm text-muted-foreground">
                        {t("skills.all.typeToSearch")}
                      </p>
                    </div>
                  ) : (
                    <>
                      <p className="mb-3 text-sm text-muted-foreground">
                        {t("skills.count", {
                          count: filteredRepoSkills.length,
                        })}
                        {" · "}
                        {t("skills.all.typeToSearchMore")}
                      </p>
                      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                        {filteredRepoSkills.map((skill) => (
                          <SkillCard
                            key={skill.key}
                            skill={skill}
                            source="repos"
                            onInstall={handleInstall}
                            onUninstall={handleUninstall}
                          />
                        ))}
                      </div>
                    </>
                  )
                ) : mergedResults.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-48 text-center">
                    <p className="text-lg font-medium text-foreground">
                      {t("skills.noResults")}
                    </p>
                  </div>
                ) : (
                  <>
                    {sourceCounts && (
                      <p className="mb-3 text-sm text-muted-foreground">
                        {t("skills.all.mergedCount", {
                          total: mergedResults.length,
                        })}
                        {sourceCounts.repos > 0 &&
                          ` · ${t("skills.searchSource.repos")} ${sourceCounts.repos}`}
                        {sourceCounts.skillssh > 0 &&
                          ` · skills.sh ${sourceCounts.skillssh}`}
                        {sourceCounts.clawhub > 0 &&
                          ` · ClawHub ${sourceCounts.clawhub}`}
                      </p>
                    )}
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                      {mergedResults.map((item) => (
                        <SkillCard
                          key={item.key}
                          skill={item.skill}
                          source={item.source}
                          installs={item.installs}
                          stars={item.stars}
                          onInstall={handleInstall}
                          onUninstall={handleUninstall}
                        />
                      ))}
                    </div>
                  </>
                )}
              </>
            )}

            {searchSource === "repos" && (
              /* ===== 仓库标签页 ===== */
              <>
                {isSearchLoading ? (
                  <div className="flex items-center justify-center h-64">
                    <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
                  </div>
                ) : repoSkills.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-64 text-center">
                    <p className="text-lg font-medium text-foreground">
                      {t("skills.empty")}
                    </p>
                    <p className="mt-2 text-sm text-muted-foreground">
                      {t("skills.emptyDescription")}
                    </p>
                    <Button
                      variant="link"
                      onClick={() => setRepoManagerOpen(true)}
                      className="mt-3 text-sm font-normal"
                    >
                      {t("skills.addRepo")}
                    </Button>
                  </div>
                ) : filteredRepoSkills.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-48 text-center">
                    <p className="text-lg font-medium text-foreground">
                      {t("skills.noResults")}
                    </p>
                  </div>
                ) : (
                  <>
                    <p className="mb-3 text-sm text-muted-foreground">
                      {t("skills.count", { count: filteredRepoSkills.length })}
                    </p>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                      {filteredRepoSkills.map((skill) => (
                        <SkillCard
                          key={skill.key}
                          skill={skill}
                          source="repos"
                          onInstall={handleInstall}
                          onUninstall={handleUninstall}
                        />
                      ))}
                    </div>
                  </>
                )}
              </>
            )}

            {searchSource === "skillssh" && (
              /* ===== skills.sh 标签页 ===== */
              <>
                {loadingSkillsSh && accumulatedResults.length === 0 ? (
                  <div className="flex items-center justify-center h-64">
                    <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                    <span className="ml-3 text-sm text-muted-foreground">
                      {t("skills.skillssh.loading")}
                    </span>
                  </div>
                ) : !hasQuery ? (
                  <div className="flex flex-col items-center justify-center h-64 text-center">
                    <Search className="h-12 w-12 text-muted-foreground/30 mb-4" />
                    <p className="text-sm text-muted-foreground">
                      {t("skills.skillssh.searchPlaceholder")}
                    </p>
                  </div>
                ) : accumulatedResults.length === 0 && !loadingSkillsSh ? (
                  <div className="flex flex-col items-center justify-center h-48 text-center">
                    <p className="text-lg font-medium text-foreground">
                      {t("skills.skillssh.noResults", {
                        query: searchQuery,
                      })}
                    </p>
                  </div>
                ) : (
                  <>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                      {accumulatedResults.map((skill) => (
                        <SkillCard
                          key={skill.key}
                          skill={{
                            ...toDiscoverableSkill(skill),
                            installed: isSkillsShInstalled(skill),
                          }}
                          source="skillssh"
                          installs={skill.installs}
                          onInstall={handleInstall}
                          onUninstall={handleUninstall}
                        />
                      ))}
                    </div>
                    <div className="mt-6 flex flex-col items-center gap-2">
                      {hasMoreSkillsSh && (
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={fetchingSkillsSh}
                          onClick={() =>
                            setSkillsShOffset((prev) => prev + PAGE_SIZE)
                          }
                        >
                          {fetchingSkillsSh ? (
                            <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                          ) : null}
                          {t("skills.skillssh.loadMore")}
                        </Button>
                      )}
                      <p className="text-xs text-muted-foreground">
                        {t("skills.skillssh.poweredBy")}
                      </p>
                    </div>
                  </>
                )}
              </>
            )}

            {searchSource === "clawhub" && (
              /* ===== ClawHub 标签页 ===== */
              <>
                {loadingClawHub ? (
                  <div className="flex items-center justify-center h-64">
                    <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                    <span className="ml-3 text-sm text-muted-foreground">
                      {t("skills.clawhub.loading")}
                    </span>
                  </div>
                ) : !hasQuery ? (
                  <div className="flex flex-col items-center justify-center h-64 text-center">
                    <Search className="h-12 w-12 text-muted-foreground/30 mb-4" />
                    <p className="text-sm text-muted-foreground">
                      {t("skills.clawhub.searchPlaceholder")}
                    </p>
                  </div>
                ) : !clawHubResult ||
                  clawHubResult.skills.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-48 text-center">
                    <p className="text-lg font-medium text-foreground">
                      {t("skills.clawhub.noResults", { query: searchQuery })}
                    </p>
                  </div>
                ) : (
                  <>
                    <p className="mb-3 text-sm text-muted-foreground">
                      {t("skills.clawhub.resultCount", {
                        count: clawHubResult.skills.length,
                      })}
                    </p>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                      {clawHubResult.skills.map((s) => {
                        const d: DiscoverableSkill = {
                          key: `clawhub:${s.slug}`,
                          name: s.displayName,
                          description: s.summary,
                          directory: s.slug,
                          repoOwner: "",
                          repoName: "",
                          repoBranch: "main",
                          readmeUrl: `https://clawhub.ai/skills/${s.slug}`,
                        };
                        const stats = clawHubStats[s.slug];
                        return (
                          <SkillCard
                            key={`clawhub:${s.slug}`}
                            skill={{
                              ...d,
                              installed: installedKeys.has(
                                `clawhub:${s.slug}`,
                              ),
                            }}
                            source="clawhub"
                            stars={stats?.stars}
                            installs={stats?.installs ?? stats?.downloads}
                            onInstall={handleInstall}
                            onUninstall={handleUninstall}
                          />
                        );
                      })}
                    </div>
                    <p className="mt-4 text-xs text-muted-foreground text-center">
                      {t("skills.clawhub.poweredBy")}
                    </p>
                  </>
                )}
              </>
            )}
          </div>
        </div>

        {/* 仓库管理面板 */}
        {repoManagerOpen && (
          <RepoManagerPanel
            repos={repos}
            skills={repoSkills}
            onAdd={handleAddRepo}
            onRemove={handleRemoveRepo}
            onToggle={handleToggleRepo}
            onClose={() => setRepoManagerOpen(false)}
          />
        )}
      </div>
    );
  },
);

SkillsPage.displayName = "SkillsPage";
