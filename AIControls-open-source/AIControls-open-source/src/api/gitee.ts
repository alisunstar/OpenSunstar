import { invoke } from "@tauri-apps/api/core";

function isZhUi(): boolean {
  if (typeof navigator === "undefined") return false;
  return navigator.language.toLowerCase().startsWith("zh");
}

export type GiteeSyncStatus = {
  connected: boolean;
  nextAutoCheckMs: number;
  lastBackupMs: number | null;
  lastMessage: string | null;
  lastOk: boolean | null;
};

export async function getGiteeSyncStatus(): Promise<GiteeSyncStatus | null> {
  try {
    return await invoke<GiteeSyncStatus>("get_gitee_sync_status");
  } catch {
    return null;
  }
}

export type GiteeSettings = {
  appConfigured: boolean;
  connected: boolean;
  ownerLogin: string | null;
  repoName: string | null;
  clientIdSaved: string | null;
  savedRepoName: string | null;
  oauthCallbackUrl: string;
};

export async function getGiteeSettings(): Promise<GiteeSettings | null> {
  try {
    return await invoke<GiteeSettings>("get_gitee_settings");
  } catch {
    return null;
  }
}

export async function saveGiteeApp(
  clientId: string,
  clientSecret: string,
  repoName: string,
): Promise<{ ok: boolean; message: string }> {
  try {
    await invoke("save_gitee_app", { clientId, clientSecret, repoName });
    return { ok: true, message: isZhUi() ? "已保存。" : "Saved." };
  } catch (e) {
    return {
      ok: false,
      message: e instanceof Error ? e.message : String(e),
    };
  }
}

export async function giteeOauthLogin(): Promise<{ ok: boolean; message: string }> {
  try {
    const msg = await invoke<string>("gitee_oauth_login");
    return { ok: true, message: msg };
  } catch (e) {
    return {
      ok: false,
      message: e instanceof Error ? e.message : String(e),
    };
  }
}

/** 手动备份：始终上传（与定时任务的「有变更才传」不同）。 */
export async function giteeBackupNow(): Promise<{ ok: boolean; message: string }> {
  try {
    const msg = await invoke<string>("gitee_backup_now", { force: true });
    return { ok: true, message: msg };
  } catch (e) {
    return {
      ok: false,
      message: e instanceof Error ? e.message : String(e),
    };
  }
}

export async function giteeDisconnect(): Promise<{ ok: boolean; message: string }> {
  try {
    await invoke("gitee_disconnect");
    return { ok: true, message: isZhUi() ? "已解除 Gitee 授权。" : "Gitee authorization removed." };
  } catch (e) {
    return {
      ok: false,
      message: e instanceof Error ? e.message : String(e),
    };
  }
}

export async function giteeRestoreFromRepoUrl(
  repoUrl: string,
): Promise<{ ok: boolean; message: string }> {
  try {
    const msg = await invoke<string>("gitee_restore_from_repo_url", { repoUrl });
    return { ok: true, message: msg };
  } catch (e) {
    return {
      ok: false,
      message: e instanceof Error ? e.message : String(e),
    };
  }
}
