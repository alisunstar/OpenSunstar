import { useState, useCallback, useEffect, useRef } from "react";
import { buildProviderConfig, type AIProviderConfig } from "@/api/aiInsight";

/**
 * 检测当前是否已配置 AI 提供方。
 * 用于看板页面决定是否启用 AI 功能。
 */
export function useAIConfig() {
  const [configured, setConfigured] = useState(false);
  const configRef = useRef<AIProviderConfig | null>(null);

  const refresh = useCallback(async () => {
    const config = await buildProviderConfig();
    configRef.current = config;
    setConfigured(config !== null);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const getConfig = useCallback((): AIProviderConfig | null => {
    return configRef.current;
  }, []);

  return {
    aiConfigured: configured,
    refreshConfig: refresh,
    getConfig,
  };
}
