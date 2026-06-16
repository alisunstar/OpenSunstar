import { act } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, describe, expect, it, vi } from "vitest";

const eventApiMock = vi.hoisted(() => {
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
  const listeners = new Map<
    string,
    (
      event:
        | { payload: FloatBallHoverPayload }
        | { payload: ClaudeCompletionPendingPayload }
        | { payload: ClaudeExecutionStatePayload },
    ) => void
  >();
  return {
    listeners,
    listen: vi.fn(
      (
        event: string,
        handler: (
          event:
            | { payload: FloatBallHoverPayload }
            | { payload: ClaudeCompletionPendingPayload }
            | { payload: ClaudeExecutionStatePayload },
        ) => void,
      ): Promise<() => void> => {
        listeners.set(event, handler);
        return Promise.resolve(() => {
          listeners.delete(event);
        });
      },
    ),
  };
});

const openProjectPath = vi.fn(() => Promise.resolve());
const invoke = vi.fn(() => Promise.resolve());
const startDragging = vi.fn(() => Promise.resolve());
const createPointerEvent = (type: string, init: MouseEventInit & { pointerId?: number }) => {
  const event = new MouseEvent(type, init);
  Object.defineProperty(event, "pointerId", {
    configurable: true,
    value: init.pointerId ?? 0,
  });
  return event;
};

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

vi.mock("@tauri-apps/api/dpi", () => ({
  PhysicalPosition: class PhysicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
  PhysicalSize: class PhysicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    startDragging,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: eventApiMock.listen,
}));

vi.mock("./api/openProject", () => ({
  openProjectPath,
}));

