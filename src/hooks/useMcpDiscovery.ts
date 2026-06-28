import { useQuery, useMutation, useQueryClient, keepPreviousData } from "@tanstack/react-query";
import { mcpRegistryApi } from "@/lib/api/mcpRegistry";
import { smitheryRegistryApi } from "@/lib/api/smitheryRegistry";
import type { McpApps } from "@/types";
import { toast } from "sonner";

/**
 * 搜索/浏览 MCP 注册表服务器
 */
export function useRegistryServers(
  query?: string,
  cursor?: string,
  limit?: number,
) {
  return useQuery({
    queryKey: ["mcpRegistry", "servers", query, cursor, limit],
    queryFn: () => mcpRegistryApi.searchServers(query, cursor, limit),
    staleTime: 60_000, // 1 分钟内不重新请求
  });
}

/**
 * 获取注册表服务器详情
 */
export function useRegistryServerDetail(name: string | null) {
  return useQuery({
    queryKey: ["mcpRegistry", "detail", name],
    queryFn: () => mcpRegistryApi.getServer(name!),
    enabled: !!name,
    staleTime: 120_000,
  });
}

/**
 * 从注册表安装服务器
 */
export function useInstallFromRegistry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      name,
      enabledApps,
    }: {
      name: string;
      enabledApps: McpApps;
    }) => mcpRegistryApi.installServer(name, enabledApps),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["mcp", "all"] });
      toast.success(`MCP 服务器 "${data.name}" 安装成功`, {
        closeButton: true,
      });
    },
    onError: (error: any) => {
      toast.error("安装失败", {
        description: String(error),
        closeButton: true,
      });
    },
  });
}

/**
 * 测试 MCP 服务器连接
 */
export function useMcpTestConnection() {
  return useMutation({
    mutationFn: (serverSpec: Record<string, unknown>) =>
      mcpRegistryApi.testConnection(serverSpec),
  });
}

// ───────────────────── Smithery Registry Hooks ─────────────────────

/**
 * 搜索/浏览 Smithery Registry 服务器列表
 * 支持分页、认证筛选、远程/本地筛选
 */
export function useSmitheryServers(
  page?: number,
  pageSize?: number,
  verified?: boolean,
  remote?: boolean,
) {
  return useQuery({
    queryKey: ["smithery", "servers", page, pageSize, verified, remote],
    queryFn: () =>
      smitheryRegistryApi.searchServers(page, pageSize, verified, remote),
    staleTime: 5 * 60_000, // 5 分钟缓存
    placeholderData: keepPreviousData,
  });
}

/**
 * 获取 Smithery 服务器详情
 */
export function useSmitheryServerDetail(qualifiedName: string | null) {
  return useQuery({
    queryKey: ["smithery", "detail", qualifiedName],
    queryFn: () => smitheryRegistryApi.getServerDetail(qualifiedName!),
    enabled: !!qualifiedName,
    staleTime: 120_000,
  });
}

/**
 * 从 Smithery Registry 安装服务器
 */
export function useInstallFromSmithery() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      qualifiedName,
      enabledApps,
    }: {
      qualifiedName: string;
      enabledApps: McpApps;
    }) => smitheryRegistryApi.installServer(qualifiedName, enabledApps),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["mcp", "all"] });
      toast.success(`MCP 服务器 "${data.name}" 安装成功`, {
        closeButton: true,
      });
    },
    onError: (error: any) => {
      toast.error("安装失败", {
        description: String(error),
        closeButton: true,
      });
    },
  });
}
