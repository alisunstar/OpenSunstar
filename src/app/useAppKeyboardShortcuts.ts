import { useEffect } from "react";

import type { PageView } from "./navigation";

interface UseAppKeyboardShortcutsOptions {
  currentView: PageView;
  onNavigate: (view: PageView) => void;
  onToggleShortcuts: () => void;
}

const ALT_VIEW_SHORTCUTS: Record<string, PageView> = {
  "1": "mcp",
  "2": "prompts",
  "3": "skills",
  "4": "sessions",
  "5": "tokenStats",
  "6": "kanban",
};

export function useAppKeyboardShortcuts({
  currentView,
  onNavigate,
  onToggleShortcuts,
}: UseAppKeyboardShortcutsOptions): void {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "b") {
        event.preventDefault();
        window.dispatchEvent(new Event("toggle-sidebar"));
        return;
      }

      if (
        ((event.ctrlKey || event.metaKey) && event.key === "/") ||
        (!event.ctrlKey && !event.metaKey && !event.altKey && event.key === "?")
      ) {
        event.preventDefault();
        onToggleShortcuts();
        return;
      }

      if (event.altKey && !event.ctrlKey && !event.metaKey) {
        const view = ALT_VIEW_SHORTCUTS[event.key];
        if (view !== undefined) {
          event.preventDefault();
          onNavigate(view);
          return;
        }
      }

      if (event.key !== "Escape" || event.defaultPrevented) return;
      if (document.body.style.overflow === "hidden") return;
      if (currentView === "mcpDiscovery") onNavigate("mcp");
      if (currentView === "skillsDiscovery") onNavigate("skills");
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [currentView, onNavigate, onToggleShortcuts]);
}
