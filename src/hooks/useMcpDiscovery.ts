import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { mcpRegistryApi } from "@/lib/api/mcpRegistry";
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
