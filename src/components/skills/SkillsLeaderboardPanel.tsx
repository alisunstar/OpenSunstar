import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  RefreshCw,
  Loader2,
  ExternalLink,
  Trophy,
  TrendingUp,
  AlertTriangle,
  ShieldCheck,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { SkillCard } from "./SkillCard";
import {
  useInstalledSkills,
  useInstallSkill,
  useSkillsShLeaderboard,
  type SkillsLeaderboardTabPeriod,
} from "@/hooks/useSkills";
import type { DiscoverableSkill } from "@/lib/api/skills";
import type { AppId } from "@/lib/api/types";
import { formatSkillError } from "@/lib/errors/skillErrorParser";
import { cn } from "@/lib/utils";

/** 每页展示条数（TOP50 分 5 页，首屏约 2 行 × 3 列） */
const LEADERBOARD_PAGE_SIZE = 12;

interface SkillsLeaderboardPanelProps {
  period: SkillsLeaderboardTabPeriod;
  currentApp: AppId;
  refreshKey?: number;
}

function formatSyncedAt(ms: number, locale: string): string {
  try {
    return new Intl.DateTimeFormat(locale, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(ms));
  } catch {
    return new Date(ms).toLocaleString();
  }
}

function formatInstallCount(value: number): string {
  return new Intl.NumberFormat(undefined, {
    notation: value >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: 1,
  }).format(value);
}

function formatNextSyncAt(syncedAtMs: number, ttlSecs: number, locale: string): string {
  const nextMs = syncedAtMs + ttlSecs * 1000;
  return formatSyncedAt(nextMs, locale);
}

