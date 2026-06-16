import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useProjectPaths } from "./projectPathsStorage";

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "Project";
}

const AUTO_OPEN_MS = 4000;

export function SwitcherApp() {
  const projectPaths = useProjectPaths();
  const [selectedIndex, setSelectedIndex] = useState(0);
  const pathsRef = useRef(projectPaths);
  const switcherWindow = getCurrentWebviewWindow();

  useEffect(() => { pathsRef.current = projectPaths; }, [projectPaths]);

  // Transparent body for this window
  useEffect(() => {
    document.body.style.background = "transparent";
    document.documentElement.style.background = "transparent";
  }, []);

  const dismiss = () => {
    switcherWindow.hide().catch(() => {});
  };

  const openSelected = () => {
    const paths = pathsRef.current;
    if (paths.length === 0) { dismiss(); return; }
    const idx = Math.min(selectedIndex, paths.length - 1);
    const path = paths[idx];
    switcherWindow.emit("switcher-navigate", path).catch(() => {});
    dismiss();
  };

  // ── Global shortcut listener (cycles within this popup) ──
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listen("global-cmd-k", () => {
      const count = pathsRef.current.length;
      if (count > 0) {
        setSelectedIndex((prev) => (prev + 1) % count);
      }
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  // ── Auto-open timer ──
  useEffect(() => {
    if (projectPaths.length === 0) return;
    const timer = setTimeout(openSelected, AUTO_OPEN_MS);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIndex, projectPaths]);

  // ── Keyboard: Escape / Enter ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        dismiss();
      } else if (e.key === "Enter") {
        e.preventDefault();
        openSelected();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIndex, projectPaths]);

  // ── Scroll active card into view ──
  useEffect(() => {
    const el = document.querySelector(".gps-item--active");
    el?.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
  }, [selectedIndex]);

  // ── Listen for navigate from main window ──
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen<{ path: string }>("switcher-navigate", (event) => {
      const { path } = event.payload;
      switcherWindow.emit("main-navigate", path).catch(() => {});
      dismiss();
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="switcher-root">
      <div className="gps-hint">⌘K 切换项目 · Enter 打开 · Esc 关闭</div>
      <div className="gps-list">
        {projectPaths.map((path, index) => (
          <div
            key={path}
            className={`gps-item${index === selectedIndex ? " gps-item--active" : ""}`}
            onClick={openSelected}
          >
            <span className="gps-item__name">{folderBasename(path)}</span>
            <span className="gps-item__path">{path}</span>
          </div>
        ))}
      </div>
      <div className="gps-counter">{selectedIndex + 1} / {projectPaths.length}</div>
    </div>
  );
}
