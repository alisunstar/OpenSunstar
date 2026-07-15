import { invoke } from "@tauri-apps/api/core";
import type { QueryClient } from "@tanstack/react-query";
import type { Provider } from "@/types";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import { generateUUID } from "@/utils/uuid";

export type QuickStartOperationStatus =
  | "pending"
  | "applying"
  | "verifying"
  | "succeeded"
  | "failed"
  | "rolling_back"
  | "rolled_back"
  | "rollback_failed";

export interface QuickStartOperation {
  id: string;
  idempotencyKey: string;
  appType: QuickStartAppId;
  providerId?: string | null;
  previousProviderId?: string | null;
  status: QuickStartOperationStatus;
  currentStep: string;
  revision: number;
  providerCreated: boolean;
  providerSwitched: boolean;
  takeoverEnabled: boolean;
  takeoverWasEnabled: boolean;
  proxyStarted: boolean;
  proxyWasRunning: boolean;
  postVerified: boolean;
  errorCode?: string | null;
  errorMessage?: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt?: string | null;
}

export interface QuickStartOperationEvent {
  sequence: number;
  eventType: string;
  fromStatus?: QuickStartOperationStatus | null;
  toStatus?: QuickStartOperationStatus | null;
  step: string;
  errorCode?: string | null;
  errorMessage?: string | null;
  detailJson?: string | null;
  createdAt: string;
}

export interface QuickStartApplyDeps {
  appId: QuickStartAppId;
  queryClient: QueryClient;
}

export interface QuickStartApplyResult {
  operation: QuickStartOperation;
  takeoverOk: boolean;
  providerId: string;
  identity: QuickStartAttemptIdentity;
}

export interface QuickStartAttemptIdentity {
  providerId: string;
  idempotencyKey: string;
}

export function createQuickStartAttemptIdentity(): QuickStartAttemptIdentity {
  return {
    providerId: generateUUID(),
    idempotencyKey: generateUUID(),
  };
}

export async function runQuickStartApplyPipeline(
  deps: QuickStartApplyDeps,
  providerInput: Omit<Provider, "id"> & {
    ensureClaudeDesktopOfficialSeed?: boolean;
  },
  attemptIdentity = createQuickStartAttemptIdentity(),
): Promise<QuickStartApplyResult> {
  const { appId, queryClient } = deps;
  const { ensureClaudeDesktopOfficialSeed: _ignoredSeed, ...providerFields } =
    providerInput;
  const provider: Provider = {
    ...providerFields,
    id: attemptIdentity.providerId,
    createdAt: Date.now(),
  };
  let operation = await invoke<QuickStartOperation>("quick_start_apply", {
    request: {
      idempotencyKey: attemptIdentity.idempotencyKey,
      appType: appId,
      provider,
    },
  });
  operation = await waitForTerminalOperation(operation);

  await Promise.all([
    queryClient.invalidateQueries({ queryKey: ["providers", appId] }),
    queryClient.invalidateQueries({ queryKey: ["proxyStatus"] }),
    queryClient.invalidateQueries({ queryKey: ["proxyTakeover"] }),
  ]);

  return {
    operation,
    takeoverOk:
      operation.status === "succeeded" &&
      (appId === "claude-desktop" ||
        operation.takeoverEnabled ||
        operation.takeoverWasEnabled),
    providerId: provider.id,
    identity: attemptIdentity,
  };
}

const TERMINAL_STATUSES: ReadonlySet<QuickStartOperationStatus> = new Set([
  "succeeded",
  "failed",
  "rolled_back",
  "rollback_failed",
]);

async function waitForTerminalOperation(
  initial: QuickStartOperation,
): Promise<QuickStartOperation> {
  let operation = initial;
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (TERMINAL_STATUSES.has(operation.status)) return operation;
    await new Promise((resolve) => window.setTimeout(resolve, 500));
    operation = await invoke<QuickStartOperation>("quick_start_get_operation", {
      operationId: operation.id,
    });
  }
  throw new Error(
    `QuickStart operation ${operation.id} is still running; retry to resume it`,
  );
}

export async function rollbackQuickStartOperation(
  operation: QuickStartOperation,
): Promise<QuickStartOperation> {
  return invoke<QuickStartOperation>("quick_start_rollback", {
    operationId: operation.id,
    expectedRevision: operation.revision,
  });
}

export async function listRecoverableQuickStartOperations(): Promise<
  QuickStartOperation[]
> {
  return invoke<QuickStartOperation[]>("quick_start_list_recoverable");
}

export async function listRecentQuickStartOperations(
  limit = 20,
): Promise<QuickStartOperation[]> {
  return invoke<QuickStartOperation[]>("quick_start_list_recent", { limit });
}

export async function getQuickStartOperationEvents(
  operationId: string,
): Promise<QuickStartOperationEvent[]> {
  return invoke<QuickStartOperationEvent[]>("quick_start_get_events", {
    operationId,
  });
}
