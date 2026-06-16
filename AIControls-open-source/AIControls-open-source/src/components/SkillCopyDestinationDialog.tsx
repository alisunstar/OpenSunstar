import { useEffect, useMemo, useState } from "react";
import { listVisibleProjectSkillBuckets } from "../api/agents";
import { useI18n } from "../i18n/provider";
import { normalizeProjectPath } from "../projectPathsStorage";
import type {
  CopySkillMenuSection,
  CopySkillTargetPayload,
} from "../skillCopyTargets";

/** 与 payload 对应的复制桶键，用于与后端返回的可见列表对齐 */
function bucketKey(payload: CopySkillTargetPayload): string {
  if (payload.destKind === "myLibrary" || payload.destKind === "promptGlobal") {
    return "";
  }
  return `${payload.agentId}:${payload.bucketIndex}`;
}

type ProjectGate = Set<string> | null | "loading";

export type SkillCopyDialogRow = {
  id: string;
  title: string;
  sourcePath?: string;
};

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "Project";
}

function projectPathFromKey(key: string): string | null {
  const prefix = "proj:";
  if (!key.startsWith(prefix)) return null;
  return key.slice(prefix.length);
}

function matchesFilter(query: string, text: string): boolean {
  const t = query.trim().toLowerCase();
  if (!t) return true;
  return text.toLowerCase().includes(t);
}

type Props = {
  row: SkillCopyDialogRow;
  sections: CopySkillMenuSection[];
  onClose: () => void;
  dialogTitle?: string;
  busy?: boolean;
  busyText?: string;
  /** 由父层串 sourcePath 并调用复制 */
  onChoose: (payload: CopySkillTargetPayload) => void;
};

