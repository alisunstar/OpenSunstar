import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router-dom";
import heroImage from "../../首页头图.png";
import { openProjectPath } from "../api/openProject";
import { revealPathInFolder } from "../api/reveal";
import {
  getAgentGlobalInventoryCached,
  invalidateCachedAgentGlobalInventory,
  invalidateCachedProjectInventory,
  getProjectLatestMtimeMsCached,
  scanProjectDirectoryCached,
} from "../api/agentInventoryCache";
import {
  detectGithubRepoSkills,
  importGithubSkillToDestination,
  listDetectedAgents,
  type AgentInventory,
  type AgentScanResult,
  type GithubSkillCandidate,
} from "../api/agents";
import { bucketInventoryByAgent, inventoryAssetCount } from "../agentAssetGrouping";
import { getOpenAppForProject, setOpenAppForProject } from "../projectOpenAppStorage";
import { useProjectPaths } from "../projectPathsStorage";
import { PageRefreshButton } from "../components/PageRefreshButton";
import HomeGiteeSyncHud from "../components/HomeGiteeSyncHud";
import {
  buildCopySkillMenuSections,
  type CopySkillTargetPayload,
} from "../skillCopyTargets";
import { SkillCopyDestinationDialog } from "../components/SkillCopyDestinationDialog";
import { useI18n } from "../i18n/provider";

type Props = {
  title: string;
  subtitle?: string;
};

const AGENT_LABEL_BY_ID: Record<string, string> = {
  cursor: "Cursor",
  claude: "Claude Code",
  codex: "Codex",
  hermes: "Hermes",
  openclaw: "OpenClaw",
  trae: "Trae",
  qoder: "Qoder",
  kiro: "Kiro",
  opencode: "opencode",
};

function fallbackAgentLabel(agentId: string): string {
  return AGENT_LABEL_BY_ID[agentId] ?? agentId;
}

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? path;
}

function summarizeInventory(inv: AgentInventory) {
  return {
    skills: inv.skills.length,
    mcp: inv.mcp.length,
    rules: inv.rules.length,
  };
}

/** 首页统计卡片 →「全部」页；与 `SkillBrowseShell` 的 `?kind=` 约定一致 */
function assetsPathForHomeMetric(metricKey: string): string {
  if (metricKey === "skills") return "/assets?kind=skill";
  if (metricKey === "mcp") return "/assets?kind=mcp";
  if (metricKey === "rules") return "/assets?kind=rule";
  return "/assets";
}

function appLabelFromPath(appPath: string): string {
  const base = appPath.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? appPath;
  return base.replace(/\.(app|exe)$/i, "");
}

type ProjectMenuState = { path: string; left: number; top: number };
type GithubDetectedSkillsModalState = {
  repoUrl: string;
  skills: GithubSkillCandidate[];
};
type PendingGithubImport = {
  repoUrl: string;
  skills: GithubSkillCandidate[];
};

type HomeLiveStatus = {
  message: string;
  detail?: string;
  completed: number;
  total: number;
};

function HomeLiveProgress({ status }: { status: HomeLiveStatus }) {
  const progress =
    status.total > 0
      ? Math.min(100, Math.round((status.completed / status.total) * 100))
      : 0;

  return (
    <div
      className="board-live-progress board-live-progress--scanning"
      role="status"
      aria-live="polite"
      aria-label={`首页扫描进度：${status.message}`}
    >
      <span className="board-live-progress__pulse" aria-hidden />
      <span className="board-live-progress__body">
        <strong>{status.message}</strong>
        {status.detail ? <em>{status.detail}</em> : null}
      </span>
      <span className="board-live-progress__meta">
        {status.total > 0 ? `${status.completed}/${status.total}` : "--"}
      </span>
      <span className="board-live-progress__bar" aria-hidden>
        <span style={{ width: `${progress}%` }} />
      </span>
    </div>
  );
}

