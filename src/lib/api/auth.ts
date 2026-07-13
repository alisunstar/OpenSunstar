import { invoke } from "@tauri-apps/api/core";

export type ManagedAuthProvider = "github_copilot" | "codex_oauth";

export interface ManagedAuthAccount {
  id: string;
  provider: ManagedAuthProvider;
  login: string;
  avatar_url: string | null;
  authenticated_at: number;
  is_default: boolean;
  github_domain: string;
}

export interface ManagedAuthStatus {
  provider: ManagedAuthProvider;
  authenticated: boolean;
  default_account_id: string | null;
  migration_error?: string | null;
  accounts: ManagedAuthAccount[];
}

export interface ManagedAuthDeviceCodeResponse {
  provider: ManagedAuthProvider;
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export type LocalCliAccessMode =
  | "third_party_key"
  | "official_cli_login"
  | "unknown";

export type LocalCliCredentialState =
  | "missing"
  | "present_unverified"
  | "logged_in_detected"
  | "unknown";

export type LocalCliRouteState =
  | "applied"
  | "not_applied"
  | "not_applicable"
  | "unknown";

export type LocalCliConfidence = "high" | "medium" | "low";

export interface LocalCliAuthStatus {
  toolKey: "claude" | "gemini";
  simpleConnectTool: "claude-code" | "gemini-cli";
  displayName: string;
  accessMode: LocalCliAccessMode;
  credentialState: LocalCliCredentialState;
  routeState: LocalCliRouteState;
  confidence: LocalCliConfidence;
  action: "none" | "apply" | "open_auth_center";
  configPath: string;
  configExists: boolean;
  selectedAuthType: string | null;
  simpleConnectConfigured: boolean;
  simpleConnectBaseUrl: string | null;
  simpleConnectModel: string | null;
  keyHint: string | null;
  detectedSources: string[];
  evidence: string[];
}

export async function authStartLogin(
  authProvider: ManagedAuthProvider,
  githubDomain?: string,
): Promise<ManagedAuthDeviceCodeResponse> {
  return invoke<ManagedAuthDeviceCodeResponse>("auth_start_login", {
    authProvider,
    githubDomain: githubDomain || null,
  });
}

export async function authPollForAccount(
  authProvider: ManagedAuthProvider,
  deviceCode: string,
  githubDomain?: string,
): Promise<ManagedAuthAccount | null> {
  return invoke<ManagedAuthAccount | null>("auth_poll_for_account", {
    authProvider,
    deviceCode,
    githubDomain: githubDomain || null,
  });
}

export async function authListAccounts(
  authProvider: ManagedAuthProvider,
): Promise<ManagedAuthAccount[]> {
  return invoke<ManagedAuthAccount[]>("auth_list_accounts", {
    authProvider,
  });
}

export async function authGetStatus(
  authProvider: ManagedAuthProvider,
): Promise<ManagedAuthStatus> {
  return invoke<ManagedAuthStatus>("auth_get_status", {
    authProvider,
  });
}

export async function authRemoveAccount(
  authProvider: ManagedAuthProvider,
  accountId: string,
): Promise<void> {
  return invoke("auth_remove_account", {
    authProvider,
    accountId,
  });
}

export async function authSetDefaultAccount(
  authProvider: ManagedAuthProvider,
  accountId: string,
): Promise<void> {
  return invoke("auth_set_default_account", {
    authProvider,
    accountId,
  });
}

export async function authLogout(
  authProvider: ManagedAuthProvider,
): Promise<void> {
  return invoke("auth_logout", {
    authProvider,
  });
}

export async function getLocalCliAuthStatus(): Promise<LocalCliAuthStatus[]> {
  return invoke<LocalCliAuthStatus[]>("get_local_cli_auth_status");
}

export const authApi = {
  authStartLogin,
  authPollForAccount,
  authListAccounts,
  authGetStatus,
  authRemoveAccount,
  authSetDefaultAccount,
  authLogout,
  getLocalCliAuthStatus,
};
