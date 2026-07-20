import { act, screen, waitFor } from "@testing-library/react";
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
    // Sidebar workspace section/menu label (post-refactor: "跨项目工作区").
    expect(screen.getAllByText("跨项目工作区").length).toBeGreaterThan(0);
  });

  it("does not throw when background sync status events fire", async () => {
    const { default: App } = await import("@/App");
    renderAppWithProviders(App);

    await waitFor(() =>
      expect(screen.getByTestId("workspace-page")).toBeInTheDocument(),
    );

    emitTauriEvent("webdav-sync-status-updated", {
      source: "auto",
      status: "error",
      error: "network timeout",
    });

    // SyncStatusBar updates inline state; global auto-sync toasts are not wired yet.
    expect(toastErrorMock).not.toHaveBeenCalled();

    emitTauriEvent("s3-sync-status-updated", {
      source: "auto",
      status: "error",
      error: "s3 timeout",
    });
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
