import type { QueryClient } from "@tanstack/react-query";
import type { AppId } from "@/lib/api";
import { proxyApi, simpleConnectApi } from "@/lib/api";
import type { Provider } from "@/types";
import type { QuickStartAppId } from "@/config/quickStartCurated";

export interface QuickStartApplyDeps {
  appId: QuickStartAppId;
  addProvider: (
    input: Omit<Provider, "id"> & {
      addToLive?: boolean;
      ensureClaudeDesktopOfficialSeed?: boolean;
    },
  ) => Promise<Provider>;
  switchProvider: (id: string) => Promise<unknown>;
  queryClient: QueryClient;
}

export interface QuickStartApplyResult {
  takeoverOk: boolean;
  providerId: string;
}

export async function runQuickStartApplyPipeline(
  deps: QuickStartApplyDeps,
  providerInput: Omit<Provider, "id"> & {
    ensureClaudeDesktopOfficialSeed?: boolean;
  },
): Promise<QuickStartApplyResult> {
  const { appId, addProvider, switchProvider, queryClient } = deps;

  const created = await addProvider({
    ...providerInput,
    addToLive: true,
  });

  if (appId === "claude") {
    try {
      await simpleConnectApi.clear("claude-code");
    } catch (e) {
      console.warn("[QuickStart] simple_connect_clear failed:", e);
    }
  }

  await switchProvider(created.id);

  let takeoverOk = false;
  try {
    if (appId === "claude" || appId === "codex" || appId === "gemini") {
      await proxyApi.setProxyTakeoverForApp(appId as AppId, true);
    }
    await proxyApi.startProxyServer();
    await queryClient.invalidateQueries({ queryKey: ["proxyStatus"] });
    await queryClient.invalidateQueries({ queryKey: ["proxyTakeover"] });
    takeoverOk = true;
  } catch (e) {
    console.error("[QuickStart] proxy pipeline failed:", e);
  }

  await queryClient.invalidateQueries({ queryKey: ["providers", appId] });

  return { takeoverOk, providerId: created.id };
}
