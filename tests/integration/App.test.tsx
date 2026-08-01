import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderAppWithProviders } from "../renderWithProviders";
import { emitTauriEvent } from "../msw/tauriMocks";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/onboarding/OnboardingWizard", () => ({
  OnboardingWizard: () => null,
}));

vi.mock("@/components/kanban/KanbanPage", () => ({
  KanbanPage: () => <div data-testid="workspace-page">Workspace</div>,
}));

vi.mock("@/components/team/TeamCollaborationPage", () => ({
  TeamCollaborationPage: () => (
    <div data-testid="team-collaboration-page">Team collaboration</div>
  ),
}));

describe("App integration with MSW", () => {
  // Full-app integration renders (real Sidebar w/ framer-motion, dialogs, the
  // whole provider stack + MSW round-trips) are heavy in jsdom and sit near the
  // 5s default; give this file headroom so it stays stable under full-suite load.
  vi.setConfig({ testTimeout: 20000 });

  beforeEach(() => {
    localStorage.clear();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
  });

  it("renders sidebar and default workspace view", async () => {
    const { default: App } = await import("@/App");
    renderAppWithProviders(App);

    await waitFor(() => {
      expect(screen.getByTestId("workspace-page")).toBeInTheDocument();
    });
    // 侧栏项目驾驶舱菜单名。分组标题只出现一次 —— 再出现两次就是空嵌套挂回来了。
    expect(screen.getAllByText("项目驾驶舱")).toHaveLength(1);
  });

  it("navigates from the sidebar into the team collaboration center", async () => {
    const user = userEvent.setup();
    const { default: App } = await import("@/App");
    renderAppWithProviders(App);

    await user.click(
      await screen.findByRole("button", { name: /团队协作配置/i }),
    );

    expect(
      await screen.findByTestId("team-collaboration-page"),
    ).toBeInTheDocument();
  });

  it("does not throw when background sync status events fire", async () => {
    const { default: App } = await import("@/App");
    renderAppWithProviders(App);

    await waitFor(() =>
      expect(screen.getByTestId("workspace-page")).toBeInTheDocument(),
    );

    await act(async () => {
      emitTauriEvent("webdav-sync-status-updated", {
        source: "auto",
        status: "error",
        error: "network timeout",
      });
      emitTauriEvent("s3-sync-status-updated", {
        source: "auto",
        status: "error",
        error: "s3 timeout",
      });
    });

    // SyncStatusBar updates inline state; global auto-sync toasts are not wired yet.
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("opens the deep-link import confirmation dialog for a provider import", async () => {
    const { default: App } = await import("@/App");
    renderAppWithProviders(App);

    await waitFor(() =>
      expect(screen.getByTestId("workspace-page")).toBeInTheDocument(),
    );

    // The legacy SimpleConnect page/event was removed; provider imports now flow
    // through the `deeplink-import` event handled by DeepLinkImportDialog.
    // No `config`/`configUrl` → the dialog opens directly without a merge call.
    await act(async () => {
      emitTauriEvent("deeplink-import", {
        version: "1",
        resource: "provider",
        app: "claude",
        name: "DeepSeek",
        homepage: "https://deepseek.com",
        endpoint: "https://api.deepseek.com/anthropic",
        apiKey: "sk-test-key-1234",
      });
    });

    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("DeepSeek")).toBeInTheDocument();
  });
});
