import { invoke } from "@tauri-apps/api/core";
import type { McpApps, McpServer } from "@/types";

// ───────────────────── Smithery API 类型 ─────────────────────

export interface SmitheryServer {
  id: string;
  qualifiedName: string;
  namespace: string;
  slug?: string;
  displayName: string;
  description?: string;
  iconUrl?: string;
  verified: boolean;
  useCount: number;
  remote: boolean;
  isDeployed: boolean;
  createdAt?: string;
  homepage?: string;
  bySmithery: boolean;
}

export interface SmitheryPagination {
  currentPage: number;
  pageSize: number;
  totalPages: number;
  totalCount: number;
}

export interface SmitheryListResponse {
  servers: SmitheryServer[];
  pagination: SmitheryPagination;
}

export interface SmitheryConnection {
  type: string;
  deploymentUrl?: string;
  bundleUrl?: string;
  runtime?: string;
  configSchema?: Record<string, unknown>;
}

export interface SmitheryTool {
  name: string;
  description?: string;
  inputSchema?: Record<string, unknown>;
}

export interface SmitheryServerDetail {
  qualifiedName: string;
  displayName: string;
  description?: string;
  iconUrl?: string;
  remote: boolean;
  deploymentUrl?: string;
  connections: SmitheryConnection[];
  tools: SmitheryTool[];
  resources?: unknown[];
  prompts?: unknown[];
}

// ───────────────────── API 调用 ─────────────────────

export const smitheryRegistryApi = {
  /** 搜索/浏览 Smithery 服务器列表 */
  async searchServers(
    page?: number,
    pageSize?: number,
    verified?: boolean,
    remote?: boolean,
  ): Promise<SmitheryListResponse> {
    const args: Record<string, unknown> = {};
    if (page !== undefined) args.page = page;
    if (pageSize !== undefined) args.pageSize = pageSize;
    if (verified !== undefined) args.verified = verified;
    if (remote !== undefined) args.remote = remote;
    return await invoke("search_smithery_servers", args);
  },

  /** 获取 Smithery 服务器详情 */
  async getServerDetail(
    qualifiedName: string,
  ): Promise<SmitheryServerDetail> {
    return await invoke("get_smithery_server_detail", { qualifiedName });
  },

  /** 从 Smithery 安装 MCP 服务器 */
  async installServer(
    qualifiedName: string,
    enabledApps: McpApps,
  ): Promise<McpServer> {
    return await invoke("install_mcp_from_smithery", {
      qualifiedName,
      enabledApps: enabledApps,
    });
  },
};
