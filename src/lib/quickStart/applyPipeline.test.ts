import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { runQuickStartApplyPipeline } from "./applyPipeline";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@/utils/uuid", () => ({
  generateUUID: vi
    .fn()
    .mockReturnValueOnce("provider-id")
    .mockReturnValueOnce("idempotency-id"),
}));

describe("QuickStart transactional apply pipeline", () => {
  beforeEach(() => vi.clearAllMocks());

  it("does not report takeover success when backend compensated the operation", async () => {
    vi.mocked(invoke).mockResolvedValue({
      id: "operation-id",
      status: "rolled_back",
      takeoverEnabled: false,
      revision: 5,
    });
    const invalidateQueries = vi.fn().mockResolvedValue(undefined);

    const result = await runQuickStartApplyPipeline(
      {
        appId: "codex",
        queryClient: { invalidateQueries } as never,
      },
      { name: "Provider", settingsConfig: {} },
      { providerId: "provider-id", idempotencyKey: "idempotency-id" },
    );

    expect(result.takeoverOk).toBe(false);
    expect(result.operation.status).toBe("rolled_back");
    expect(invoke).toHaveBeenCalledWith("quick_start_apply", {
      request: expect.objectContaining({
        appType: "codex",
        provider: expect.objectContaining({ id: "provider-id" }),
      }),
    });
  });

  it("polls the existing operation when an idempotent concurrent retry is still active", async () => {
    vi.useFakeTimers();
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        id: "operation-id",
        status: "applying",
        revision: 2,
      })
      .mockResolvedValueOnce({
        id: "operation-id",
        status: "succeeded",
        revision: 8,
        takeoverEnabled: true,
      });
    const invalidateQueries = vi.fn().mockResolvedValue(undefined);

    const pending = runQuickStartApplyPipeline(
      {
        appId: "codex",
        queryClient: { invalidateQueries } as never,
      },
      { name: "Provider", settingsConfig: {} },
      { providerId: "provider-id", idempotencyKey: "idempotency-id" },
    );
    await vi.advanceTimersByTimeAsync(500);
    const result = await pending;

    expect(result.operation.status).toBe("succeeded");
    expect(invoke).toHaveBeenNthCalledWith(2, "quick_start_get_operation", {
      operationId: "operation-id",
    });
    vi.useRealTimers();
  });
});
