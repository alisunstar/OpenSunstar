import { invoke } from "@tauri-apps/api/core";

export interface SupplierProfile {
  id: string;
  name: string;
  openai_base: string;
  anthropic_base?: string | null;
  default_model: string;
  website?: string | null;
}

export interface PoolKeyMeta {
  id: string;
  label: string;
  weight: number;
  enabled: boolean;
}

export interface SimpleConnectState {
  supplier_id: string;
  custom_openai_base?: string | null;
  pool_enabled: boolean;
  fail_threshold: number;
  preferred_key_id?: string | null;
  pool_keys: PoolKeyMeta[];
  last_model?: string | null;
  last_tool?: string | null;
  last_applied_supplier_id?: string | null;
  deeplink_import_enabled?: boolean;
  require_key_verify?: boolean;
}

export interface ApplyResult {
  tool: string;
  files: string[];
  backup_path?: string | null;
  used_pool: boolean;
  proxy_port?: number | null;
}

export interface ToolConfigStatus {
  tool: string;
  configured: boolean;
  base_url?: string | null;
  model?: string | null;
  key_hint?: string | null;
  supported: boolean;
}

export interface PoolKeyStat {
  id: string;
  label: string;
  enabled: boolean;
  weight: number;
  success: number;
  failure: number;
  cooldown_stage: number;
  cooling_remaining_secs?: number | null;
  last_status?: number | null;
  available: boolean;
}

export interface SimpleConnectRuntimeStats {
  running: boolean;
  local_base?: string | null;
  upstream?: string | null;
  supplier_id?: string | null;
  port: number;
  pool_keys: PoolKeyStat[];
}

export interface SimpleConnectImportPayload {
  keys: string[];
  label?: string | null;
  model?: string | null;
  poolEnabled?: boolean | null;
  supplierId?: string | null;
  sourceUrl: string;
}

export interface SimpleConnectImportResult {
  keysAdded: number;
  duplicates: number;
  primaryKeyHint?: string | null;
  model?: string | null;
  poolEnabled?: boolean | null;
  supplierId: string;
}

export interface VerifyKeyResult {
  ok: boolean;
  model_count: number;
  error?: string | null;
}

export interface ToolUsageBreakdown {
  tool: string;
  records: number;
  input_tokens: number;
  output_tokens: number;
}

export interface UsageRecord {
  ts: number;
  tool: string;
  session: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
}

export interface SimpleConnectUsageSummary {
  files_scanned: number;
  record_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  by_tool: ToolUsageBreakdown[];
  recent_records: UsageRecord[];
  proxy_session_input: number;
  proxy_session_output: number;
  proxy_total_input: number;
  proxy_total_output: number;
  proxy_port: number;
  note: string;
}

export interface BackupAuditItem {
  path: string;
  suspicious: boolean;
  detail: string;
}

export interface BackupAuditReport {
  files_scanned: number;
  suspicious_count: number;
  items: BackupAuditItem[];
  all_clean: boolean;
}

export const simpleConnectApi = {
  listSuppliers(): Promise<SupplierProfile[]> {
    return invoke("simple_connect_list_suppliers");
  },

  listTools(): Promise<string[]> {
    return invoke("simple_connect_list_tools");
  },

  getState(): Promise<SimpleConnectState> {
    return invoke("simple_connect_get_state");
  },

  setSupplier(
    supplierId: string,
    customOpenaiBase?: string,
  ): Promise<SimpleConnectState> {
    return invoke("simple_connect_set_supplier", {
      supplierId,
      customOpenaiBase: customOpenaiBase || null,
    });
  },

  saveState(state: SimpleConnectState): Promise<void> {
    return invoke("simple_connect_save_state", { state });
  },

  storeKey(supplierId: string, apiKey: string): Promise<string> {
    return invoke("simple_connect_store_key", { supplierId, apiKey });
  },

  storePoolKey(
    supplierId: string,
    keyId: string,
    apiKey: string,
  ): Promise<string> {
    return invoke("simple_connect_store_pool_key", {
      supplierId,
      keyId,
      apiKey,
    });
  },

  removePoolKey(supplierId: string, keyId: string): Promise<void> {
    return invoke("simple_connect_remove_pool_key", { supplierId, keyId });
  },

  keyConfigured(supplierId: string): Promise<boolean> {
    return invoke("simple_connect_key_configured", { supplierId });
  },

  fetchModels(
    supplierId: string,
    customBase?: string,
  ): Promise<string[]> {
    return invoke("simple_connect_fetch_models", {
      supplierId,
      customBase: customBase || null,
    });
  },

  apply(params: {
    tool: string;
    supplierId: string;
    model: string;
    customBase?: string;
    usePool?: boolean;
  }): Promise<ApplyResult> {
    return invoke("simple_connect_apply", {
      tool: params.tool,
      supplierId: params.supplierId,
      model: params.model,
      customBase: params.customBase || null,
      usePool: params.usePool ?? false,
    });
  },

  clear(tool: string): Promise<void> {
    return invoke("simple_connect_clear", { tool });
  },

  listToolStatus(): Promise<ToolConfigStatus[]> {
    return invoke("simple_connect_list_tool_status");
  },

  poolStats(): Promise<SimpleConnectRuntimeStats> {
    return invoke("simple_connect_pool_stats");
  },

  verifyKey(
    supplierId: string,
    apiKey: string,
    customBase?: string,
  ): Promise<VerifyKeyResult> {
    return invoke("simple_connect_verify_key", {
      supplierId,
      apiKey,
      customBase: customBase || null,
    });
  },

  importKeys(
    payload: SimpleConnectImportPayload,
    skipVerify?: boolean,
  ): Promise<SimpleConnectImportResult> {
    return invoke("simple_connect_import_keys", {
      payload,
      skipVerify: skipVerify ?? false,
    });
  },

  usageSummary(): Promise<SimpleConnectUsageSummary> {
    return invoke("simple_connect_usage_summary");
  },

  backupAudit(): Promise<BackupAuditReport> {
    return invoke("simple_connect_backup_audit");
  },
};
