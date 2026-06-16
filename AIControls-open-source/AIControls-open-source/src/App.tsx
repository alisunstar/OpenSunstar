import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import {
  NavLink,
  Route,
  Routes,
  useLocation,
  useParams,
  useSearchParams,
} from "react-router-dom";
import AddProjectNavButton from "./components/AddProjectNavButton";
import BrandLogo from "./components/BrandLogo";
import AgentNavLinks from "./components/AgentNavLinks";
import ProjectNavItem from "./components/ProjectNavItem";
import {
  NavIconHome,
  NavIconBoard,
  NavIconLayers,
  NavIconFolder,
  NavIconPrompt,
  NavIconSettings,
} from "./components/navIcons";
import {
  appendProjectPath,
  pathsReferToSameDir,
  useProjectPaths,
} from "./projectPathsStorage";
import ShellPage from "./views/ShellPage";
import SettingsPage from "./views/SettingsPage";
import SkillBrowseShell from "./views/SkillBrowseShell";
import PromptLibraryPage from "./views/PromptLibraryPage";
import ResourceLibraryPage from "./views/ResourceLibraryPage";
import ProjectBoardPage from "./views/ProjectBoardPage";
import { useI18n } from "./i18n/provider";
import { listDetectedAgents } from "./api/agents";

function navClass(active: boolean) {
  return `side-nav-link${active ? " active" : ""}`;
}

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "Project";
}

function readBoolFromLocalStorage(key: string, fallback: boolean): boolean {
  try {
    const v = window.localStorage.getItem(key);
    if (v === "1") return true;
    if (v === "0") return false;
    return fallback;
  } catch {
    return fallback;
  }
}

function Layout({ children }: { children: ReactNode }) {
  const { t } = useI18n();
  const [searchParams] = useSearchParams();
  const location = useLocation();
  const { pathname, search } = location;
  const pathFromUrl = searchParams.get("path");
  const projectPaths = useProjectPaths();
  const [pendingActivePath, setPendingActivePath] = useState<string | null>(null);
  const [agentsCollapsed, setAgentsCollapsed] = useState(() => {
    if (typeof window === "undefined") return false;
    return readBoolFromLocalStorage("aicontrols-nav-collapse-agents", false);
  });
  const [projectsCollapsed, setProjectsCollapsed] = useState(() => {
    if (typeof window === "undefined") return false;
    return readBoolFromLocalStorage("aicontrols-nav-collapse-projects", false);
  });

  useEffect(() => {
    document.documentElement.dataset.theme = "light";
    document.documentElement.style.colorScheme = "light";
  }, []);

  useEffect(() => {
    if (pathFromUrl) {
      appendProjectPath(pathFromUrl);
    }
  }, [pathFromUrl]);

  useEffect(() => {
    if (pathname === "/board") {
      setAgentsCollapsed(true);
      try {
        window.localStorage.setItem("aicontrols-nav-collapse-agents", "1");
      } catch {
        // ignore
      }
    }
  }, [pathname]);

  useEffect(() => {
    const currentPath = `${pathname}${search}`;
    if (
      pendingActivePath === currentPath ||
      (pendingActivePath != null && !pendingActivePath.includes("?") && pendingActivePath === pathname)
    ) {
      setPendingActivePath(null);
    }
  }, [pathname, search, pendingActivePath]);

  const activeProjectPath =
    pathname === "/project" ? searchParams.get("path") : null;
  const navLinkClass = (targetPath: string, isActive: boolean) =>
    navClass(pendingActivePath ? pendingActivePath === targetPath : isActive);

  return (
    <div className="app-shell">
      <aside
        className="side-nav"
        aria-label={t("nav.main")}
        onDragStartCapture={(e) => e.preventDefault()}
      >
        <div className="side-nav__primary">
          <div className="side-nav-brand">
            <div className="side-nav-brand__mark" aria-hidden>
              <BrandLogo />
            </div>
            <div className="side-nav-brand__text">
              <span className="side-nav-brand__name">AIControls</span>
              <span className="side-nav-brand__tag">{t("nav.tagline")}</span>
            </div>
          </div>
          <NavLink
            to="/"
            end
            className={({ isActive }) => navLinkClass("/", isActive)}
            onPointerDown={() => setPendingActivePath("/")}
            onClick={() => setPendingActivePath("/")}
          >
            <span className="side-nav-link__icon">
              <NavIconHome />
            </span>
            <span className="side-nav-link__label side-nav-link__label--cjk-optical">
              {t("nav.home")}
            </span>
          </NavLink>
          <NavLink
            to="/board"
            className={({ isActive }) => navLinkClass("/board", isActive)}
            onPointerDown={() => setPendingActivePath("/board")}
            onClick={() => setPendingActivePath("/board")}
          >
            <span className="side-nav-link__icon">
              <NavIconBoard />
            </span>
            <span className="side-nav-link__label side-nav-link__label--cjk-optical">
              {t("nav.board")}
            </span>
          </NavLink>
          <NavLink
            to="/assets"
            className={({ isActive }) => navLinkClass("/assets", isActive)}
            onPointerDown={() => setPendingActivePath("/assets")}
            onClick={() => setPendingActivePath("/assets")}
          >
            <span className="side-nav-link__icon">
              <NavIconLayers />
            </span>
            <span className="side-nav-link__label side-nav-link__label--cjk-optical">
              {t("nav.assets")}
            </span>
          </NavLink>
          <NavLink
            to="/prompts"
            className={({ isActive }) => navLinkClass("/prompts", isActive)}
            onPointerDown={() => setPendingActivePath("/prompts")}
            onClick={() => setPendingActivePath("/prompts")}
          >
            <span className="side-nav-link__icon">
              <NavIconPrompt />
            </span>
            <span className="side-nav-link__label side-nav-link__label--cjk-optical">
              {t("nav.prompts")}
            </span>
          </NavLink>
          <NavLink
            to="/resources"
            className={({ isActive }) => navLinkClass("/resources", isActive)}
            onPointerDown={() => setPendingActivePath("/resources")}
            onClick={() => setPendingActivePath("/resources")}
          >
            <span className="side-nav-link__icon">
              <NavIconFolder />
            </span>
            <span className="side-nav-link__label side-nav-link__label--cjk-optical">
              {t("nav.resources")}
            </span>
          </NavLink>
        </div>

        <div className="side-nav__scroll">
          <button
            type="button"
            className="side-nav-section-toggle"
            aria-expanded={!agentsCollapsed}
            aria-controls="side-nav-agents"
            onClick={() => {
              setAgentsCollapsed((prev) => {
                const next = !prev;
                try {
                  window.localStorage.setItem(
                    "aicontrols-nav-collapse-agents",
                    next ? "1" : "0",
                  );
                } catch {
                  // ignore
                }
                return next;
              });
            }}
          >
            <span>Agent</span>
            <span className="side-nav-section-toggle__chevron" aria-hidden>
              ▾
            </span>
          </button>
          <div id="side-nav-agents" hidden={agentsCollapsed}>
            <AgentNavLinks
              pendingActivePath={pendingActivePath}
              onPendingActivePath={setPendingActivePath}
            />
          </div>
          <div
            className={`side-nav__projects${projectsCollapsed ? " side-nav__projects--collapsed" : ""}`}
            aria-label={t("nav.projects")}
          >
            <button
              type="button"
              className="side-nav-section-toggle side-nav__projects-heading"
              aria-expanded={!projectsCollapsed}
              aria-controls="side-nav-projects"
              onClick={() => {
                setProjectsCollapsed((prev) => {
                  const next = !prev;
                  try {
                    window.localStorage.setItem(
                      "aicontrols-nav-collapse-projects",
                      next ? "1" : "0",
                    );
                  } catch {
                    // ignore
                  }
                  return next;
                });
              }}
            >
              <span>{t("nav.projects")}</span>
              <span className="side-nav-section-toggle__chevron" aria-hidden>
                ▾
              </span>
            </button>
            <div
              id="side-nav-projects"
              className="side-nav__projects-list"
              hidden={projectsCollapsed}
            >
              {projectPaths.map((p) => {
                const isCurrent =
                  activeProjectPath !== null &&
                  pathsReferToSameDir(activeProjectPath, p);
                return (
                  <ProjectNavItem
                    key={p}
                    projectPath={p}
                    isCurrent={isCurrent}
                    pendingActivePath={pendingActivePath}
                    onPendingActivePath={setPendingActivePath}
                  />
                );
              })}
              <AddProjectNavButton />
            </div>
          </div>
        </div>

        <div className="side-nav-footer">
          <NavLink
            to="/settings"
            className={({ isActive }) => navLinkClass("/settings", isActive)}
            title={t("nav.settings")}
            onPointerDown={() => setPendingActivePath("/settings")}
            onClick={() => setPendingActivePath("/settings")}
          >
            <span className="side-nav-link__icon">
              <NavIconSettings />
            </span>
            <span className="side-nav-link__label side-nav-link__label--cjk-optical">
              {t("nav.settings")}
            </span>
          </NavLink>
        </div>
      </aside>
      <div className="main-wrap">
        <main>{children}</main>
      </div>
    </div>
  );
}

