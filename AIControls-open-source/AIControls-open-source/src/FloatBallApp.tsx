import type { CSSProperties, MouseEvent, PointerEvent } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openProjectPath } from "./api/openProject";
import { getOpenAppForProject } from "./projectOpenAppStorage";
import {
  matchProjectPathForCwd,
  useProjectPaths,
} from "./projectPathsStorage";

const floatWindow = getCurrentWindow();
const FLOAT_BALL_HOVER_EVENT = "float-ball-hover-state";
const CLAUDE_EXECUTION_STATE_EVENT = "claude-execution-state";
const CLAUDE_COMPLETION_PENDING_EVENT = "claude-completion-pending";
const FLOAT_BALL_SHELL_WIDTH = 224;
const FLOAT_BALL_SHELL_HEIGHT = 286;
const FLOAT_BALL_SIZE = 34;
const FLOAT_BALL_PROJECT_GAP = 7;
const FLOAT_BALL_MAIN_TO_PROJECTS_GAP = 10;
const FLOAT_BALL_MAIN_BOTTOM = 20;
const FLOAT_BALL_PROJECTS_BOTTOM =
  FLOAT_BALL_MAIN_BOTTOM + FLOAT_BALL_SIZE + FLOAT_BALL_MAIN_TO_PROJECTS_GAP;
const FLOAT_BALL_DRAG_THRESHOLD = 5;

type FloatBallHoverPayload =
  | boolean
  | {
      inside: boolean;
      x: number;
      y: number;
    };

type ClaudeCompletionPendingPayload = {
  cwd: string;
  sessionId?: string | null;
};

type ClaudeExecutionStatePayload = {
  cwd: string;
  sessionId?: string | null;
  state: string;
  toolName?: string | null;
};

type ClaudeExecutionVisualState = "running" | "waiting";

type HoverTarget = "main" | `project-${number}` | null;

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "Project";
}

function projectInitial(name: string): string {
  return name.trim().slice(0, 1).toLocaleUpperCase() || "P";
}

function pointHitsBall(x: number, y: number, centerX: number, centerY: number) {
  const radius = FLOAT_BALL_SIZE / 2;
  return Math.hypot(x - centerX, y - centerY) <= radius;
}

function hoverTargetFromPayload(
  payload: FloatBallHoverPayload,
  projectCount: number,
): { hovering: boolean; target: HoverTarget } {
  if (typeof payload === "boolean") {
    return { hovering: payload, target: payload ? "main" : null };
  }

  if (!payload.inside) {
    return { hovering: false, target: null };
  }

  const centerX = FLOAT_BALL_SHELL_WIDTH / 2;
  const mainCenterY =
    FLOAT_BALL_SHELL_HEIGHT - FLOAT_BALL_MAIN_BOTTOM - FLOAT_BALL_SIZE / 2;
  if (pointHitsBall(payload.x, payload.y, centerX, mainCenterY)) {
    return { hovering: true, target: "main" };
  }

  const firstProjectCenterY =
    FLOAT_BALL_SHELL_HEIGHT -
    FLOAT_BALL_PROJECTS_BOTTOM -
    FLOAT_BALL_SIZE / 2;
  const projectStep = FLOAT_BALL_SIZE + FLOAT_BALL_PROJECT_GAP;
  for (let index = 0; index < projectCount; index += 1) {
    const centerY = firstProjectCenterY - index * projectStep;
    if (pointHitsBall(payload.x, payload.y, centerX, centerY)) {
      return { hovering: true, target: `project-${index}` };
    }
  }

  return { hovering: true, target: null };
}

