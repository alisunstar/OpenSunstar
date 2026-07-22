import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AppPageActions, hasPageActions } from "./AppPageActions";
import type { PageActionRefs } from "./pageActionRefs";

const refs = {} as PageActionRefs;

describe("AppPageActions", () => {
  it("exposes header actions for every legacy add-panel route", () => {
    expect(hasPageActions("ignore")).toBe(true);
    expect(hasPageActions("permissions")).toBe(true);
    expect(hasPageActions("teamCollaboration")).toBe(false);
  });

  it("navigates to MCP discovery from the route-owned action bar", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    render(<AppPageActions view="mcp" refs={refs} onNavigate={onNavigate} />);

    await user.click(screen.getByRole("button", { name: "发现MCP" }));

    expect(onNavigate).toHaveBeenCalledWith("mcpDiscovery");
  });
});
