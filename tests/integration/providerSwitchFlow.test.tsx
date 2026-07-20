import type { ReactNode } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nextProvider } from "react-i18next";
import i18n from "i18next";
import { http, HttpResponse } from "msw";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useProviderActions } from "@/hooks/useProviderActions";
import type { Provider } from "@/types";
import { server } from "../msw/server";
import { getCurrentProviderId } from "../msw/state";

/**
 * Integration coverage for the provider switch critical flow.
 *
 * `useProviderActions.test.tsx` mocks the mutation layer; here the real
 * `useProviderActions` → `useSwitchProviderMutation` → `providersApi.switch`
 * chain runs against the mocked Tauri `invoke` → MSW state, so a successful
 * switch actually mutates the shared current-provider state, and the safety
 * guardrails are verified to short-circuit before touching the backend.
 */

const TAURI_ENDPOINT = "http://tauri.local";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastWarningMock = vi.fn();
const toastInfoMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
    </QueryClientProvider>
  );
  return { wrapper };
}

const provider = (overrides: Partial<Provider> = {}): Provider => ({
  id: "claude-2",
  name: "Claude Custom",
  settingsConfig: {},
  category: "custom",
  ...overrides,
});

describe("Provider switch flow integration (invoke → MSW)", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
    toastInfoMock.mockReset();
  });

  it("switches the active provider through the real mutation and updates shared state", async () => {
    expect(getCurrentProviderId("claude")).toBe("claude-1");

    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => useProviderActions("claude", /* proxyRunning */ true, false),
      { wrapper },
    );

    await act(async () => {
      await result.current.switchProvider(provider({ id: "claude-2" }));
    });

    await waitFor(() => {
      expect(getCurrentProviderId("claude")).toBe("claude-2");
    });
    expect(toastSuccessMock).toHaveBeenCalledTimes(1);
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("blocks switching to an official provider while proxy takeover is active", async () => {
    let switchCalls = 0;
    server.use(
      http.post(`${TAURI_ENDPOINT}/switch_provider`, () => {
        switchCalls += 1;
        return HttpResponse.json(true);
      }),
    );

    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => useProviderActions("claude", true, /* proxyTakeover */ true),
      { wrapper },
    );

    await act(async () => {
      await result.current.switchProvider(
        provider({ id: "claude-1", name: "Claude Official", category: "official" }),
      );
    });

    expect(switchCalls).toBe(0);
    expect(toastErrorMock).toHaveBeenCalledTimes(1);
    // Current provider is untouched by the blocked switch.
    expect(getCurrentProviderId("claude")).toBe("claude-1");
  });

  it("warns that a non-official provider needs the proxy when it is not running", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => useProviderActions("claude", /* proxyRunning */ false, false),
      { wrapper },
    );

    await act(async () => {
      await result.current.switchProvider(
        provider({
          id: "claude-2",
          category: "custom",
          meta: { apiFormat: "openai_chat" },
        }),
      );
    });

    await waitFor(() => {
      expect(getCurrentProviderId("claude")).toBe("claude-2");
    });
    // The proxy-required warning replaces the success toast.
    expect(toastWarningMock).toHaveBeenCalledTimes(1);
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("reports a backend switch failure via the mutation error handler", async () => {
    server.use(
      http.post(`${TAURI_ENDPOINT}/switch_provider`, () =>
        HttpResponse.text("provider not found", { status: 404 }),
      ),
    );

    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => useProviderActions("claude", true, false),
      { wrapper },
    );

    await act(async () => {
      await result.current.switchProvider(provider({ id: "claude-2" }));
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledTimes(1);
    });
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });
});
