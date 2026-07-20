import { screen, act, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { SyncStatusBar } from "@/components/layout/SyncStatusBar";
import { renderWithProviders } from "../renderWithProviders";
import { server } from "../msw/server";
import { emitTauriEvent } from "../msw/tauriMocks";

/**
 * Integration coverage for the cross-device sync status surface.
 *
 * Drives the real `SyncStatusBar` through the mocked Tauri `invoke`
 * (`get_settings`) → MSW and the mocked event bus, asserting the live status
 * machine (idle → syncing → success → error) reacts to backend push events.
 */

const TAURI_ENDPOINT = "http://tauri.local";

const withWebdavAutoSync = () =>
  server.use(
    http.post(`${TAURI_ENDPOINT}/get_settings`, () =>
      HttpResponse.json({
        showInTray: true,
        language: "zh",
        webdavSync: {
          enabled: true,
          autoSync: true,
          status: { lastSyncAt: null, lastError: null },
        },
      }),
    ),
  );

describe("Sync status flow integration (invoke + event bus)", () => {
  it("hides itself when no auto-sync backend is configured", async () => {
    // Default MSW get_settings has no webdavSync/s3Sync → disabled → renders null.
    const { container } = renderWithProviders(<SyncStatusBar collapsed={false} />);
    await waitFor(() => {
      expect(container).toBeEmptyDOMElement();
    });
  });

  it("shows the active WebDAV backend in the idle state after reading settings", async () => {
    withWebdavAutoSync();
    renderWithProviders(<SyncStatusBar collapsed={false} />);

    // Backend name + status label share one span, so match on substring.
    expect(await screen.findByText(/WebDAV/)).toBeInTheDocument();
    expect(screen.getByText(/sync\.waiting/)).toBeInTheDocument();
  });

  it("transitions to success then error as backend push events arrive", async () => {
    withWebdavAutoSync();
    renderWithProviders(<SyncStatusBar collapsed={false} />);
    await screen.findByText(/WebDAV/);

    act(() => {
      emitTauriEvent("webdav-sync-status-updated", {
        source: "auto",
        status: "success",
      });
    });
    expect(await screen.findByText(/sync\.synced/)).toBeInTheDocument();

    act(() => {
      emitTauriEvent("webdav-sync-status-updated", {
        source: "auto",
        status: "error",
        error: "network timeout",
      });
    });
    expect(await screen.findByText(/sync\.failed/)).toBeInTheDocument();
  });

  it("ignores S3 events while the WebDAV backend is active", async () => {
    withWebdavAutoSync();
    renderWithProviders(<SyncStatusBar collapsed={false} />);
    await screen.findByText(/WebDAV/);

    act(() => {
      emitTauriEvent("s3-sync-status-updated", {
        source: "auto",
        status: "error",
        error: "s3 boom",
      });
    });

    // WebDAV backend stays in its idle state; the S3 event does not flip it.
    expect(screen.getByText(/sync\.waiting/)).toBeInTheDocument();
    expect(screen.queryByText(/sync\.failed/)).not.toBeInTheDocument();
  });
});
