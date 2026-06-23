import { screen, waitFor } from "@testing-library/react";
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

vi.mock("@/components/simpleConnect/SimpleConnectPage", () => ({
  SimpleConnectPage: () => (
    <div data-testid="simple-connect-page">Simple Connect</div>
  ),
}));

vi.mock("@/components/kanban/KanbanPage", () => ({
  KanbanPage: () => <div data-testid="workspace-page">Workspace</div>,
}));

describe("App integration with MSW", () => {
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
    expect(screen.getAllByText("工作区").length).toBeGreaterThan(0);
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
});
