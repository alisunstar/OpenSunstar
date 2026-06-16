import { invoke } from "@tauri-apps/api/core";
import type { McpApps, McpServer } from "@/types";

// ───────────────────── Registry API 类型 ─────────────────────

export interface RegistryRemote {
  type: string;
  url: string;
}

export interface RegistryRepository {
  url?: string;
  source?: string;
}

export interface RegistryServer {
  name: string;
  title?: string;
  description?: string;
  version?: string;
  remotes: RegistryRemote[];
  repository?: RegistryRepository;
  websiteUrl?: string;
  tags?: string[];
}

export interface RegistryServerEntry {
  server: RegistryServer;
  _meta?: Record<string, unknown>;
}

export interface RegistryListMetadata {
  nextCursor?: string;
  count?: number;
}

export interface RegistryListResponse {
  servers: RegistryServerEntry[];
  metadata: RegistryListMetadata;
}

export interface RegistryServerDetail {
  server: RegistryServer;
  _meta?: Record<string, unknown>;
}

// ───────────────────── API 调用 ─────────────────────

export const mcpRegistryApi = {
  /** 搜索/浏览注册表服务器 */
  async searchServers(
    query?: string,
    cursor?: string,
    limit?: number,
  ): Promise<RegistryListResponse> {
    // 过滤掉 undefined，避免 Tauri invoke 序列化异常
    const args: Record<string, unknown> = {};
    if (limit !== undefined) args.limit = limit;
    if (query !== undefined && query !== "") args.query = query;
    if (cursor !== undefined && cursor !== "") args.cursor = cursor;
    return await invoke("search_mcp_registry", args);
  },

  /** 获取服务器详情 */
  async getServer(name: string): Promise<RegistryServerDetail> {
    return await invoke("get_mcp_registry_server", { name });
  },

  /** 从注册表安装 */
  async installServer(
    name: string,
    enabledApps: McpApps,
  ): Promise<McpServer> {
    return await invoke("install_mcp_from_registry", {
      name,
      enabledApps: enabledApps,
    });
  },

  /** 测试 MCP 服务器连接 */
  async testConnection(
    serverSpec: Record<string, unknown>,
  ): Promise<McpConnectionTestResult> {
    return await invoke("test_mcp_connection", { serverSpec });
  },
};

// ───────────────────── 连接测试类型 ─────────────────────

export type McpConnectionStatus =
  | "connected"
  | "auth_required"
  | "unreachable"
  | "timeout"
  | "unexpected_response"
  | "command_failed";

export interface McpServerInfo {
  name?: string;
  version?: string;
}

export interface McpConnectionTestResult {
  status: McpConnectionStatus;
  message: string;
  server_info?: McpServerInfo;
  error_detail?: string;
}
