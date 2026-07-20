import { invoke } from "@tauri-apps/api/core";
import type {
  UsageSummary,
  UsageSummaryByApp,
  DailyStats,
  ProviderStats,
  ModelStats,
  RequestLog,
  LogFilters,
  ModelPricing,
  ProviderLimitStatus,
  PaginatedLogs,
  SessionSyncResult,
  DataSourceSummary,
} from "@/types/usage";
import type { UsageResult } from "@/types";
import type { AppId } from "./types";
import type { TemplateType } from "@/config/constants";

export const usageApi = {
  // Provider usage script methods
  query: async (providerId: string, appId: AppId): Promise<UsageResult> => {
    return invoke("queryProviderUsage", { providerId, app: appId });
  },

  testScript: async (
    providerId: string,
    appId: AppId,
    scriptCode: string,
    timeout?: number,
    apiKey?: string,
    baseUrl?: string,
    accessToken?: string,
    userId?: string,
    templateType?: TemplateType,
  ): Promise<UsageResult> => {
    return invoke("testUsageScript", {
      providerId,
      app: appId,
      scriptCode,
      timeout,
      apiKey,
      baseUrl,
      accessToken,
      userId,
      templateType,
    });
  },

  // P0-2 域名确认闸门：用户在弹窗确认信任某目标域名后调用，后端持久化
  // (app_type, provider_id) -> host，此后该 host 的外发不再触发确认。
  confirmUsageScriptHost: async (
    providerId: string,
    appId: AppId,
    host: string,
  ): Promise<void> => {
    return invoke("confirm_usage_script_host", {
      providerId,
      app: appId,
      host,
    });
  },

  // Proxy usage statistics methods
  getUsageSummary: async (
    startDate?: number,
    endDate?: number,
    appType?: string,
  ): Promise<UsageSummary> => {
    return invoke("get_usage_summary", { startDate, endDate, appType });
  },

  getUsageSummaryByApp: async (
    startDate?: number,
    endDate?: number,
  ): Promise<UsageSummaryByApp[]> => {
    return invoke("get_usage_summary_by_app", { startDate, endDate });
  },

  getUsageTrends: async (
    startDate?: number,
    endDate?: number,
    appType?: string,
  ): Promise<DailyStats[]> => {
    return invoke("get_usage_trends", { startDate, endDate, appType });
  },

  getProviderStats: async (
    startDate?: number,
    endDate?: number,
    appType?: string,
  ): Promise<ProviderStats[]> => {
    return invoke("get_provider_stats", { startDate, endDate, appType });
  },

  getModelStats: async (
    startDate?: number,
    endDate?: number,
    appType?: string,
  ): Promise<ModelStats[]> => {
    return invoke("get_model_stats", { startDate, endDate, appType });
  },

  getRequestLogs: async (
    filters: LogFilters,
    page: number = 0,
    pageSize: number = 20,
  ): Promise<PaginatedLogs> => {
    return invoke("get_request_logs", {
      filters,
      page,
      pageSize,
    });
  },

  getRequestDetail: async (requestId: string): Promise<RequestLog | null> => {
    return invoke("get_request_detail", { requestId });
  },

  getModelPricing: async (): Promise<ModelPricing[]> => {
    return invoke("get_model_pricing");
  },

  updateModelPricing: async (
    modelId: string,
    displayName: string,
    inputCost: string,
    outputCost: string,
    cacheReadCost: string,
    cacheCreationCost: string,
  ): Promise<void> => {
    return invoke("update_model_pricing", {
      modelId,
      displayName,
      inputCost,
      outputCost,
      cacheReadCost,
      cacheCreationCost,
    });
  },

  deleteModelPricing: async (modelId: string): Promise<void> => {
    return invoke("delete_model_pricing", { modelId });
  },

  checkProviderLimits: async (
    providerId: string,
    appType: string,
  ): Promise<ProviderLimitStatus> => {
    return invoke("check_provider_limits", { providerId, appType });
  },

  // Session usage sync
  syncSessionUsage: async (): Promise<SessionSyncResult> => {
    return invoke("sync_session_usage");
  },

  getDataSourceBreakdown: async (): Promise<DataSourceSummary[]> => {
    return invoke("get_usage_data_sources");
  },
};