export default function ShellPage({ subtitle }: Props) {
  const { locale } = useI18n();
  const navigate = useNavigate();
  const projectPaths = useProjectPaths();
  const [githubRepoUrl, setGithubRepoUrl] = useState("");
  const [projectMenu, setProjectMenu] = useState<ProjectMenuState | null>(null);
  const projectMenuRef = useRef<HTMLDivElement>(null);
  const [detectedAgents, setDetectedAgents] = useState<AgentScanResult[]>([]);
  const [totals, setTotals] = useState({ skills: 0, mcp: 0, rules: 0 });
  const [projectStats, setProjectStats] = useState<
    Record<
      string,
      {
        skills: number;
        mcp: number;
        rules: number;
        status: "ok" | "error";
        topAgent: string;
      }
    >
  >({});
  const [projectLatestMtimeMs, setProjectLatestMtimeMs] = useState<
    Record<string, number | null>
  >({});
  const [homeRefreshKey, setHomeRefreshKey] = useState(0);
  const [homeScanBusy, setHomeScanBusy] = useState(false);
  const [homeLiveStatus, setHomeLiveStatus] = useState<HomeLiveStatus>({
    message: locale === "zh" ? "等待扫描" : "Waiting",
    completed: 0,
    total: 0,
  });
  const [githubImportBusy, setGithubImportBusy] = useState(false);
  const [githubSkillPickModal, setGithubSkillPickModal] =
    useState<GithubDetectedSkillsModalState | null>(null);
  const [selectedGithubSkillIds, setSelectedGithubSkillIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [pendingGithubImport, setPendingGithubImport] =
    useState<PendingGithubImport | null>(null);
  const [homeToast, setHomeToast] = useState<{
    at: number;
    kind: "success" | "error";
    message: string;
  } | null>(null);

  useEffect(() => {
    let cancelled = false;
    listDetectedAgents().then((agents) => {
      if (cancelled) return;
      setDetectedAgents(agents ?? []);
    });
    return () => {
      cancelled = true;
    };
  }, [homeRefreshKey]);

  useEffect(() => {
    const bump = () => setHomeRefreshKey((k) => k + 1);
    window.addEventListener("aicontrols-agents-changed", bump);
    return () => window.removeEventListener("aicontrols-agents-changed", bump);
  }, []);

  useEffect(() => {
    let cancelled = false;
    const agentIds = detectedAgents.map((a) => a.id);
    const scanRoots = [...projectPaths];

    if (agentIds.length === 0 && scanRoots.length === 0) {
      setTotals({ skills: 0, mcp: 0, rules: 0 });
      setProjectStats({});
      setHomeScanBusy(false);
      return;
    }

    setHomeScanBusy(true);
    const agentInventories: Array<AgentInventory | null> = Array(agentIds.length).fill(null);
    const scanInventories: Array<AgentInventory | null> = Array(scanRoots.length).fill(null);
    const mtimeList: Array<number | null> = Array(scanRoots.length).fill(null);
    const totalTasks = agentIds.length + scanRoots.length * 2;
    let completed = 0;

    const updateProgress = (message: string, detail?: string) => {
      if (cancelled) return;
      setHomeLiveStatus({
        message,
        detail,
        completed,
        total: totalTasks,
      });
    };

    const finishTask = (message: string, detail?: string) => {
      completed += 1;
      updateProgress(message, detail);
      if (completed !== totalTasks || cancelled) return;

      const skillIds = new Set<string>();
      const mcpIds = new Set<string>();
      const ruleIds = new Set<string>();
      const addInventory = (inv: AgentInventory | null) => {
        if (!inv) return;
        for (const e of inv.skills) skillIds.add(e.id);
        for (const e of inv.mcp) mcpIds.add(e.id);
        for (const e of inv.rules) ruleIds.add(e.id);
      };

      for (const inv of agentInventories) addInventory(inv);
      for (const inv of scanInventories) addInventory(inv);

      const nextProjectStats: Record<
        string,
        {
          skills: number;
          mcp: number;
          rules: number;
          status: "ok" | "error";
          topAgent: string;
        }
      > = {};
      const nextProjectMtime: Record<string, number | null> = {};
      for (let i = 0; i < scanRoots.length; i += 1) {
        const root = scanRoots[i];
        const inv = scanInventories[i];
        nextProjectMtime[root] = mtimeList[i] ?? null;
        if (inv) {
          const topBucket = bucketInventoryByAgent(inv).sort(
            (a, b) => inventoryAssetCount(b.inv) - inventoryAssetCount(a.inv),
          )[0];
          nextProjectStats[root] = {
            ...summarizeInventory(inv),
            status: "ok",
            topAgent: topBucket ? fallbackAgentLabel(topBucket.agentId) : locale === "zh" ? "未识别" : "Unknown",
          };
        } else {
          nextProjectStats[root] = {
            skills: 0,
            mcp: 0,
            rules: 0,
            status: "error",
            topAgent: locale === "zh" ? "扫描失败" : "Scan failed",
          };
        }
      }

      setProjectStats(nextProjectStats);
      setProjectLatestMtimeMs(nextProjectMtime);
      setTotals({
        skills: skillIds.size,
        mcp: mcpIds.size,
        rules: ruleIds.size,
      });
      setHomeScanBusy(false);
    };

    updateProgress(locale === "zh" ? "准备扫描首页" : "Preparing scan", `${totalTasks} tasks`);

    agentIds.forEach((id, index) => {
      updateProgress(locale === "zh" ? "正在扫描 Agent 资产" : "Scanning agent assets", fallbackAgentLabel(id));
      void getAgentGlobalInventoryCached(id)
        .then((inv) => {
          if (cancelled) return;
          agentInventories[index] = inv;
          finishTask(locale === "zh" ? "已完成 Agent 资产" : "Agent assets done", fallbackAgentLabel(id));
        })
        .catch(() => {
          if (cancelled) return;
          finishTask(locale === "zh" ? "Agent 资产扫描失败" : "Agent assets failed", fallbackAgentLabel(id));
        });
    });

    scanRoots.forEach((root, index) => {
      updateProgress(locale === "zh" ? "正在扫描项目资产" : "Scanning project assets", folderBasename(root));
      void scanProjectDirectoryCached(root)
        .then((inv) => {
          if (cancelled) return;
          scanInventories[index] = inv;
          finishTask(locale === "zh" ? "已完成项目资产" : "Project assets done", folderBasename(root));
        })
        .catch(() => {
          if (cancelled) return;
          finishTask(locale === "zh" ? "项目资产扫描失败" : "Project assets failed", folderBasename(root));
        });

      updateProgress(locale === "zh" ? "正在读取最近修改" : "Reading latest update", folderBasename(root));
      void getProjectLatestMtimeMsCached(root)
        .then((mtime) => {
          if (cancelled) return;
          mtimeList[index] = mtime;
          finishTask(locale === "zh" ? "已读取最近修改" : "Latest update done", folderBasename(root));
        })
        .catch(() => {
          if (cancelled) return;
          finishTask(locale === "zh" ? "最近修改读取失败" : "Latest update failed", folderBasename(root));
        });
    });

    return () => {
      cancelled = true;
    };
  }, [detectedAgents, projectPaths, homeRefreshKey]);

  const formatProjectLatestUpdate = (root: string): string => {
    const ms = projectLatestMtimeMs[root];
    if (ms == null) return locale === "zh" ? "最近修改：—" : "Updated: —";
    const dt = new Date(ms);
    const pad2 = (n: number) => String(n).padStart(2, "0");
    const yy = pad2(dt.getFullYear() % 100);
    const mm = pad2(dt.getMonth() + 1);
    const dd = pad2(dt.getDate());
    const hh = pad2(dt.getHours());
    const mi = pad2(dt.getMinutes());
    return `UP:${yy}${mm}${dd}-${hh}:${mi}`;
  };

  const metrics = useMemo(
    () => [
      { key: "agent", label: "Agent", value: String(detectedAgents.length) },
      { key: "project", label: locale === "zh" ? "项目" : "Projects", value: String(projectPaths.length) },
      { key: "skills", label: "Skills", value: totals.skills.toLocaleString() },
      { key: "mcp", label: "MCP", value: totals.mcp.toLocaleString() },
      { key: "rules", label: "Rules", value: totals.rules.toLocaleString() },
    ],
    [detectedAgents.length, projectPaths.length, totals, locale],
  );

  const recentProjects = useMemo(
    () =>
      projectPaths.slice().reverse().map((path) => {
        const stat = projectStats[path];
        const assetCount = (stat?.skills ?? 0) + (stat?.mcp ?? 0) + (stat?.rules ?? 0);
        const topAgent =
          detectedAgents.length > 0 ? fallbackAgentLabel(detectedAgents[0].id) : locale === "zh" ? "未识别" : "Unknown";
        return {
          name: folderBasename(path),
          path,
          assets: assetCount,
          mcp: stat?.mcp ?? 0,
          rules: stat?.rules ?? 0,
          agent: stat?.topAgent ?? topAgent,
          updated: stat?.status === "error" ? (locale === "zh" ? "扫描失败" : "Scan failed") : formatProjectLatestUpdate(path),
        };
      }),
    [detectedAgents, projectPaths, projectStats, projectLatestMtimeMs, locale],
  );

  const renderMetricIcon = (key: string) => {
    switch (key) {
      case "agent":
        return (
          <svg viewBox="0 0 24 24" aria-hidden>
            <circle cx="12" cy="7.25" r="2.6" />
            <path d="M6.25 18.2a5.75 5.75 0 0 1 11.5 0" fill="none" strokeLinecap="round" />
            <path d="M3.9 10.8h3.15m13.05 0h-3.15m-5 8.35v-2.5" fill="none" strokeLinecap="round" />
          </svg>
        );
      case "project":
        return (
          <svg viewBox="0 0 24 24" aria-hidden>
            <rect x="4.4" y="5.2" width="15.2" height="13.6" rx="2.4" />
            <path d="M4.4 9.6h15.2M9.05 5.2l1.1 4.4" fill="none" strokeLinecap="round" />
            <circle cx="15.7" cy="14.15" r="1.55" />
          </svg>
        );
      case "skills":
        return (
          <svg viewBox="0 0 24 24" aria-hidden>
            <path d="M8.2 9.55 12 5.8l3.8 3.75M8.2 14.45 12 18.2l3.8-3.75" fill="none" strokeLinecap="round" />
            <path d="M5.9 12h12.2" fill="none" strokeLinecap="round" />
            <circle cx="12" cy="12" r="1.65" />
          </svg>
        );
      case "mcp":
        return (
          <svg viewBox="0 0 24 24" aria-hidden>
            <path d="M6.15 6.35h4.35v4.35H6.15zM13.5 6.35h4.35v4.35H13.5zM9.8 13.7h4.4v4.4H9.8z" />
            <path d="M10.5 8.55h3m-1.5 2.15v3.05" fill="none" strokeLinecap="round" />
          </svg>
        );
      default:
        return (
          <svg viewBox="0 0 24 24" aria-hidden>
            <path d="M6.45 4.9h11.1v14.2H6.45z" />
            <path d="M8.55 9.15h6.9m-6.9 3.2h6.9m-6.9 3.2h4.25" fill="none" strokeLinecap="round" />
          </svg>
        );
    }
  };

  const onRefreshHome = () => {
    invalidateCachedAgentGlobalInventory();
    invalidateCachedProjectInventory();
    setHomeRefreshKey((k) => k + 1);
  };

  useEffect(() => {
    if (!projectMenu) return;
    const close = () => setProjectMenu(null);
    const onPointerDown = (e: PointerEvent) => {
      if (projectMenuRef.current?.contains(e.target as Node)) return;
      close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKey);
    };
  }, [projectMenu]);

  const closeProjectMenu = () => setProjectMenu(null);

  const pickApplicationForProject = async (projectPath: string) => {
    closeProjectMenu();
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        title: locale === "zh" ? "选择用于打开该项目的应用程序" : "Choose app for this project",
      });
      if (selected === null) return;
      const appPath = Array.isArray(selected) ? selected[0] : selected;
      if (typeof appPath === "string" && appPath.length > 0) {
        setOpenAppForProject(projectPath, appPath);
      }
    } catch {
      const manual = window.prompt(
        locale === "zh"
          ? "请输入应用程序的完整路径（例如 /Applications/Cursor.app）："
          : "Enter full app path (e.g. /Applications/Cursor.app):",
      );
      const trimmed = manual?.trim();
      if (trimmed) setOpenAppForProject(projectPath, trimmed);
    }
  };

  const onOpenProjectCard = (projectPath: string) => {
    const customApp = getOpenAppForProject(projectPath);
    void openProjectPath(projectPath, {
      applicationPath: customApp ?? null,
      alertOnError: true,
    });
  };

  const onImportGithubSkill = () => {
    const trimmed = githubRepoUrl.trim();
    if (!trimmed || githubImportBusy) return;
    const normalized = /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
    void (async () => {
      setGithubImportBusy(true);
      const detected = await detectGithubRepoSkills(normalized);
      setGithubImportBusy(false);
      if ("error" in detected) {
        window.alert(`${locale === "zh" ? "识别失败" : "Detection failed"}: ${detected.error}`);
        return;
      }
      if (detected.skills.length === 0) {
        window.alert(locale === "zh" ? "未识别到 Skill（未找到 SKILL.md）" : "No Skill detected (SKILL.md not found)");
        return;
      }
      if (detected.skills.length === 1) {
        setPendingGithubImport({ repoUrl: normalized, skills: [detected.skills[0]!] });
        return;
      }
      setSelectedGithubSkillIds(new Set());
      setGithubSkillPickModal({ repoUrl: normalized, skills: detected.skills });
    })();
  };

  const userCustomAgentIds = useMemo(
    () => detectedAgents.filter((a) => a.id.startsWith("useragent-")).map((a) => a.id),
    [detectedAgents],
  );

  const homeImportMenuSections = useMemo(
    () =>
      buildCopySkillMenuSections({
        dataSet: "aggregate",
        projectPaths,
        projectRoot: undefined,
        ecosystem: undefined,
        agentProjectScanPaths: [],
        userCustomAgentIds,
        copyVerb: "导入",
        includeMyLibrary: true,
      }),
    [projectPaths, userCustomAgentIds],
  );

  const closeGithubSkillPickModal = () => setGithubSkillPickModal(null);

  const toggleGithubSkillSelection = (skillId: string) => {
    setSelectedGithubSkillIds((prev) => {
      const next = new Set(prev);
      if (next.has(skillId)) next.delete(skillId);
      else next.add(skillId);
      return next;
    });
  };

  const proceedWithSelectedGithubSkills = () => {
    const modal = githubSkillPickModal;
    if (!modal) return;
    const selected = modal.skills.filter((s) => selectedGithubSkillIds.has(s.id));
    if (selected.length === 0) return;
    setGithubSkillPickModal(null);
    setPendingGithubImport({ repoUrl: modal.repoUrl, skills: selected });
  };

  const importPendingGithubSkillToDestination = (
    payload: CopySkillTargetPayload,
  ) => {
    const pending = pendingGithubImport;
    if (!pending) return;
    if (payload.destKind === "promptGlobal") {
      setHomeToast({
        at: Date.now(),
        kind: "error",
        message:
          locale === "zh"
            ? "从 GitHub 导入的 Skill 仅支持「用户全局」「项目」或「我的 Skills」。"
            : "GitHub skills can only import to global, project, or My Skills.",
      });
      return;
    }
    void (async () => {
      setGithubImportBusy(true);
      let success = 0;
      const failed: string[] = [];
      for (const skill of pending.skills) {
        const common = {
          repoUrl: pending.repoUrl,
          skillPath: skill.path,
          onConflict: "suffix" as const,
        };
        const result =
          payload.destKind === "myLibrary"
            ? await importGithubSkillToDestination({
                ...common,
                destKind: "myLibrary",
                agentId: "",
                bucketIndex: 0,
              })
            : payload.destKind === "project"
              ? await importGithubSkillToDestination({
                  ...common,
                  destKind: "project",
                  agentId: payload.agentId,
                  bucketIndex: payload.bucketIndex,
                  projectRoot: payload.projectRoot,
                })
              : await importGithubSkillToDestination({
                  ...common,
                  destKind: "global",
                  agentId: payload.agentId,
                  bucketIndex: payload.bucketIndex,
                });
        if ("error" in result) {
          failed.push(`${skill.title}：${result.error}`);
          continue;
        }
        success += 1;
      }
      setGithubImportBusy(false);
      if (success === 0) {
        setHomeToast({
          at: Date.now(),
          kind: "error",
          message: `导入失败：${failed[0] ?? "未知错误"}`,
        });
        return;
      }
      setPendingGithubImport(null);
      setGithubRepoUrl("");
      setHomeToast({
        at: Date.now(),
        kind: failed.length > 0 ? "error" : "success",
        message:
          failed.length > 0
            ? `导入完成：成功 ${success} 个，失败 ${failed.length} 个`
            : `导入成功：${success} 个 Skill`,
      });
    })();
  };

  useEffect(() => {
    const hasModal = !!githubSkillPickModal || !!pendingGithubImport;
    if (!hasModal) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (pendingGithubImport) {
        setPendingGithubImport(null);
      } else {
        setGithubSkillPickModal(null);
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [githubSkillPickModal, pendingGithubImport]);

  useEffect(() => {
    if (!homeToast) return;
    const t = window.setTimeout(() => setHomeToast(null), 2200);
    return () => window.clearTimeout(t);
  }, [homeToast?.at]);

  return (
    <div className="home-board">
      <header className="home-board-hero">
        <div className="home-board-hero__toolbar">
          {homeScanBusy && <HomeLiveProgress status={homeLiveStatus} />}
          <PageRefreshButton
            onClick={onRefreshHome}
            disabled={homeScanBusy}
            spinning={homeScanBusy}
            label={locale === "zh" ? "重新扫描并加载" : "Rescan and reload"}
          />
        </div>
        <div className="home-board-hero__content">
          <h1 className="home-board-hello">
            {locale === "zh" ? "下午好，Controller" : "Good afternoon, Controller"} <span aria-hidden>👋</span>
          </h1>
          <p className="home-board-lead">
            {subtitle ??
              locale === "zh"
                ? `AIControls 已识别到 ${detectedAgents.length} 个 Agent，${projectPaths.length} 个项目`
                : `AIControls detected ${detectedAgents.length} agents and ${projectPaths.length} projects`}
          </p>
          <p className="home-board-sub">
            {locale === "zh"
              ? `聚合了 ${totals.skills.toLocaleString()} 个 Skills，${totals.mcp.toLocaleString()} 个 MCP，${totals.rules.toLocaleString()} 条 Rules`
              : `Aggregated ${totals.skills.toLocaleString()} skills, ${totals.mcp.toLocaleString()} MCP, and ${totals.rules.toLocaleString()} rules`}
          </p>
        </div>
        <div className="home-board-visual" aria-hidden>
          <img src={heroImage} alt="" className="home-board-visual__image" />
        </div>
      </header>

      <section className="home-board-metrics" aria-label={locale === "zh" ? "统计概览" : "Stats overview"}>
        {metrics.map((item) => {
          const to = assetsPathForHomeMetric(item.key);
          const navTitle =
            item.key === "skills" || item.key === "mcp" || item.key === "rules"
              ? `前往「全部」资产页（${item.label}）`
              : locale === "zh"
                ? "前往「全部」资产页"
                : "Go to Assets page";
          return (
            <article
              key={item.label}
              className={`home-board-metric home-board-metric--${item.key} home-board-metric--interactive`}
              role="button"
              tabIndex={0}
              title={navTitle}
              onClick={() => navigate(to)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  navigate(to);
                }
              }}
            >
              <p className="home-board-metric__label">
                <span className="home-board-metric__icon" aria-hidden>
                  {renderMetricIcon(item.key)}
                </span>
                {item.label}
              </p>
              <p className="home-board-metric__value">{item.value}</p>
            </article>
          );
        })}
      </section>

      <section className="home-board-github-import" aria-label={locale === "zh" ? "从 GitHub 导入 Skill" : "Import Skill from GitHub"}>
        <div className="home-board-github-import__head">
          <span className="home-board-github-import__icon" aria-hidden>
            <svg viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.21.68-.47v-1.65c-2.77.6-3.35-1.18-3.35-1.18-.46-1.15-1.1-1.46-1.1-1.46-.9-.62.07-.61.07-.61 1 .07 1.52 1.01 1.52 1.01.88 1.49 2.31 1.06 2.88.8.09-.63.35-1.06.63-1.3-2.21-.25-4.54-1.09-4.54-4.85 0-1.07.39-1.94 1.02-2.62-.1-.25-.44-1.27.1-2.64 0 0 .84-.26 2.75 1a9.63 9.63 0 0 1 5.02 0c1.91-1.26 2.75-1 2.75-1 .54 1.37.2 2.39.1 2.64.64.68 1.02 1.55 1.02 2.62 0 3.77-2.33 4.6-4.56 4.85.36.31.67.92.67 1.86v2.75c0 .26.18.57.69.47A10 10 0 0 0 12 2Z"
              />
            </svg>
          </span>
          <div>
            <p className="home-board-github-import__title">{locale === "zh" ? "从 GitHub 导入 Skill" : "Import Skill from GitHub"}</p>
            <p className="home-board-github-import__desc">
              {locale === "zh"
                ? "粘贴 GitHub 仓库链接，自动识别并导入 Skill 到你的库中"
                : "Paste a GitHub repo URL to detect and import skills"}
            </p>
          </div>
        </div>
        <div className="home-board-github-import__form">
          <input
            type="text"
            value={githubRepoUrl}
            onChange={(e) => setGithubRepoUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") onImportGithubSkill();
            }}
            className="home-board-github-import__input"
            placeholder="https://github.com/username/repo"
            aria-label={locale === "zh" ? "GitHub 仓库链接" : "GitHub repository URL"}
          />
          <button
            type="button"
            className="home-board-github-import__button"
            onClick={onImportGithubSkill}
            disabled={githubRepoUrl.trim().length === 0 || githubImportBusy}
          >
            {githubImportBusy ? (locale === "zh" ? "识别中…" : "Detecting…") : locale === "zh" ? "导入 Skill" : "Import Skill"}
          </button>
        </div>
      </section>

      <section className="home-board-projects" aria-label={locale === "zh" ? "最近项目" : "Recent projects"}>
        <div className="home-board-section-head">
          <h2>{locale === "zh" ? "最近项目" : "Recent projects"}</h2>
        </div>
        <div className="home-board-project-grid">
          {recentProjects.map((project) => (
            <article
              className="home-board-project-card home-board-project-card--interactive"
              key={project.path}
              role="button"
              tabIndex={0}
              onClick={() => onOpenProjectCard(project.path)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onOpenProjectCard(project.path);
                }
              }}
            >
              <div className="home-board-project-card__head">
                <div>
                  <h3>{project.name}</h3>
                  <p title={project.path}>{project.path}</p>
                </div>
                <button
                  type="button"
                  className="home-board-project-card__menu"
                  aria-label="更多操作"
                  aria-haspopup="menu"
                  aria-expanded={projectMenu?.path === project.path}
                  onClick={(e) => {
                    e.stopPropagation();
                    const r = e.currentTarget.getBoundingClientRect();
                    const menuWidth = 200;
                    const gap = 4;
                    const maxLeft = window.innerWidth - menuWidth - 8;
                    /** 菜单左缘从 ⋮ 按钮右缘向右展开，贴右屏时向左夹紧 */
                    const left = Math.max(8, Math.min(r.right + gap, maxLeft));
                    setProjectMenu((prev) =>
                      prev?.path === project.path
                        ? null
                        : { path: project.path, left, top: r.bottom + 4 },
                    );
                  }}
                >
                  ⋮
                </button>
              </div>
              <div className="home-board-project-card__stats">
                <span>● {project.assets}</span>
                <span>✦ {project.mcp}</span>
                <span>◈ {project.rules}</span>
              </div>
              <div className="home-board-project-card__foot">
                {(() => {
                  const customApp = getOpenAppForProject(project.path);
                  const label = customApp ? appLabelFromPath(customApp) : "VS Code";
                  const title = customApp
                    ? `打开应用：${customApp}`
                    : "打开应用：默认（VS Code → Cursor → 文件夹）";
                  return (
                    <span className="home-board-chip" title={title}>
                      {label}
                    </span>
                  );
                })()}
                <span>{project.updated}</span>
              </div>
            </article>
          ))}
        </div>
      </section>
      <HomeGiteeSyncHud />
      {githubSkillPickModal
        ? createPortal(
            <div className="prompt-create-modal-root">
              <div
                className="prompt-create-modal-backdrop"
                onClick={closeGithubSkillPickModal}
                aria-hidden
              />
              <div
                className="prompt-create-modal home-github-skill-pick-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="home-github-skill-pick-title"
              >
                <header className="prompt-create-modal__header">
                  <div className="prompt-create-modal__header-text">
                    <h2 id="home-github-skill-pick-title" className="prompt-create-modal__title">
                      选择要导入的 Skill
                    </h2>
                    <p className="prompt-create-modal__subtitle">
                      该仓库识别到多个 Skill，请勾选需要导入的项。
                    </p>
                  </div>
                  <button
                    type="button"
                    className="prompt-create-modal__close"
                    onClick={closeGithubSkillPickModal}
                    aria-label="关闭"
                  >
                    ✕
                  </button>
                </header>
                <div className="home-github-skill-pick-modal__meta" aria-live="polite">
                  已选择 {selectedGithubSkillIds.size} / {githubSkillPickModal.skills.length}
                </div>
                <div className="home-github-skill-pick-modal__list">
                  {githubSkillPickModal.skills.map((skill) => (
                    <button
                      key={skill.id}
                      type="button"
                      className={`home-github-skill-pick-modal__item${selectedGithubSkillIds.has(skill.id) ? " is-selected" : ""}`}
                      aria-pressed={selectedGithubSkillIds.has(skill.id)}
                      onClick={() => toggleGithubSkillSelection(skill.id)}
                    >
                      <span className="home-github-skill-pick-modal__item-main">
                        <span className="home-github-skill-pick-modal__item-title">
                          {skill.title}
                        </span>
                        <span className="home-github-skill-pick-modal__item-path">
                          {skill.path}
                        </span>
                      </span>
                      <span className="home-github-skill-pick-modal__check" aria-hidden>
                        {selectedGithubSkillIds.has(skill.id) ? "✓" : ""}
                      </span>
                    </button>
                  ))}
                </div>
                <footer className="prompt-create-modal__footer">
                  <div className="prompt-create-modal__actions">
                    <button
                      type="button"
                      className="prompt-create-modal__cancel"
                      onClick={closeGithubSkillPickModal}
                    >
                      取消
                    </button>
                    <button
                      type="button"
                      className="prompt-create-modal__submit"
                      onClick={proceedWithSelectedGithubSkills}
                      disabled={selectedGithubSkillIds.size === 0}
                    >
                      下一步
                    </button>
                  </div>
                </footer>
              </div>
            </div>,
            document.body,
          )
        : null}
      {pendingGithubImport
        ? createPortal(
            <SkillCopyDestinationDialog
              row={{
                id: `github:${pendingGithubImport.skills.map((s) => s.id).join(",")}`,
                title:
                  pendingGithubImport.skills.length === 1
                    ? pendingGithubImport.skills[0]!.title
                    : `已选 ${pendingGithubImport.skills.length} 个 Skill`,
                sourcePath: pendingGithubImport.skills[0]?.path,
              }}
              sections={homeImportMenuSections}
              dialogTitle="导入到…"
              busy={githubImportBusy}
              busyText={`正在导入 ${pendingGithubImport.skills.length} 个 Skill，请稍候…`}
              onClose={() => {
                if (!githubImportBusy) setPendingGithubImport(null);
              }}
              onChoose={importPendingGithubSkillToDestination}
            />,
            document.body,
          )
        : null}
      {homeToast
        ? createPortal(
            <div className="toast-stack" role="status" aria-live="polite">
              <div className={`toast ${homeToast.kind === "error" ? "toast--error" : "toast--success"}`}>
                <span className="toast__text">{homeToast.message}</span>
              </div>
            </div>,
            document.body,
          )
        : null}
      {projectMenu
        ? createPortal(
            <div
              ref={projectMenuRef}
              className="card-context-menu"
              style={{
                position: "fixed",
                left: projectMenu.left,
                top: projectMenu.top,
                zIndex: 10_000,
                minWidth: "11.5rem",
              }}
              role="menu"
              aria-label="项目打开方式"
              onClick={(e) => e.stopPropagation()}
            >
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                onClick={() => pickApplicationForProject(projectMenu.path)}
              >
                选择默认打开应用…
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                onClick={() => {
                  void revealPathInFolder(projectMenu.path, { alertOnError: true });
                  closeProjectMenu();
                }}
              >
                打开所在目录
              </button>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}