const AGENT_TITLES: Record<string, string> = {
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

function AgentRoute() {
  const { ecosystem } = useParams();
  const id = ecosystem?.trim() ?? "";
  const [title, setTitle] = useState(
    () => (id && AGENT_TITLES[id] ? AGENT_TITLES[id] : id || "Agent"),
  );

  useEffect(() => {
    if (!id) {
      setTitle("Agent");
      return;
    }
    if (AGENT_TITLES[id]) {
      setTitle(AGENT_TITLES[id]);
      return;
    }
    let cancelled = false;
    listDetectedAgents().then((agents) => {
      if (cancelled) return;
      const row = agents?.find((a) => a.id === id);
      setTitle(row?.label ?? id);
    });
    return () => {
      cancelled = true;
    };
  }, [id]);

  return (
    <SkillBrowseShell title={title} ecosystem={id || undefined} dataSet="skills" />
  );
}

function ProjectRoute() {
  const [sp] = useSearchParams();
  const path = sp.get("path");
  const folderTitle =
    path != null && path.length > 0 ? folderBasename(path) : "Project";

  return (
    <SkillBrowseShell
      title={folderTitle}
      dataSet="project"
      projectRoot={path ?? undefined}
      subtitle={
        path ? `Path: ${path}` : "Please add a local project folder from the sidebar."
      }
    />
  );
}

export default function App() {
  useEffect(() => {
    const suppressNativeContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };
    document.addEventListener("contextmenu", suppressNativeContextMenu, { capture: true });
    return () =>
      document.removeEventListener("contextmenu", suppressNativeContextMenu, { capture: true });
  }, []);

  return (
    <Layout>
      <Routes>
        <Route path="/" element={<ShellPage title="Home" />} />
        <Route path="/board" element={<ProjectBoardPage />} />
        <Route
          path="/assets"
          element={<SkillBrowseShell title="Assets" dataSet="aggregate" />}
        />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="/prompts" element={<PromptLibraryPage />} />
        <Route path="/resources" element={<ResourceLibraryPage />} />
        <Route path="/agent/:ecosystem" element={<AgentRoute />} />
        <Route path="/project" element={<ProjectRoute />} />
      </Routes>
    </Layout>
  );
}