export function SkillCopyDestinationDialog({
  row,
  sections,
  onClose,
  dialogTitle = "Copy to…",
  busy = false,
  busyText = "Processing…",
  onChoose,
}: Props) {
  const { locale } = useI18n();
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<"global" | "projects">("global");
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  /** 项目根 → 允许的 bucket 键；null = 检测失败沿用全部；loading = 检测中 */
  const [projectBucketGate, setProjectBucketGate] = useState<
    Record<string, ProjectGate>
  >({});

  const { mineSections, globalSections, projectSections, globalItemCount } =
    useMemo(() => {
      const mine = sections.filter((s) => s.key === "mine-library");
      const global = sections.filter((s) => s.key.startsWith("global"));
      const projects = sections.filter((s) => s.key.startsWith("proj:"));
      const globalItemCount =
        mine.reduce((n, s) => n + s.items.length, 0) +
        global.reduce((n, s) => n + s.items.length, 0);
      return {
        mineSections: mine,
        globalSections: global,
        projectSections: projects,
        globalItemCount,
      };
    }, [sections]);

  useEffect(() => {
    setQuery("");
    setExpandedKey(null);
    const projSecs = sections.filter((s) => s.key.startsWith("proj:"));
    const gItems =
      sections
        .filter((s) => s.key === "mine-library" || s.key.startsWith("global"))
        .reduce((n, s) => n + s.items.length, 0);
    setTab(projSecs.length > 0 && gItems === 0 ? "projects" : "global");
  }, [row.id, sections]);

  useEffect(() => {
    const roots = [
      ...new Set(
        sections
          .filter((s) => s.key.startsWith("proj:"))
          .map((s) => normalizeProjectPath(projectPathFromKey(s.key) ?? ""))
          .filter((r) => r.length > 0),
      ),
    ];
    if (roots.length === 0) {
      setProjectBucketGate({});
      return;
    }
    let cancelled = false;
    const loading: Record<string, ProjectGate> = {};
    for (const r of roots) loading[r] = "loading";
    setProjectBucketGate(loading);
    void (async () => {
      const entries = await Promise.all(
        roots.map(async (root) => {
          const rows = await listVisibleProjectSkillBuckets(root);
          if (rows === null) {
            return [root, null] as const;
          }
          const set = new Set(rows.map((x) => `${x.agentId}:${x.bucketIndex}`));
          return [root, set] as const;
        }),
      );
      if (cancelled) return;
      setProjectBucketGate(Object.fromEntries(entries));
    })();
    return () => {
      cancelled = true;
    };
  }, [sections, row.id]);

  const filteredGlobal = useMemo(() => {
    return globalSections
      .map((sec) => ({
        ...sec,
        items: sec.items.filter((it) => matchesFilter(query, it.label)),
      }))
      .filter((sec) => sec.items.length > 0);
  }, [globalSections, query]);

  const resolveVisibleProjectItems = (
    sec: CopySkillMenuSection,
  ): {
    gate: ProjectGate;
    items: CopySkillMenuSection["items"];
  } => {
    const root = normalizeProjectPath(projectPathFromKey(sec.key) ?? "");
    const gate: ProjectGate = projectBucketGate[root] ?? "loading";
    if (gate === "loading") {
      return { gate, items: [] };
    }
    if (gate === null) {
      return { gate, items: sec.items };
    }
    return {
      gate,
      items: sec.items.filter((it) => {
        const k = bucketKey(it.payload);
        return k.length > 0 && gate.has(k);
      }),
    };
  };

  const filteredMine = useMemo(() => {
    return mineSections
      .map((sec) => ({
        ...sec,
        items: sec.items.filter((it) => matchesFilter(query, it.label)),
      }))
      .filter((sec) => sec.items.length > 0);
  }, [mineSections, query]);

  const filteredProjects = useMemo(() => {
    return projectSections.filter((sec) => {
      const { gate, items } = resolveVisibleProjectItems(sec);
      const path = projectPathFromKey(sec.key) ?? "";
      const base = folderBasename(path);
      const q = query.trim().toLowerCase();
      if (!q) {
        if (gate !== "loading" && gate !== null && items.length === 0) {
          return false;
        }
        return true;
      }
      if (matchesFilter(query, sec.title)) return true;
      if (matchesFilter(query, path)) return true;
      if (matchesFilter(query, base)) return true;
      return items.some((it) => matchesFilter(query, it.label));
    });
  }, [projectSections, query, projectBucketGate]);

  useEffect(() => {
    if (tab !== "projects") return;
    const q = query.trim();
    if (!q) {
      setExpandedKey(null);
      return;
    }
    if (filteredProjects.length === 1) {
      setExpandedKey(filteredProjects[0]!.key);
    }
  }, [tab, query, filteredProjects]);

  const emptyAll = sections.every((s) => s.items.length === 0);
  const hasGlobal = globalItemCount > 0;
  const hasProjects = projectSections.length > 0;
  const showTabs = hasGlobal && hasProjects;
  const showGlobalPanel = hasGlobal && (!showTabs || tab === "global");
  const showProjectsPanel = hasProjects && (!showTabs || tab === "projects");

  const toggleProject = (key: string) => {
    setExpandedKey((prev) => (prev === key ? null : key));
  };

  return (
    <div className="skill-copy-dialog-root" role="presentation">
      <div
        className="skill-copy-dialog-backdrop"
        aria-hidden
        onClick={busy ? undefined : onClose}
      />
      <div
        className="skill-copy-dialog skill-copy-dialog--v2"
        role="dialog"
        aria-modal="true"
        aria-labelledby="skill-copy-dialog-title"
      >
        <button
          type="button"
          className="skill-copy-dialog__close"
          aria-label={locale === "zh" ? "关闭" : "Close"}
          onClick={onClose}
          disabled={busy}
        >
          ✕
        </button>
        <div className="skill-copy-dialog__header skill-copy-dialog__header--v2">
          <h2 id="skill-copy-dialog-title" className="skill-copy-dialog__title">
            {dialogTitle}
          </h2>
          <p className="skill-copy-dialog__subtitle">
            「{row.title}」
          </p>
          {busy ? (
            <p className="muted" role="status" aria-live="polite" style={{ margin: "0.25rem 0 0" }}>
              {busyText}
            </p>
          ) : null}
          <div className="skill-copy-dialog__toolbar">
            <input
              type="search"
              className="skill-copy-dialog__search"
              placeholder={
                locale === "zh"
                  ? "搜索文件夹名、路径或目标类型…"
                  : "Search folder, path, or target type…"
              }
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label="筛选复制目标"
              autoComplete="off"
              disabled={busy}
            />
            {showTabs ? (
              <div className="seg skill-copy-dialog__seg" role="tablist" aria-label="目标范围">
                <button
                  type="button"
                  role="tab"
                  aria-selected={tab === "global"}
                  className={`seg__item${tab === "global" ? " active" : ""}`}
                  onClick={() => setTab("global")}
                  disabled={busy}
                >
                  {locale === "zh" ? "用户全局" : "Global"}
                  <span className="skill-copy-dialog__tab-badge">{globalItemCount}</span>
                </button>
                <button
                  type="button"
                  role="tab"
                  aria-selected={tab === "projects"}
                  className={`seg__item${tab === "projects" ? " active" : ""}`}
                  onClick={() => setTab("projects")}
                  disabled={busy}
                >
                  {locale === "zh" ? "项目" : "Projects"}
                  <span className="skill-copy-dialog__tab-badge">{projectSections.length}</span>
                </button>
              </div>
            ) : null}
          </div>
        </div>

        <div className="skill-copy-dialog__body skill-copy-dialog__body--v2">
          {emptyAll ? (
            <p className="skill-copy-dialog__empty">
              当前没有可用的复制目标（例如未添加项目时不会出现「复制到项目」路径）。
            </p>
          ) : (
            <>
              {showGlobalPanel && (
                <div
                  className="skill-copy-dialog__panel"
                  role="tabpanel"
                  hidden={showTabs && tab !== "global"}
                  aria-hidden={showTabs && tab !== "global"}
                >
                  {filteredMine.length === 0 && filteredGlobal.length === 0 ? (
                    <p className="skill-copy-dialog__empty skill-copy-dialog__empty--inline">
                      {query.trim()
                        ? locale === "zh"
                          ? "没有匹配的全局目标，试试其它关键词或切换到「项目」。"
                          : "No matching global targets. Try another keyword or switch to Projects."
                        : locale === "zh"
                          ? "暂无全局目标。"
                          : "No global targets."}
                    </p>
                  ) : (
                    <>
                      {filteredMine.map((sec) => (
                        <div key={sec.key} className="skill-copy-dialog__block">
                          <div className="skill-copy-dialog__block-label">{sec.title}</div>
                          <div className="skill-copy-dialog__option-grid">
                            {sec.items.map((it) => (
                              <button
                                key={it.id}
                                type="button"
                                className="skill-copy-dialog__tile skill-copy-dialog__tile--accent"
                                disabled={busy}
                                onClick={() => onChoose(it.payload)}
                              >
                                {it.label}
                              </button>
                            ))}
                          </div>
                        </div>
                      ))}
                      {filteredGlobal.map((sec) => (
                        <div key={sec.key} className="skill-copy-dialog__block">
                          <div className="skill-copy-dialog__block-label">{sec.title}</div>
                          <div className="skill-copy-dialog__option-grid">
                            {sec.items.map((it) => (
                              <button
                                key={it.id}
                                type="button"
                                className="skill-copy-dialog__tile"
                                disabled={busy}
                                onClick={() => onChoose(it.payload)}
                              >
                                {it.label}
                              </button>
                            ))}
                          </div>
                        </div>
                      ))}
                    </>
                  )}
                </div>
              )}

              {showProjectsPanel && (
                <div
                  className="skill-copy-dialog__panel skill-copy-dialog__panel--projects"
                  role="tabpanel"
                  hidden={showTabs && tab !== "projects"}
                  aria-hidden={showTabs && tab !== "projects"}
                >
                  {projectSections.length === 0 ? (
                    <p className="skill-copy-dialog__empty skill-copy-dialog__empty--inline">
                      {locale === "zh"
                        ? "侧栏未添加项目时，无法复制到项目目录。可在左侧「添加项目」后加入文件夹。"
                        : 'Add a project from the sidebar to copy into its agent directories.'}
                    </p>
                  ) : filteredProjects.length === 0 ? (
                    <p className="skill-copy-dialog__empty skill-copy-dialog__empty--inline">
                      {locale === "zh"
                        ? "没有匹配的项目，请调整搜索词。"
                        : "No matching projects. Adjust your search."}
                    </p>
                  ) : (
                    <ul className="skill-copy-dialog__project-list">
                      {filteredProjects.map((sec) => {
                        const path = projectPathFromKey(sec.key) ?? "";
                        const expanded = expandedKey === sec.key;
                        const name = folderBasename(path);
                        const { gate, items: visibleItems } =
                          resolveVisibleProjectItems(sec);
                        const countLabel =
                          gate === "loading" ? "…" : String(visibleItems.length);
                        return (
                          <li key={sec.key} className="skill-copy-dialog__project-item">
                            <button
                              type="button"
                              className="skill-copy-dialog__project-trigger"
                              aria-expanded={expanded}
                              onClick={() => toggleProject(sec.key)}
                              disabled={busy}
                            >
                              <span
                                className="skill-copy-dialog__project-chevron"
                                aria-hidden
                              >
                                {expanded ? "▾" : "▸"}
                              </span>
                              <span className="skill-copy-dialog__project-trigger-text">
                                <span className="skill-copy-dialog__project-name">
                                  {name}
                                </span>
                                <span
                                  className="skill-copy-dialog__project-path"
                                  title={path}
                                >
                                  {path}
                                </span>
                              </span>
                              <span className="skill-copy-dialog__project-count">
                                {countLabel}
                              </span>
                            </button>
                            {expanded ? (
                              <div className="skill-copy-dialog__project-targets">
                                {gate === "loading" ? (
                                  <p className="skill-copy-dialog__gate-hint">
                                    {locale === "zh"
                                      ? "正在检测该仓库下的 Agent 目录…"
                                      : "Detecting agent directories…"}
                                  </p>
                                ) : visibleItems.length === 0 ? (
                                  <p className="skill-copy-dialog__gate-hint">
                                    {locale === "zh"
                                      ? "未发现用于存放技能的 Agent 目录（例如 .cursor、.claude）。此处只列出磁盘上已存在的配置文件夹。"
                                      : 'No agent skill directories found (e.g. .cursor, .claude). Only existing config folders are listed.'}
                                  </p>
                                ) : (
                                  <div className="skill-copy-dialog__option-grid">
                                    {visibleItems.map((it) => (
                                      <button
                                        key={it.id}
                                        type="button"
                                        className="skill-copy-dialog__tile"
                                        disabled={busy}
                                        onClick={() => onChoose(it.payload)}
                                      >
                                        {it.label}
                                      </button>
                                    ))}
                                  </div>
                                )}
                              </div>
                            ) : null}
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