export function SkillsLeaderboardPanel({
  period,
  currentApp,
  refreshKey = 0,
}: SkillsLeaderboardPanelProps) {
  const { t, i18n } = useTranslation();
  const [forceNonce, setForceNonce] = useState(0);
  const [page, setPage] = useState(1);
  const { data, isLoading, isError, error, refetch, isFetching } =
    useSkillsShLeaderboard(period, refreshKey, forceNonce);

  useEffect(() => {
    setPage(1);
  }, [period]);

  const totalCount = data?.skills.length ?? 0;
  const totalPages = Math.max(1, Math.ceil(totalCount / LEADERBOARD_PAGE_SIZE));
  const safePage = Math.min(page, totalPages);

  const pageItems = useMemo(() => {
    if (!data) return [];
    const start = (safePage - 1) * LEADERBOARD_PAGE_SIZE;
    return data.skills.slice(start, start + LEADERBOARD_PAGE_SIZE);
  }, [data, safePage]);

  const rankRange =
    pageItems.length > 0
      ? `#${pageItems[0].rank}–#${pageItems[pageItems.length - 1].rank}`
      : "";

  const { data: installedSkills } = useInstalledSkills();
  const installedKeys = new Set(
    installedSkills?.map(
      (s) =>
        `${s.directory.toLowerCase()}:${s.repoOwner?.toLowerCase() || ""}:${s.repoName?.toLowerCase() || ""}`,
    ) ?? [],
  );

  const installMutation = useInstallSkill();

  const handleForceRefresh = () => {
    setForceNonce((n) => n + 1);
    void refetch();
  };

  const handleInstall = async (key: string) => {
    const item = data?.skills.find((s) => s.key === key);
    if (!item) return;

    const skill: DiscoverableSkill = {
      key: item.key,
      name: item.name,
      description: "",
      directory: item.directory,
      repoOwner: item.repoOwner,
      repoName: item.repoName,
      repoBranch: "main",
      readmeUrl: item.readmeUrl,
    };

    try {
      await installMutation.mutateAsync({ skill, currentApp });
      toast.success(t("skills.installSuccess", { name: item.name }), {
        closeButton: true,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      const { title, description } = formatSkillError(msg, t, "skills.installFailed");
      toast.error(title, { description, duration: 10000 });
    }
  };

  const handleUninstall = async () => {
    toast.info(t("skills.uninstallInMainPanel"));
  };

  const periodLabel =
    period === "all_time"
      ? t("skills.leaderboard.allTimeTitle", { defaultValue: "全站总榜" })
      : t("skills.leaderboard.trendingTitle", { defaultValue: "24h 趋势" });

  const periodDesc =
    period === "all_time"
      ? t("skills.leaderboard.allTimeDesc", {
          defaultValue: "与 skills.sh 官网 All Time 排行榜 TOP50 对齐（历史总安装量）",
        })
      : t("skills.leaderboard.trendingDesc", {
          defaultValue: "与 skills.sh 官网 Trending (24h) 排行榜 TOP50 对齐",
        });

  const PeriodIcon = period === "all_time" ? Trophy : TrendingUp;
  const cacheTtlSecs = data?.cacheTtlSecs ?? 6 * 3600;

  return (
    <div className="flex flex-col gap-4">
      <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-4 py-3 space-y-2">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex items-start gap-2 min-w-0">
            <ShieldCheck className="h-4 w-4 text-emerald-600 dark:text-emerald-400 shrink-0 mt-0.5" />
            <div className="min-w-0 space-y-1">
              <div className="flex flex-wrap items-center gap-2">
                <PeriodIcon
                  className={cn(
                    "h-4 w-4 shrink-0",
                    period === "all_time"
                      ? "text-amber-500"
                      : "text-orange-500",
                  )}
                />
                <h3 className="text-sm font-semibold">{periodLabel}</h3>
                <span className="rounded-full bg-emerald-500/15 px-2 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-300">
                  skills.sh
                </span>
                <span className="text-[10px] text-muted-foreground">TOP 50</span>
              </div>
              <p className="text-xs text-muted-foreground">{periodDesc}</p>
              <p className="text-[11px] text-muted-foreground/90">
                {t("skills.leaderboard.syncPolicy", {
                  hours: Math.round(cacheTtlSecs / 3600),
                  defaultValue: `非实时数据 · 默认每 ${Math.round(cacheTtlSecs / 3600)} 小时从 skills.sh 同步 · 点击刷新可立即更新`,
                })}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <a
              href={
                period === "all_time"
                  ? "https://skills.sh/"
                  : "https://skills.sh/trending"
              }
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1 text-xs text-primary hover:underline"
            >
              skills.sh
              <ExternalLink className="h-3 w-3" />
            </a>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleForceRefresh}
              disabled={isFetching}
              title={t("skills.leaderboard.refreshNow", {
                defaultValue: "立即从 skills.sh 同步",
              })}
            >
              <RefreshCw
                size={14}
                className={isFetching ? "animate-spin" : ""}
              />
            </Button>
          </div>
        </div>

        {data && (
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-muted-foreground border-t border-emerald-500/10 pt-2">
            <span>
              {t("skills.leaderboard.syncedAt", {
                time: formatSyncedAt(data.syncedAt, i18n.language),
                defaultValue: `同步于 ${formatSyncedAt(data.syncedAt, i18n.language)}`,
              })}
            </span>
            {data.fromCache && (
              <span>
                {t("skills.leaderboard.fromCache", { defaultValue: "本地缓存" })}
              </span>
            )}
            <span>
              {t("skills.leaderboard.nextSyncAt", {
                time: formatNextSyncAt(data.syncedAt, cacheTtlSecs, i18n.language),
                defaultValue: `下次自动同步约 ${formatNextSyncAt(data.syncedAt, cacheTtlSecs, i18n.language)}`,
              })}
            </span>
            {data.allTimeTotal != null && period === "all_time" && (
              <span>
                {t("skills.leaderboard.registryTotal", {
                  count: formatInstallCount(data.allTimeTotal),
                  defaultValue: `全站累计 ${formatInstallCount(data.allTimeTotal)} 次安装`,
                })}
              </span>
            )}
          </div>
        )}
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <span className="ml-3 text-sm text-muted-foreground">
            {t("skills.leaderboard.loading", {
              defaultValue: "正在同步 skills.sh 官方排行榜…",
            })}
          </span>
        </div>
      ) : isError ? (
        <div className="flex flex-col items-center justify-center h-48 gap-3 rounded-lg border border-dashed border-destructive/30 bg-destructive/5 p-6">
          <AlertTriangle className="h-8 w-8 text-destructive/70" />
          <p className="text-sm text-destructive text-center max-w-md">
            {error instanceof Error
              ? error.message
              : t("skills.leaderboard.error", {
                  defaultValue: "无法同步 skills.sh 官方排行榜",
                })}
          </p>
          <Button variant="outline" size="sm" onClick={handleForceRefresh}>
            <RefreshCw size={14} className="mr-1" />
            {t("common.retry", { defaultValue: "重试" })}
          </Button>
        </div>
      ) : !data || totalCount === 0 ? (
        <div className="flex flex-col items-center justify-center h-48">
          <p className="text-sm text-muted-foreground">
            {t("skills.leaderboard.empty", { defaultValue: "暂无排行榜数据" })}
          </p>
        </div>
      ) : (
        <>
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>
              {t("skills.leaderboard.pageRange", {
                range: rankRange,
                page: safePage,
                totalPages,
                total: totalCount,
                defaultValue: `${rankRange} · 第 ${safePage}/${totalPages} 页（共 ${totalCount} 条）`,
              })}
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {pageItems.map((item) => {
              const installed = installedKeys.has(
                `${item.directory.toLowerCase()}:${item.repoOwner.toLowerCase()}:${item.repoName.toLowerCase()}`,
              );
              return (
                <SkillCard
                  key={item.key}
                  skill={{
                    key: item.key,
                    name: item.name,
                    description: item.source,
                    directory: item.directory,
                    repoOwner: item.repoOwner,
                    repoName: item.repoName,
                    repoBranch: "main",
                    readmeUrl: item.readmeUrl,
                    installed,
                  }}
                  source="skillssh"
                  installs={item.installs}
                  rank={item.rank}
                  onInstall={handleInstall}
                  onUninstall={handleUninstall}
                />
              );
            })}
          </div>

          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-4 pt-2">
              <Button
                variant="ghost"
                size="sm"
                disabled={safePage <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
              >
                <ChevronLeft size={14} className="mr-1" />
                {t("common.previous", { defaultValue: "上一页" })}
              </Button>
              <span className="text-xs text-muted-foreground tabular-nums">
                {safePage} / {totalPages}
              </span>
              <Button
                variant="ghost"
                size="sm"
                disabled={safePage >= totalPages}
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              >
                {t("common.next", { defaultValue: "下一页" })}
                <ChevronRight size={14} className="ml-1" />
              </Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
