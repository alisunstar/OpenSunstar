import { QueryClient } from "@tanstack/react-query";
import { http, HttpResponse } from "msw";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  runQuickStartApplyPipeline,
  verifyQuickStartKey,
  type QuickStartFormFields,
  type QuickStartSelection,
} from "@/lib/quickStart";
import { server } from "../msw/server";

/**
 * Integration coverage for the QuickStart critical flow.
 *
 * Unlike the component-level `QuickStartPage.test.tsx` (which mocks the whole
 * `@/lib/quickStart` module), these tests drive the real pipeline + verify
 * functions through the mocked Tauri `invoke` → MSW HTTP layer, so the
 * request/response serialization and the api wrappers are exercised end to end.
 */

const TAURI_ENDPOINT = "http://tauri.local";

const succeededOperation = (overrides: Record<string, unknown> = {}) => ({
  id: "op-1",
  idempotencyKey: "idem-1",
  appType: "codex",
  providerId: "provider-1",
  status: "succeeded",
  currentStep: "done",
  revision: 3,
  providerCreated: true,
  providerSwitched: true,
  takeoverEnabled: true,
  takeoverWasEnabled: false,
  proxyStarted: true,
  proxyWasRunning: false,
  postVerified: true,
  createdAt: "2026-07-21T00:00:00Z",
  updatedAt: "2026-07-21T00:00:01Z",
  ...overrides,
});

const identity = { providerId: "provider-1", idempotencyKey: "idem-1" };

const t = (key: string, opts?: Record<string, unknown>) =>
  (opts?.defaultValue as string | undefined) ?? key;

describe("QuickStart flow integration (invoke → MSW)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("applies a provider end to end and invalidates the affected caches", async () => {
    const applyBodies: unknown[] = [];
    server.use(
      http.post(`${TAURI_ENDPOINT}/quick_start_apply`, async ({ request }) => {
        applyBodies.push(await request.json());
        return HttpResponse.json(succeededOperation());
      }),
    );

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const result = await runQuickStartApplyPipeline(
      { appId: "codex", queryClient },
      { name: "DeepSeek", settingsConfig: {} },
      identity,
    );

    expect(result.operation.status).toBe("succeeded");
    expect(result.takeoverOk).toBe(true);
    expect(result.providerId).toBe("provider-1");

    // Real invoke serialized the request through the MSW boundary.
    expect(applyBodies).toHaveLength(1);
    expect(applyBodies[0]).toMatchObject({
      request: {
        appType: "codex",
        idempotencyKey: "idem-1",
        provider: { id: "provider-1", name: "DeepSeek" },
      },
    });

    const invalidatedKeys = invalidateSpy.mock.calls.map(
      (call) => (call[0] as { queryKey?: unknown[] })?.queryKey,
    );
    expect(invalidatedKeys).toContainEqual(["providers", "codex"]);
    expect(invalidatedKeys).toContainEqual(["proxyStatus"]);
    expect(invalidatedKeys).toContainEqual(["proxyTakeover"]);
  });

  it("does not report takeover success when the backend compensated the operation", async () => {
    server.use(
      http.post(`${TAURI_ENDPOINT}/quick_start_apply`, () =>
        HttpResponse.json(
          succeededOperation({
            status: "rolled_back",
            takeoverEnabled: false,
            providerSwitched: false,
          }),
        ),
      ),
    );

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    const result = await runQuickStartApplyPipeline(
      { appId: "codex", queryClient },
      { name: "DeepSeek", settingsConfig: {} },
      identity,
    );

    expect(result.operation.status).toBe("rolled_back");
    expect(result.takeoverOk).toBe(false);
  });

  it("surfaces a backend apply failure to the caller", async () => {
    server.use(
      http.post(`${TAURI_ENDPOINT}/quick_start_apply`, () =>
        HttpResponse.text("provider store is locked", { status: 500 }),
      ),
    );

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    await expect(
      runQuickStartApplyPipeline(
        { appId: "codex", queryClient },
        { name: "DeepSeek", settingsConfig: {} },
        identity,
      ),
    ).rejects.toThrow(/provider store is locked/);
  });

  it("verifies an OpenAI-compatible key and returns the fetched model list", async () => {
    const verifyBodies: unknown[] = [];
    server.use(
      http.post(
        `${TAURI_ENDPOINT}/verify_provider_key`,
        async ({ request }) => {
          verifyBodies.push(await request.json());
          return HttpResponse.json({ ok: true, model_count: 2 });
        },
      ),
      http.post(`${TAURI_ENDPOINT}/fetch_models_for_config`, () =>
        HttpResponse.json([
          { id: "deepseek-v4-pro", ownedBy: "deepseek" },
          { id: "deepseek-v4-flash", ownedBy: "deepseek" },
        ]),
      ),
    );

    const selection: QuickStartSelection = { mode: "custom", appId: "codex" };
    const fields: QuickStartFormFields = {
      apiKey: "sk-test-key",
      customName: "My Gateway",
      customBaseUrl: "https://api.example.test/v1",
      customModel: "deepseek-v4-pro",
    };

    const outcome = await verifyQuickStartKey("codex", selection, fields, t);

    expect(outcome.ok).toBe(true);
    expect(outcome.protocol).toBe("openai");
    expect(outcome.models.map((m) => m.id)).toEqual([
      "deepseek-v4-pro",
      "deepseek-v4-flash",
    ]);
    expect(outcome.message).toContain("2");
    expect(verifyBodies[0]).toMatchObject({
      baseUrl: "https://api.example.test/v1",
      apiKey: "sk-test-key",
      protocol: "openai",
    });
  });

  it("reports an invalid key from the upstream verification without fetching models", async () => {
    const modelFetch = vi.fn(() => HttpResponse.json([]));
    server.use(
      http.post(`${TAURI_ENDPOINT}/verify_provider_key`, () =>
        HttpResponse.json({
          ok: false,
          model_count: 0,
          error: "401 unauthorized",
        }),
      ),
      http.post(`${TAURI_ENDPOINT}/fetch_models_for_config`, modelFetch),
    );

    const selection: QuickStartSelection = { mode: "custom", appId: "codex" };
    const fields: QuickStartFormFields = {
      apiKey: "sk-bad",
      customName: "",
      customBaseUrl: "https://api.example.test/v1",
      customModel: "",
    };

    const outcome = await verifyQuickStartKey("codex", selection, fields, t);

    expect(outcome.ok).toBe(false);
    expect(outcome.message).toBe("401 unauthorized");
    expect(outcome.models).toEqual([]);
    expect(modelFetch).not.toHaveBeenCalled();
  });

  it("guards against verifying before an API key is entered", async () => {
    const verify = vi.fn(() => HttpResponse.json({ ok: true, model_count: 0 }));
    server.use(http.post(`${TAURI_ENDPOINT}/verify_provider_key`, verify));

    const selection: QuickStartSelection = { mode: "custom", appId: "codex" };
    const fields: QuickStartFormFields = {
      apiKey: "   ",
      customName: "",
      customBaseUrl: "https://api.example.test/v1",
      customModel: "",
    };

    const outcome = await verifyQuickStartKey("codex", selection, fields, t);

    expect(outcome.ok).toBe(false);
    expect(outcome.message).toContain("API Key");
    expect(verify).not.toHaveBeenCalled();
  });
});
