import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import type { PageView } from "./navigation";
import { useAppKeyboardShortcuts } from "./useAppKeyboardShortcuts";

function ShortcutHarness({ initialView }: { initialView: PageView }) {
  const [view, setView] = useState(initialView);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  useAppKeyboardShortcuts({
    currentView: view,
    onNavigate: setView,
    onToggleShortcuts: () => setShortcutsOpen((open) => !open),
  });
  return <output>{`${view}:${shortcutsOpen}`}</output>;
}

describe("useAppKeyboardShortcuts", () => {
  it("routes Alt shortcuts without putting shortcut state in App", () => {
    render(<ShortcutHarness initialView="kanban" />);

    fireEvent.keyDown(window, { altKey: true, key: "1" });

    expect(screen.getByText("mcp:false")).toBeInTheDocument();
  });

  it("returns discovery routes to their parent page on Escape", () => {
    render(<ShortcutHarness initialView="skillsDiscovery" />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(screen.getByText("skills:false")).toBeInTheDocument();
  });
});