export default function FloatBallApp() {
  const projectPaths = useProjectPaths();
  const projects = useMemo(() => projectPaths.slice(0, 5), [projectPaths]);
  const [expanded, setExpanded] = useState(false);
  const [hovering, setHovering] = useState(false);
  const [hoverTarget, setHoverTarget] = useState<HoverTarget>(null);
  const [pendingCompletionProjectPath, setPendingCompletionProjectPath] = useState<string | null>(null);
  const [activeExecution, setActiveExecution] = useState<{
    projectPath: string;
    state: ClaudeExecutionVisualState;
  } | null>(null);
  const expandedRef = useRef(false);
  const collapseTimerRef = useRef<number | null>(null);
  const openingProjectRef = useRef(false);
  const suppressHoverExpansionRef = useRef(false);
  const dragStartRef = useRef<{
    pointerId: number;
    x: number;
    y: number;
  } | null>(null);
  const draggedMainBallRef = useRef(false);
  const projectsCountRef = useRef(0);
  projectsCountRef.current = projects.length;

  const clearCollapseTimer = () => {
    if (collapseTimerRef.current !== null) {
      window.clearTimeout(collapseTimerRef.current);
      collapseTimerRef.current = null;
    }
  };

  const expandMenu = () => {
    clearCollapseTimer();
    if (expandedRef.current || projectsCountRef.current === 0) return;
    expandedRef.current = true;
    setExpanded(true);
  };

  const collapseMenu = () => {
    clearCollapseTimer();
    if (!expandedRef.current) return;
    expandedRef.current = false;
    setExpanded(false);
  };

  const scheduleCollapse = () => {
    clearCollapseTimer();
    collapseTimerRef.current = window.setTimeout(collapseMenu, 140);
  };

  const startDragging = () => {
    draggedMainBallRef.current = true;
    floatWindow.startDragging().catch(() => {
      // Dragging is a native-window affordance; ignore unsupported edge cases.
    });
  };

  const startPotentialDrag = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;
    draggedMainBallRef.current = false;
    dragStartRef.current = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
    };
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // Pointer capture can fail if the native window starts handling the drag.
    }
  };

  const maybeStartDragging = (event: PointerEvent<HTMLButtonElement>) => {
    const dragStart = dragStartRef.current;
    if (!dragStart || dragStart.pointerId !== event.pointerId) return;
    const distance = Math.hypot(
      event.clientX - dragStart.x,
      event.clientY - dragStart.y,
    );
    if (distance < FLOAT_BALL_DRAG_THRESHOLD) return;
    dragStartRef.current = null;
    startDragging();
  };

  const clearPotentialDrag = (event: PointerEvent<HTMLButtonElement>) => {
    if (dragStartRef.current?.pointerId === event.pointerId) {
      dragStartRef.current = null;
    }
    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      // It may already be released by the browser or native drag handling.
    }
  };

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<FloatBallHoverPayload>(FLOAT_BALL_HOVER_EVENT, (event) => {
      if (disposed) return;
      const nextHover = hoverTargetFromPayload(
        event.payload,
        projectsCountRef.current,
      );
      if (!nextHover.hovering) {
        suppressHoverExpansionRef.current = false;
      }
      if (nextHover.hovering && suppressHoverExpansionRef.current) {
        if (nextHover.target === "main") {
          suppressHoverExpansionRef.current = false;
        } else {
          setHovering(false);
          setHoverTarget(null);
          return;
        }
      }
      setHovering(nextHover.hovering);
      setHoverTarget(nextHover.target);
      if (nextHover.hovering) {
        expandMenu();
      } else {
        scheduleCollapse();
      }
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      })
      .catch(() => {
        // DOM hover still works on platforms that deliver pointer events normally.
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<ClaudeExecutionStatePayload>(
      CLAUDE_EXECUTION_STATE_EVENT,
      (event) => {
        if (disposed) return;
        const projectPath = matchProjectPathForCwd(event.payload.cwd, projectPaths);
        if (!projectPath) return;

        const rawState = event.payload.state;
        if (rawState === "waiting" || rawState === "error") {
          setActiveExecution({ projectPath, state: "waiting" });
          return;
        }
        if (rawState === "running" || rawState === "tool") {
          setActiveExecution({ projectPath, state: "running" });
        }
      },
    )
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      })
      .catch(() => {
        // Ignore if the native bridge is unavailable.
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [projectPaths]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<ClaudeCompletionPendingPayload>(
      CLAUDE_COMPLETION_PENDING_EVENT,
      (event) => {
        if (disposed) return;
        const projectPath = matchProjectPathForCwd(event.payload.cwd, projectPaths);
        setPendingCompletionProjectPath(projectPath);
        if (projectPath) {
          setActiveExecution(null);
        }
      },
    )
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      })
      .catch(() => {
        // Ignore if the native bridge is unavailable.
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [projectPaths]);

  const openProjectWithSavedApp = (path: string) => {
    const customApp = getOpenAppForProject(path);
    return openProjectPath(path, {
      applicationPath: customApp ?? null,
      alertOnError: true,
    });
  };

  const openProject = (
    event: MouseEvent<HTMLButtonElement> | PointerEvent<HTMLButtonElement>,
    path: string,
  ) => {
    event.preventDefault();
    event.stopPropagation();
    if (openingProjectRef.current) return;
    openingProjectRef.current = true;
    void openProjectWithSavedApp(path).finally(() => {
      window.setTimeout(() => {
        openingProjectRef.current = false;
      }, 600);
    });
    suppressHoverExpansionRef.current = true;
    setHovering(false);
    setHoverTarget(null);
    collapseMenu();
  };

  const handleMainBallClick = (event: MouseEvent<HTMLButtonElement>) => {
    if (draggedMainBallRef.current) {
      draggedMainBallRef.current = false;
      return;
    }
    if (!pendingCompletionProjectPath || openingProjectRef.current) return;

    event.preventDefault();
    event.stopPropagation();
    openingProjectRef.current = true;
    const projectPath = pendingCompletionProjectPath;
    void openProjectWithSavedApp(projectPath).finally(() => {
      setPendingCompletionProjectPath(null);
      window.setTimeout(() => {
        openingProjectRef.current = false;
      }, 600);
    });
    suppressHoverExpansionRef.current = true;
    setHovering(false);
    setHoverTarget(null);
    collapseMenu();
  };

  const handleEnter = () => {
    suppressHoverExpansionRef.current = false;
    setHovering(true);
    setHoverTarget("main");
    expandMenu();
  };

  const handleLeave = () => {
    setHovering(false);
    setHoverTarget(null);
    scheduleCollapse();
  };

  const floatBallStateClass = useMemo(() => {
    if (activeExecution?.state === "waiting") {
      return " float-ball--waiting";
    }
    if (pendingCompletionProjectPath) {
      return " float-ball--notifying";
    }
    if (activeExecution?.state === "running") {
      return " float-ball--executing";
    }
    return "";
  }, [activeExecution, pendingCompletionProjectPath]);

  return (
    <div
      className={`float-ball-shell${expanded ? " float-ball-shell--expanded" : ""}${hovering ? " float-ball-shell--hovering" : ""}`}
      onPointerEnter={handleEnter}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
    >
      <div
        className="float-ball-projects"
        style={{ bottom: FLOAT_BALL_PROJECTS_BOTTOM } as CSSProperties}
        aria-label="Project shortcuts"
      >
        {projects.map((path, index) => {
          const name = folderBasename(path);
          return (
            <button
              key={path}
              type="button"
              className={`float-ball-project${hoverTarget === `project-${index}` ? " float-ball-project--hovering" : ""}`}
              style={{ "--float-project-index": index } as CSSProperties}
              title={`${name}\n${path}`}
              aria-label={`Open project ${name}`}
              onPointerEnter={() => setHoverTarget(`project-${index}`)}
              onMouseEnter={() => setHoverTarget(`project-${index}`)}
              onPointerDown={(event) => openProject(event, path)}
              onClick={(event) => openProject(event, path)}
            >
              <span className="float-ball-project__initial" aria-hidden>
                {projectInitial(name)}
              </span>
              <span className="float-ball-project__name">{name}</span>
            </button>
          );
        })}
      </div>

      <button
        type="button"
        className={`float-ball${hoverTarget === "main" ? " float-ball--hovering" : ""}${floatBallStateClass}`}
        aria-label={
          pendingCompletionProjectPath
            ? "Claude finished — click to open project"
            : "AIControls floating ball"
        }
        aria-expanded={expanded}
        onPointerEnter={() => setHoverTarget("main")}
        onMouseEnter={() => setHoverTarget("main")}
        onPointerDown={startPotentialDrag}
        onPointerMove={maybeStartDragging}
        onPointerUp={clearPotentialDrag}
        onPointerCancel={clearPotentialDrag}
        onClick={handleMainBallClick}
      >
        <span className="float-ball__core" aria-hidden />
        <span className="float-ball__logo" aria-hidden>
          <svg viewBox="0 0 32 32" role="presentation" focusable="false">
            <g transform="translate(0 -0.4)">
              <path
                d="M24.8 8.6 19.9 5.8 13.8 6.7 9.1 10.9 7 16.7 8.9 23 13.6 26.9 20.1 27.7 25.1 25 22.6 20.7 19.1 22.2 15.3 21.7 12.8 19.5 12 16.3 13.2 13.1 15.8 10.9 19.2 10.4 22.5 12 24.8 8.6Z"
                fill="currentColor"
              />
              <path
                d="M19.9 5.8 19.2 10.4 22.5 12 24.8 8.6 19.9 5.8Z"
                fill="rgba(255,255,255,0.28)"
              />
              <path
                d="M9.1 10.9 13.2 13.1 12 16.3 7 16.7 9.1 10.9Z"
                fill="rgba(255,255,255,0.22)"
              />
              <path
                d="M13.6 26.9 15.3 21.7 19.1 22.2 20.1 27.7 13.6 26.9Z"
                fill="rgba(0,0,0,0.18)"
              />
            </g>
          </svg>
        </span>
        <span className="float-ball__sheen" aria-hidden />
      </button>
    </div>
  );
}
