import { afterEach, describe, expect, it } from "vitest";

import {
  getInitialApp,
  getInitialView,
  isAgentConfigView,
  PAGE_META,
} from "./navigation";

describe("application navigation contract", () => {
  afterEach(() => localStorage.clear());

  it("restores only supported persisted views and preserves the legacy settings migration", () => {
    localStorage.setItem("OpenSunstar-ext-last-view", "teamCollaboration");
    expect(getInitialView()).toBe("teamCollaboration");

    localStorage.setItem("OpenSunstar-ext-last-view", "syncBackup");
    expect(getInitialView()).toBe("settings");

    localStorage.setItem("OpenSunstar-ext-last-view", "unknown-view");
    expect(getInitialView()).toBe("kanban");
  });

  it("keeps the team entry outside global Agent configuration scope", () => {
    expect(PAGE_META.teamCollaboration.defaultTitle).toBe("Team Collaboration");
    expect(PAGE_META.prompts.defaultTitle).toBe("Prompt & Rules");
    expect(isAgentConfigView("teamCollaboration")).toBe(false);
    expect(isAgentConfigView("prompts")).toBe(true);
  });

  it("falls back to Claude when the persisted target app is unsupported", () => {
    localStorage.setItem("OpenSunstar-ext-last-app", "unsupported");
    expect(getInitialApp()).toBe("claude");
  });
});