describe("FloatBallApp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventApiMock.listeners.clear();
    localStorage.clear();
    localStorage.setItem(
      "aicontrols:projectPaths",
      JSON.stringify([
        "/tmp/ProjectOne",
        "/tmp/ProjectTwo",
        "/tmp/ProjectThree",
        "/tmp/ProjectFour",
        "/tmp/ProjectFive",
        "/tmp/ProjectSix",
      ]),
    );
  });

  it("expands to five project shortcuts and opens a shortcut once", async () => {
    const { default: FloatBallApp } = await import("./FloatBallApp");
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(<FloatBallApp />);
    });

    const shell = host.querySelector(".float-ball-shell") as HTMLElement;
    expect(host.querySelectorAll(".float-ball-project")).toHaveLength(5);

    await Promise.resolve();

    await act(async () => {
      eventApiMock.listeners.get("float-ball-hover-state")?.({
        payload: { inside: true, x: 112, y: 249 },
      });
    });

    await Promise.resolve();
    expect(shell.classList.contains("float-ball-shell--expanded")).toBe(true);
    expect(shell.classList.contains("float-ball-shell--hovering")).toBe(true);
    expect(
      host.querySelector(".float-ball")?.classList.contains("float-ball--hovering"),
    ).toBe(true);

    await act(async () => {
      eventApiMock.listeners.get("float-ball-hover-state")?.({
        payload: { inside: true, x: 112, y: 205 },
      });
    });

    const projects = host.querySelectorAll(".float-ball-project");
    expect(projects[0]?.classList.contains("float-ball-project--hovering")).toBe(
      true,
    );
    expect(projects[1]?.classList.contains("float-ball-project--hovering")).toBe(
      false,
    );

    const firstProject = host.querySelector(".float-ball-project") as HTMLElement;
    await act(async () => {
      firstProject.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      firstProject.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(openProjectPath).toHaveBeenCalledTimes(1);
    expect(openProjectPath).toHaveBeenCalledWith("/tmp/ProjectOne", {
      applicationPath: null,
      alertOnError: true,
    });
    expect(shell.classList.contains("float-ball-shell--expanded")).toBe(false);

    await act(async () => {
      shell.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
    });
    expect(shell.classList.contains("float-ball-shell--expanded")).toBe(false);

    await act(async () => {
      eventApiMock.listeners.get("float-ball-hover-state")?.({
        payload: { inside: true, x: 112, y: 205 },
      });
    });
    expect(shell.classList.contains("float-ball-shell--expanded")).toBe(false);

    await act(async () => {
      eventApiMock.listeners.get("float-ball-hover-state")?.({
        payload: { inside: true, x: 112, y: 249 },
      });
    });
    expect(shell.classList.contains("float-ball-shell--expanded")).toBe(true);

    await act(async () => {
      eventApiMock.listeners.get("float-ball-hover-state")?.({
        payload: { inside: false, x: -1, y: -1 },
      });
    });
    await act(async () => {
      eventApiMock.listeners.get("float-ball-hover-state")?.({
        payload: { inside: true, x: 112, y: 249 },
      });
    });
    expect(shell.classList.contains("float-ball-shell--expanded")).toBe(true);

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  it("blinks when a completion event matches a remembered project and opens it on click", async () => {
    const { default: FloatBallApp } = await import("./FloatBallApp");
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(<FloatBallApp />);
    });

    const mainBall = host.querySelector(".float-ball") as HTMLElement;

    await act(async () => {
      eventApiMock.listeners.get("claude-completion-pending")?.({
        payload: { cwd: "/tmp/ProjectTwo/packages/app", sessionId: "session-1" },
      });
    });

    expect(mainBall.classList.contains("float-ball--notifying")).toBe(true);

    await act(async () => {
      mainBall.dispatchEvent(
        createPointerEvent("pointerdown", {
          bubbles: true,
          button: 0,
          pointerId: 1,
          clientX: 10,
          clientY: 10,
        }),
      );
      mainBall.dispatchEvent(
        createPointerEvent("pointerup", {
          bubbles: true,
          button: 0,
          pointerId: 1,
          clientX: 10,
          clientY: 10,
        }),
      );
      mainBall.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    expect(openProjectPath).toHaveBeenCalledWith("/tmp/ProjectTwo", {
      applicationPath: null,
      alertOnError: true,
    });
    expect(invoke).not.toHaveBeenCalled();
    expect(mainBall.classList.contains("float-ball--notifying")).toBe(false);

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  it("ignores completion events for unknown projects", async () => {
    const { default: FloatBallApp } = await import("./FloatBallApp");
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(<FloatBallApp />);
    });

    const mainBall = host.querySelector(".float-ball") as HTMLElement;

    await act(async () => {
      eventApiMock.listeners.get("claude-completion-pending")?.({
        payload: { cwd: "/tmp/UnknownProject", sessionId: "session-2" },
      });
    });

    expect(mainBall.classList.contains("float-ball--notifying")).toBe(false);

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  it("shows executing and waiting states for matched projects", async () => {
    const { default: FloatBallApp } = await import("./FloatBallApp");
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(<FloatBallApp />);
    });

    const mainBall = host.querySelector(".float-ball") as HTMLElement;

    await act(async () => {
      eventApiMock.listeners.get("claude-execution-state")?.({
        payload: { cwd: "/tmp/ProjectOne/src", state: "running" },
      });
    });

    expect(mainBall.classList.contains("float-ball--executing")).toBe(true);
    expect(mainBall.classList.contains("float-ball--waiting")).toBe(false);

    await act(async () => {
      eventApiMock.listeners.get("claude-execution-state")?.({
        payload: { cwd: "/tmp/ProjectOne", state: "waiting" },
      });
    });

    expect(mainBall.classList.contains("float-ball--executing")).toBe(false);
    expect(mainBall.classList.contains("float-ball--waiting")).toBe(true);

    await act(async () => {
      eventApiMock.listeners.get("claude-completion-pending")?.({
        payload: { cwd: "/tmp/ProjectOne", sessionId: "session-4" },
      });
    });

    expect(mainBall.classList.contains("float-ball--waiting")).toBe(false);
    expect(mainBall.classList.contains("float-ball--executing")).toBe(false);
    expect(mainBall.classList.contains("float-ball--notifying")).toBe(true);

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  it("does not activate project opening when the main ball was dragged", async () => {
    const { default: FloatBallApp } = await import("./FloatBallApp");
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(<FloatBallApp />);
    });

    const mainBall = host.querySelector(".float-ball") as HTMLElement;

    await act(async () => {
      eventApiMock.listeners.get("claude-completion-pending")?.({
        payload: { cwd: "/tmp/ProjectThree/src", sessionId: "session-3" },
      });
    });

    await act(async () => {
      mainBall.dispatchEvent(
        createPointerEvent("pointerdown", {
          bubbles: true,
          button: 0,
          pointerId: 2,
          clientX: 10,
          clientY: 10,
        }),
      );
      mainBall.dispatchEvent(
        createPointerEvent("pointermove", {
          bubbles: true,
          pointerId: 2,
          clientX: 30,
          clientY: 30,
        }),
      );
      mainBall.dispatchEvent(
        createPointerEvent("pointerup", {
          bubbles: true,
          button: 0,
          pointerId: 2,
          clientX: 30,
          clientY: 30,
        }),
      );
      mainBall.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(startDragging).toHaveBeenCalledTimes(1);
    expect(openProjectPath).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
    expect(mainBall.classList.contains("float-ball--notifying")).toBe(true);

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
});
