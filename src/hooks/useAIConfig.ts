import { useState, useCallback } from "react";
import { buildProviderConfig, type AIProviderConfig } from "@/api/aiInsight";

/**
 * 检测当前是否已配置 AI 提供方。
 * 用于看板页面决定是否启用 AI 功能。
 */
export function useAIConfig() {
  const [configured, setConfigured] = useState<boolean>(
    () => buildProviderConfig() !== null,
  );

  const refresh = useCallback(() => {
    setConfigured(buildProviderConfig() !== null);
  }, []);

  return {
    aiConfigured: configured,
    refreshConfig: refresh,
    getConfig: buildProviderConfig as () => AIProviderConfig | null,
  };
}
