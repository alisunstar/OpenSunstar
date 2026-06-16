import { useState, useMemo, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import {
  Search,
  LayoutGrid,
  BarChart3,
  FolderArchive,
  Plus,
  RefreshCw,
  CopyCheck,
  AlertTriangle,
  X,
} from "lucide-react";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { StageSection } from "./StageSection";
import { StagePicker } from "./StagePicker";
import type { StageKey } from "@/hooks/useProjectStages";
import { useProjectStages } from "@/hooks/useProjectStages";
import { useProjectProgress } from "@/hooks/useProjectProgress";
import { revealPathInFolder } from "@/lib/reveal";
import type { Project } from "@/types/project";
import {
  countProjectCodeLines,
  readPackageVersion,
  gitCommitCountLastNDays,
  gitContributors,
  type CodeLineResult,
  type Contributor,
} from "@/api/codeMetrics";
import { detectProjectGitInfo, type ProjectGitInfo } from "@/api/projectGit";

interface KanbanPageProps {
  projects: Project[];
  selectedProjectId?: string;
  onProjectClick: (project: Project) => void;
  onProjectRemove: (projectId: string) => void;
  onAddProject: () => void;
  onClearSelection?: () => void;
}

function SearchIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width={16}
      height={16}
      aria-hidden
      className="text-muted-foreground"
    >
      <circle
        cx="11"
        cy="11"
        r="6"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path d="m16 16 4 4" fill="none" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

export function KanbanPage({
  projects,
  selectedProjectId,
  onProjectClick,
  onProjectRemove,
  onAddProject,
  onClearSelection,
}: KanbanPageProps) {
  const { t } = useTranslation();
  const { stages, getStage, setStage } = useProjectStages();
  const { progress: progressMap, getProgress, setProjectProgress } =
    useProjectProgress();
  const [searchQuery, setSearchQuery] = useState("");
  const [internalDetailId, setInternalDetailId] = useState<string | null>(null);

  // ── 扫描状态 ────────────────────────────────
  const [codeLinesMap, setCodeLinesMap] = useState<
    Map<string, CodeLineResult>
  >(new Map());
  const [versionMap, setVersionMap] = useState<Map<string, string>>(new Map());
  const [gitInfoMap, setGitInfoMap] = useState<Map<string, ProjectGitInfo>>(
    new Map(),
  );
  const [activityMap, setActivityMap] = useState<Map<string, number>>(
    new Map(),
  );
  const [contributorsMap, setContributorsMap] = useState<
    Map<string, Contributor[]>
  >(new Map());
  const [scanning, setScanning] = useState(false);
  const [scanEpoch, setScanEpoch] = useState(0);
  const [scanProgress, setScanProgress] = useState({ done: 0, total: 0 });

  // ── 重复检测 ────────────────────────────────
  type DupGroup = { reason: string; projects: Project[] };
  const [dupGroups, setDupGroups] = useState<DupGroup[] | null>(null);
  const [dupScanning, setDupScanning] = useState(false);

  const scanDuplicates = () => {
    if (projects.length < 2) {
      setDupGroups([]);
      return;
    }
    setDupScanning(true);
    const groups: DupGroup[] = [];

    // 按名称分组
    const byName = new Map<string, Project[]>();
    for (const p of projects) {
      const key = p.name.toLowerCase();
      if (!byName.has(key)) byName.set(key, []);
      byName.get(key)!.push(p);
    }
    for (const [, list] of byName) {
      if (list.length > 1) {
        groups.push({
          reason: `项目名「${list[0].name}」重复（${list.length} 个）`,
          projects: list,
        });
      }
    }

    // 按路径分组
    const byPath = new Map<string, Project[]>();
    for (const p of projects) {
      const key = p.path.toLowerCase().replace(/\\/g, "/");
      if (!byPath.has(key)) byPath.set(key, []);
      byPath.get(key)!.push(p);
    }
    for (const [, list] of byPath) {
      if (list.length > 1) {
        const alreadyInName = groups.some((g) =>
          g.projects.some((gp) => list.some((lp) => lp.id === gp.id)),
        );
        if (!alreadyInName) {
          groups.push({
            reason: `路径「${list[0].path}」重复（${list.length} 个）`,
            projects: list,
          });
        }
      }
    }

    setDupGroups(groups);
    setDupScanning(false);
  };

  // ── 自动扫描 ────────────────────────────────
  useEffect(() => {
    if (projects.length === 0) {
      setScanning(false);
      return;
    }
    let cancelled = false;
    setScanning(true);
    setScanProgress({ done: 0, total: projects.length });

    const scan = async () => {
      for (const p of projects) {
        if (cancelled) break;
        try {
          const [code, version, activity, contribs, gitInfo] =
            await Promise.all([
              countProjectCodeLines(p.path),
              readPackageVersion(p.path),
              gitCommitCountLastNDays(p.path, 30),
              gitContributors(p.path),
              detectProjectGitInfo(p.path),
            ]);
          if (cancelled) break;
          if (code) setCodeLinesMap((m) => new Map(m).set(p.id, code));
          if (version) setVersionMap((m) => new Map(m).set(p.id, version));
          setActivityMap((m) => new Map(m).set(p.id, activity));
          if (contribs.length > 0)
            setContributorsMap((m) => new Map(m).set(p.id, contribs));
          if (gitInfo) setGitInfoMap((m) => new Map(m).set(p.id, gitInfo));
        } catch {
          /* 单个项目失败不影响其他 */
        }
        setScanProgress((prev) => ({ ...prev, done: prev.done + 1 }));
      }
      if (!cancelled) {
        setScanning(false);
      }
    };

    scan();
    return () => {
      cancelled = true;
    };
  }, [projects, scanEpoch]);

  const handleRefresh = () => setScanEpoch((n) => n + 1);

  // 外部 selectedProjectId 优先
  const activeDetailId = selectedProjectId ?? internalDetailId;
  const detailProject = useMemo(
    () => projects.find((p) => p.id === activeDetailId) ?? null,
    [projects, activeDetailId],
  );

  const openDetail = (project: Project) => {
    onProjectClick(project);
    setInternalDetailId(project.id);
  };

  const closeDetail = () => {
    setInternalDetailId(null);
    onClearSelection?.();
  };

  const filtered = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q),
    );
  }, [projects, searchQuery]);

  const grouped = useMemo(() => {
    const mvp = filtered.filter((p) => getStage(p.id) === "mvp");
    const rapid = filtered.filter((p) => getStage(p.id) === "rapid");
    const stable = filtered.filter((p) => getStage(p.id) === "stable");
    return { mvp, rapid, stable };
  }, [filtered, getStage]);

  const handleOpenFolder = async (path: string) => {
    await revealPathInFolder(path, { alertOnError: true });
  };

  const handleRemove = (projectId: string) => {
    const project = projects.find((p) => p.id === projectId);
    const name = project?.name ?? projectId;
    if (
      window.confirm(
        t("kanban.confirmRemove", {
          defaultValue: `从看板中移除「${name}」？\n不会删除磁盘上的文件。`,
        }).replace("${name}", name),
      )
    ) {
      onProjectRemove(projectId);
      toast.success(
        t("kanban.removed", { defaultValue: `已移除「${name}」` }),
      );
    }
  };

  const totalCount = projects.length;
  const empty = totalCount === 0;
  const noResults = !empty && filtered.length === 0;

  // ── 聚合指标 ────────────────────────────────
  const totalCodeLines = useMemo(() => {
    let sum = 0;
    for (const [, result] of codeLinesMap) sum += result.code_lines;
    return sum;
  }, [codeLinesMap]);

  const totalCommitsThisWeek = useMemo(() => {
    let sum = 0;
    for (const [, count] of activityMap) sum += count;
    return sum;
  }, [activityMap]);

  const { averageActivityLabel, averageActivityColor } = useMemo(() => {
    if (activityMap.size === 0)
      return { averageActivityLabel: "—", averageActivityColor: "" };
    let total = 0;
    for (const count of activityMap.values()) {
      total += count >= 40 ? 4 : count >= 11 ? 3 : count >= 1 ? 2 : 1;
    }
    const avg = total / activityMap.size;
    if (avg >= 3.5)
      return {
        averageActivityLabel: t("kanban.activity.veryHigh", { defaultValue: "很高" }),
        averageActivityColor: "text-emerald-500",
      };
    if (avg >= 2.5)
      return {
        averageActivityLabel: t("kanban.activity.high", { defaultValue: "高" }),
        averageActivityColor: "text-emerald-400",
      };
    if (avg >= 1.5)
      return {
        averageActivityLabel: t("kanban.activity.medium", { defaultValue: "中等" }),
        averageActivityColor: "text-amber-500",
      };
    return {
      averageActivityLabel: t("kanban.activity.low", { defaultValue: "低" }),
      averageActivityColor: "text-muted-foreground",
    };
  }, [activityMap, t]);

  function formatNumber(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1).replace(/\.0$/, "") + "M";
    if (n >= 10_000) return (n / 1_000).toFixed(1).replace(/\.0$/, "") + "K";
    return n.toLocaleString();
  }

  return (
    <motion.div
      className="flex-1 overflow-y-auto"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
    >
      {/* 页面头部 */}
      <div className="px-6 pt-6 pb-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
              <LayoutGrid className="w-5 h-5 text-primary" />
              {t("sidebar.kanban", { defaultValue: "项目看板" })}
            </h2>
            <p className="text-sm text-muted-foreground mt-1">
              {scanning
                ? t("kanban.scanning", {
                    defaultValue: `正在扫描 ${scanProgress.done}/${scanProgress.total} 个项目…`,
                  })
                : t("kanban.subtitle", {
                    defaultValue: "全局视角，掌握所有项目的进展与阶段",
                  })}
            </p>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {!empty && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleRefresh}
                disabled={scanning}
              >
                <RefreshCw
                  className={`w-4 h-4 mr-1 ${scanning ? "animate-spin" : ""}`}
                />
                {t("kanban.refresh", { defaultValue: "刷新指标" })}
              </Button>
            )}
            <Button variant="outline" size="sm" onClick={onAddProject}>
              <Plus className="w-4 h-4 mr-1" />
              {t("kanban.addProject", { defaultValue: "添加项目" })}
            </Button>
          </div>
        </div>
      </div>

      {/* 搜索栏 */}
      {!empty && (
        <div className="px-6 pb-4">
          <div className="relative max-w-md">
            <SearchIcon />
            <span className="absolute left-8 top-1/2 -translate-y-1/2">
              {/* icon positioned */}
            </span>
            <Input
              className="pl-9"
              placeholder={t("kanban.searchPlaceholder", {
                defaultValue: "搜索项目...",
              })}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </div>
      )}

      {/* ── 项目总览 ──────────────────────────── */}
      {!empty && (
        <div className="px-6 pb-6 space-y-4">
          <h3 className="text-sm font-semibold text-foreground">
            {t("board.summary.title", { defaultValue: "项目总览" })}
          </h3>

          {/* 指标卡片 */}
          <div className="grid grid-cols-4 gap-3">
            <SummaryCard
              label={t("board.summary.totalProjects", { defaultValue: "总项目数" })}
              value={String(totalCount)}
            />
            <SummaryCard
              label={t("board.summary.totalCodeLines", { defaultValue: "总代码行数" })}
              value={
                totalCodeLines > 0 ? formatNumber(totalCodeLines) : "—"
              }
              unit={totalCodeLines > 0 ? t("board.summary.linesUnit", { defaultValue: "行" }) : undefined}
            />
            <SummaryCard
              label={t("board.summary.avgActivity", { defaultValue: "平均活跃度" })}
              value={averageActivityLabel}
              color={averageActivityColor}
              sub={
                totalCommitsThisWeek > 0
                  ? t("kanban.thisWeekCommits", { defaultValue: "本周 {{n}} 次提交" }).replace("{{n}}", String(totalCommitsThisWeek))
                  : t("kanban.noRecentActivity", { defaultValue: "近期无活动" })
              }
            />
            <SummaryCard
              label={t("board.summary.thisWeek", { defaultValue: "本周更新" })}
              value={totalCommitsThisWeek > 0 ? String(totalCommitsThisWeek) : "—"}
              unit={totalCommitsThisWeek > 0 ? t("board.summary.updatesUnit", { defaultValue: "次" }) : undefined}
            />
          </div>

          {/* 阶段分布 */}
          <div className="rounded-xl border border-border/60 bg-card/30 p-4">
            <div className="flex items-center gap-6">
              <span className="text-xs font-medium text-muted-foreground">
                {t("kanban.stageDistribution", { defaultValue: "阶段分布" })}
              </span>
              {([
                { key: "mvp" as StageKey, label: "MVP 阶段（未上线）", color: "bg-purple-500", count: grouped.mvp.length },
                { key: "rapid" as StageKey, label: "快速迭代阶段（已上线）", color: "bg-emerald-500", count: grouped.rapid.length },
                { key: "stable" as StageKey, label: "慢迭代阶段（稳定维护）", color: "bg-blue-500", count: grouped.stable.length },
              ] as const).map((item) => (
                <div key={item.key} className="flex items-center gap-2">
                  <span className={`w-2.5 h-2.5 rounded-full ${item.color}`} />
                  <span className="text-xs text-foreground/80">{item.label}</span>
                  <span className="text-xs font-semibold text-foreground tabular-nums">
                    {item.count}
                    <span className="text-muted-foreground font-normal ml-0.5">
                      ({totalCount > 0 ? Math.round((item.count / totalCount) * 100) : 0}%)
                    </span>
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* 重复资产检测 */}
          <div className="rounded-xl border border-amber-500/20 bg-amber-500/5 p-4">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <CopyCheck className="w-4 h-4 text-amber-500" />
                <div>
                  <h4 className="text-sm font-semibold text-foreground">
                    {t("board.duplicateCleanup", {
                      defaultValue: "重复资产检测与清理",
                    })}
                  </h4>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {t("health.subtitle", {
                      defaultValue:
                        "检测同名项目与重复路径，识别可能的冗余添加",
                    })}
                  </p>
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                disabled={dupScanning}
                onClick={scanDuplicates}
              >
                {dupScanning ? (
                  <RefreshCw className="w-3.5 h-3.5 mr-1 animate-spin" />
                ) : (
                  <CopyCheck className="w-3.5 h-3.5 mr-1" />
                )}
                {t("health.scan", { defaultValue: "检测重复" })}
              </Button>
            </div>

            {/* 结果 */}
            {dupGroups !== null && (
              <>
                {dupGroups.length === 0 ? (
                  <div className="flex items-center gap-2 text-xs text-emerald-600 dark:text-emerald-400 py-2 px-3 rounded-lg bg-emerald-500/5">
                    <CopyCheck className="w-3.5 h-3.5" />
                    {t("health.noDuplicates", {
                      defaultValue: "未发现重复项目",
                    })}
                  </div>
                ) : (
                  <div className="space-y-2">
                    {dupGroups.map((group, gi) => (
                      <div
                        key={gi}
                        className="rounded-lg border border-amber-500/20 bg-background/50 p-3"
                      >
                        <div className="flex items-center gap-2 mb-2">
                          <AlertTriangle className="w-3.5 h-3.5 text-amber-500" />
                          <span className="text-xs font-medium text-amber-600 dark:text-amber-400">
                            {group.reason}
                          </span>
                        </div>
                        <div className="space-y-1">
                          {group.projects.map((p) => (
                            <div
                              key={p.id}
                              className="flex items-center justify-between text-xs py-1 px-2 rounded bg-muted/30"
                            >
                              <span className="truncate flex-1">
                                <span className="font-medium">{p.name}</span>
                                <span className="text-muted-foreground ml-2 font-mono text-[10px]">
                                  {p.path}
                                </span>
                              </span>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-5 w-5 text-muted-foreground hover:text-destructive"
                                onClick={() => {
                                  onProjectRemove(p.id);
                                  // 从当前结果中移除
                                  setDupGroups((prev) =>
                                    (prev ?? []).map((g) => ({
                                      ...g,
                                      projects: g.projects.filter(
                                        (gp) => gp.id !== p.id,
                                      ),
                                    })).filter((g) => g.projects.length > 1),
                                  );
                                }}
                                title={t("kanban.remove", {
                                  defaultValue: "移除项目",
                                })}
                              >
                                <X className="w-3 h-3" />
                              </Button>
                            </div>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      )}

      {/* 项目列表 */}
      <div className="px-6 pb-8 space-y-8">
        {empty ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <FolderArchive className="w-16 h-16 text-muted-foreground/30 mb-4" />
            <h3 className="text-base font-semibold text-foreground">
              {t("kanban.empty.title", { defaultValue: "暂无项目" })}
            </h3>
            <p className="text-sm text-muted-foreground mt-1.5 max-w-sm">
              {t("kanban.empty.description", {
                defaultValue: "点击下方按钮或在侧边栏添加你的第一个项目",
              })}
            </p>
            <Button onClick={onAddProject} className="mt-4" size="sm">
              <Plus className="w-4 h-4 mr-1" />
              {t("kanban.addProject", { defaultValue: "添加项目" })}
            </Button>
          </div>
        ) : noResults ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <Search className="w-12 h-12 text-muted-foreground/30 mb-3" />
            <p className="text-sm text-muted-foreground">
              {t("kanban.noResults", {
                defaultValue: `没有找到匹配「${searchQuery}」的项目`,
              })}
            </p>
          </div>
        ) : (
          <>
            <StageSection
              stage="mvp"
              projects={grouped.mvp}
              stages={stages}
              progressMap={progressMap}
              onProjectClick={openDetail}
              onProjectRemove={handleRemove}
              onStageChange={(projectId, stage) => setStage(projectId, stage)}
              onOpenFolder={handleOpenFolder}
            />
            <StageSection
              stage="rapid"
              projects={grouped.rapid}
              stages={stages}
              progressMap={progressMap}
              onProjectClick={openDetail}
              onProjectRemove={handleRemove}
              onStageChange={(projectId, stage) => setStage(projectId, stage)}
              onOpenFolder={handleOpenFolder}
            />
            <StageSection
              stage="stable"
              projects={grouped.stable}
              stages={stages}
              progressMap={progressMap}
              onProjectClick={openDetail}
              onProjectRemove={handleRemove}
              onStageChange={(projectId, stage) => setStage(projectId, stage)}
              onOpenFolder={handleOpenFolder}
            />
          </>
        )}
      </div>

      {/* 项目详情抽屉 */}
      {detailProject && (
        <ProjectDetailSheet
          project={detailProject}
          stage={getStage(detailProject.id)}
          progress={getProgress(detailProject.id)}
          codeLines={codeLinesMap.get(detailProject.id)}
          version={versionMap.get(detailProject.id)}
          gitInfo={gitInfoMap.get(detailProject.id)}
          activity={activityMap.get(detailProject.id)}
          contributors={contributorsMap.get(detailProject.id)}
          onStageChange={(s) => setStage(detailProject.id, s)}
          onProgressChange={(p) => setProjectProgress(detailProject.id, p)}
          onClose={closeDetail}
        />
      )}
    </motion.div>
  );
}

// ── 详情抽屉 ────────────────────────────────────

function ProjectDetailSheet({
  project,
  stage,
  progress,
  codeLines,
  version,
  gitInfo,
  activity,
  contributors,
  onStageChange,
  onProgressChange,
  onClose,
}: {
  project: Project;
  stage: StageKey;
  progress: number | undefined;
  codeLines?: CodeLineResult;
  version?: string;
  gitInfo?: ProjectGitInfo;
  activity?: number;
  contributors?: Contributor[];
  onStageChange: (stage: StageKey) => void;
  onProgressChange: (progress: number) => void;
  onClose: () => void;
}) {
  const { t: tr } = useTranslation();

  function fmt(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1).replace(/\.0$/, "") + "M";
    if (n >= 10_000) return (n / 1_000).toFixed(1).replace(/\.0$/, "") + "K";
    return n.toLocaleString();
  }

  function activityLabel(count: number): { text: string; color: string } {
    if (count >= 40)
      return { text: tr("kanban.activity.veryHigh", { defaultValue: "很高" }), color: "text-emerald-500" };
    if (count >= 11)
      return { text: tr("kanban.activity.high", { defaultValue: "高" }), color: "text-emerald-400" };
    if (count >= 1)
      return { text: tr("kanban.activity.medium", { defaultValue: "中" }), color: "text-amber-500" };
    return { text: tr("kanban.activity.low", { defaultValue: "低" }), color: "text-muted-foreground" };
  }

  return (
    <motion.div
      className="fixed inset-0 z-[60] flex justify-end"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
    >
      {/* 遮罩 */}
      <div className="absolute inset-0 bg-black/20 backdrop-blur-sm" />

      {/* 抽屉 */}
      <motion.div
        className="relative w-[480px] max-w-[90vw] h-full bg-background border-l border-border shadow-2xl overflow-y-auto"
        initial={{ x: "100%" }}
        animate={{ x: 0 }}
        exit={{ x: "100%" }}
        transition={{ type: "spring", damping: 25, stiffness: 200 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-6 space-y-6">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-lg font-semibold text-foreground">
                {project.name}
              </h2>
              <p className="text-sm text-muted-foreground font-mono mt-1">
                {project.path}
              </p>
            </div>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <svg
                viewBox="0 0 24 24"
                width={18}
                height={18}
                aria-hidden
              >
                <path
                  d="M18 6L6 18M6 6l12 12"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </svg>
            </Button>
          </div>

          {/* 阶段选择 */}
          <div>
            <h3 className="text-sm font-semibold text-foreground mb-2">
              {tr("kanban.stageLabel", { defaultValue: "项目阶段" })}
            </h3>
            <p className="text-xs text-muted-foreground mb-3">
              {tr("kanban.stageHint", {
                defaultValue: "选择项目当前所处的开发阶段",
              })}
            </p>
            <StagePicker value={stage} onChange={onStageChange} />
          </div>

          {/* 项目描述 */}
          {project.description && (
            <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
              <h3 className="text-sm font-semibold text-foreground mb-1">
                {tr("projects.description", { defaultValue: "项目描述" })}
              </h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {project.description}
              </p>
            </div>
          )}

          {/* MVP 进度编辑 */}
          {stage === "mvp" && (
            <div className="rounded-xl border border-purple-500/20 bg-purple-500/5 p-4">
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-semibold text-foreground">
                  {tr("kanban.progress", { defaultValue: "开发进度" })}
                </h3>
                <span className="text-lg font-bold text-purple-600 dark:text-purple-400 tabular-nums">
                  {progress ?? 0}%
                </span>
              </div>
              <input
                type="range"
                min={0}
                max={100}
                step={5}
                value={progress ?? 0}
                onChange={(e) => onProgressChange(Number(e.target.value))}
                className="w-full h-1.5 rounded-full appearance-none bg-muted/50 cursor-pointer
                  [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4
                  [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full
                  [&::-webkit-slider-thumb]:bg-purple-500 [&::-webkit-slider-thumb]:shadow-md
                  [&::-webkit-slider-thumb]:cursor-pointer"
              />
              <div className="flex justify-between mt-1.5">
                <span className="text-[10px] text-muted-foreground">0%</span>
                <span className="text-[10px] text-muted-foreground">100%</span>
              </div>
            </div>
          )}

          {/* 元数据 */}
          <div className="rounded-xl border border-border/60 bg-muted/20 p-4 space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">
                {tr("kanban.addedAt", { defaultValue: "添加时间" })}
              </span>
              <span className="text-foreground font-medium">
                {new Date(project.addedAt).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">
                {tr("kanban.projectPath", { defaultValue: "本地路径" })}
              </span>
              <span className="text-foreground font-medium font-mono text-xs truncate max-w-[250px]">
                {project.path}
              </span>
            </div>
          </div>

          {/* 代码指标 */}
          <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
            <div className="flex items-center gap-2 mb-3">
              <BarChart3 className="w-4 h-4 text-blue-500" />
              <h3 className="text-sm font-semibold text-foreground">
                {tr("kanban.metrics", { defaultValue: "代码指标" })}
              </h3>
            </div>

            <div className="grid grid-cols-2 gap-3">
              {/* 代码行数 */}
              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.codeLines", { defaultValue: "代码行数" })}
                </span>
                <span className="text-lg font-bold text-foreground mt-1 tabular-nums">
                  {codeLines ? fmt(codeLines.code_lines) : "—"}
                </span>
                {codeLines && (
                  <span className="text-[10px] text-muted-foreground/60">
                    {codeLines.files} {tr("kanban.files", { defaultValue: "个文件" })}
                    {" · "}
                    {codeLines.languages.length}{" "}
                    {tr("kanban.languages", { defaultValue: "种语言" })}
                  </span>
                )}
              </div>

              {/* 版本 */}
              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.version", { defaultValue: "版本" })}
                </span>
                <span className="text-lg font-bold text-foreground mt-1">
                  {version || "—"}
                </span>
              </div>

              {/* 活跃度 */}
              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.commits", { defaultValue: "30天活跃" })}
                </span>
                <span
                  className={`text-lg font-bold mt-1 tabular-nums ${
                    typeof activity === "number"
                      ? activityLabel(activity).color
                      : "text-foreground/25"
                  }`}
                >
                  {typeof activity === "number"
                    ? activityLabel(activity).text
                    : "—"}
                </span>
                {typeof activity === "number" && (
                  <span className="text-[10px] text-muted-foreground/60 tabular-nums">
                    {activity} {tr("kanban.commits", { defaultValue: "次提交" })}
                  </span>
                )}
              </div>

              {/* 贡献者 */}
              <div className="flex flex-col items-center p-3 rounded-lg bg-background/50 border border-border/40">
                <span className="text-xs text-muted-foreground">
                  {tr("kanban.metric.contributors", { defaultValue: "贡献者" })}
                </span>
                <span className="text-lg font-bold text-foreground mt-1 tabular-nums">
                  {contributors ? contributors.length : "—"}
                </span>
                {contributors && contributors.length > 0 && (
                  <span className="text-[10px] text-muted-foreground/60 truncate max-w-[120px]">
                    {contributors
                      .slice(0, 3)
                      .map((c) => c.name)
                      .join(", ")}
                  </span>
                )}
              </div>
            </div>

            {/* 语言分布 */}
            {codeLines && codeLines.languages.length > 0 && (
              <div className="mt-3 pt-3 border-t border-border/40">
                <span className="text-xs text-muted-foreground mb-2 block">
                  {tr("kanban.languageBreakdown", { defaultValue: "语言分布" })}
                </span>
                <div className="space-y-1.5">
                  {codeLines.languages.slice(0, 6).map((lang) => {
                    const pct = codeLines.code_lines > 0
                      ? Math.round((lang.code_lines / codeLines.code_lines) * 100)
                      : 0;
                    return (
                      <div
                        key={lang.language}
                        className="flex items-center gap-2 text-xs"
                      >
                        <span className="w-16 text-muted-foreground truncate">
                          {lang.language}
                        </span>
                        <div className="flex-1 h-1.5 rounded-full bg-muted/50 overflow-hidden">
                          <div
                            className="h-full rounded-full bg-blue-500/60"
                            style={{ width: `${Math.max(pct, 2)}%` }}
                          />
                        </div>
                        <span className="w-10 text-right tabular-nums text-muted-foreground">
                          {pct}%
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          {/* Git 信息 */}
          {gitInfo && gitInfo.is_repo && (
            <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
              <h3 className="text-sm font-semibold text-foreground mb-3">
                {tr("kanban.gitInfo", { defaultValue: "Git 仓库" })}
              </h3>
              <div className="space-y-2 text-sm">
                {gitInfo.branch && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      {tr("kanban.branch", { defaultValue: "当前分支" })}
                    </span>
                    <span className="text-foreground font-mono font-medium">
                      {gitInfo.branch}
                    </span>
                  </div>
                )}
                {gitInfo.remote_url && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      {tr("kanban.remote", { defaultValue: "远程地址" })}
                    </span>
                    <span className="text-foreground font-mono text-xs truncate max-w-[240px]">
                      {gitInfo.remote_url}
                    </span>
                  </div>
                )}
                {gitInfo.last_commit_hash && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      {tr("kanban.lastCommit", { defaultValue: "最近提交" })}
                    </span>
                    <span className="text-foreground font-mono text-xs">
                      {gitInfo.last_commit_hash.slice(0, 7)}
                    </span>
                  </div>
                )}
                {gitInfo.last_commit_message && (
                  <p className="text-xs text-muted-foreground/70 leading-relaxed pt-1 border-t border-border/30">
                    {gitInfo.last_commit_message}
                  </p>
                )}
                {gitInfo.last_commit_author && gitInfo.last_commit_date && (
                  <p className="text-xs text-muted-foreground/50">
                    {gitInfo.last_commit_author} · {gitInfo.last_commit_date}
                  </p>
                )}
              </div>
            </div>
          )}

          {/* 代码指标未扫描提示 */}
          {!codeLines && !gitInfo?.is_repo && (
            <p className="text-xs text-muted-foreground/60 text-center py-2">
              {tr("kanban.scanHint", {
                defaultValue:
                  "点击页面顶部「刷新指标」按钮扫描项目代码数据",
              })}
            </p>
          )}
        </div>
      </motion.div>
    </motion.div>
  );
}

// ── 汇总卡片子组件 ────────────────────────────

function SummaryCard({
  label,
  value,
  unit,
  sub,
  color,
}: {
  label: string;
  value: string;
  unit?: string;
  sub?: string;
  color?: string;
}) {
  return (
    <div className="rounded-xl border border-border/60 bg-card/40 p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={`text-2xl font-bold mt-1 flex items-baseline gap-1 ${color ?? "text-foreground"}`}>
        {value}
        {unit && (
          <span className="text-xs font-normal text-muted-foreground">
            {unit}
          </span>
        )}
      </div>
      {sub && (
        <div className="text-[10px] text-muted-foreground/60 mt-0.5">
          {sub}
        </div>
      )}
    </div>
  );
}
