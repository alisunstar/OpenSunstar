import { useState, useEffect, useRef, useCallback } from "react";
import {
  buildProviderConfig,
  getAIInsight,
  getAIHealthScore,
  getAIRiskAnalysis,
  getAgentReadinessScore,
  queryProjectsNL,
  type AIInsightResult,
  type AIHealthResult,
  type AIRiskResult,
  type AgentReadinessResult,
  type ProjectContextInput,
} from "@/api/aiInsight";
import { useAICostOptional } from "@/contexts/AICostContext";

// ── 摘要 Hook ──────────────────────────────────

interface UseAIInsightOptions {
  projectId: string;
  insightType: string;
  context: ProjectContextInput | null;
  enabled?: boolean;
}

interface UseAIInsightReturn {
  data: AIInsightResult | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * 获取单条 AI 洞察（摘要 / 阶段建议等）。
 * context 为 null 时不会发起请求。
 */
export function useAIInsight({
  projectId,
  insightType,
  context,
  enabled = true,
}: UseAIInsightOptions): UseAIInsightReturn {
  const [data, setData] = useState<AIInsightResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef(false);

  const fetch = useCallback(
    async (forceRefresh: boolean) => {
      if (!enabled || !context) return;
      const config = buildProviderConfig();
      if (!config) return;

      abortRef.current = false;
      setIsLoading(true);
      setError(null);

      const result = await getAIInsight(
        projectId,
        insightType,
        config,
        context,
        forceRefresh,
      );

      if (!abortRef.current) {
        setIsLoading(false);
        if (result) {
          setData(result);
        } else {
          setError("获取 AI 洞察失败");
        }
      }
    },
    [projectId, insightType, context, enabled],
  );

  useEffect(() => {
    fetch(false);
    return () => {
      abortRef.current = true;
    };
  }, [fetch]);

  const refresh = useCallback(() => fetch(true), [fetch]);

  return { data, isLoading, error, refresh };
}

// ── 健康评分 Hook ──────────────────────────────

interface UseAIHealthOptions {
  projectId: string;
  context: ProjectContextInput | null;
  enabled?: boolean;
}

interface UseAIHealthReturn {
  data: AIHealthResult | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * 获取项目健康评分（规则评分始终返回，AI 分析可选）。
 */
export function useAIHealth({
  projectId,
  context,
  enabled = true,
}: UseAIHealthOptions): UseAIHealthReturn {
  const [data, setData] = useState<AIHealthResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef(false);

  const fetch = useCallback(
    async (forceRefresh: boolean) => {
      if (!enabled || !context) return;
      const config = buildProviderConfig();
      if (!config) {
        // 即使没有 AI 配置，也可以返回规则评分（不需要 AI 调用）
        // 但当前后端实现需要 config，所以跳过
        return;
      }

      abortRef.current = false;
      setIsLoading(true);
      setError(null);

      const result = await getAIHealthScore(
        projectId,
        config,
        context,
        forceRefresh,
      );

      if (!abortRef.current) {
        setIsLoading(false);
        if (result) {
          setData(result);
        } else {
          setError("获取健康评分失败");
        }
      }
    },
    [projectId, context, enabled],
  );

  useEffect(() => {
    fetch(false);
    return () => {
      abortRef.current = true;
    };
  }, [fetch]);

  const refresh = useCallback(() => fetch(true), [fetch]);

  return { data, isLoading, error, refresh };
}

// ── Phase 2: 风险分析 Hook ──────────────────────

interface UseAIRiskOptions {
  projectId: string;
  context: ProjectContextInput | null;
  enabled?: boolean;
}

interface UseAIRiskReturn {
  data: AIRiskResult | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * 获取项目风险分析（AI + 规则降级）。
 * 不会自动加载，需调用 refresh() 手动触发首次分析。
 */
export function useAIRisk({
  projectId,
  context,
  enabled = true,
}: UseAIRiskOptions): UseAIRiskReturn {
  const [data, setData] = useState<AIRiskResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef(false);

  const fetch = useCallback(
    async (forceRefresh: boolean) => {
      if (!enabled || !context) return;
      const config = buildProviderConfig();
      if (!config) return;

      abortRef.current = false;
      setIsLoading(true);
      setError(null);

      const result = await getAIRiskAnalysis(
        projectId,
        config,
        context,
        forceRefresh,
      );

      if (!abortRef.current) {
        setIsLoading(false);
        if (result) {
          setData(result);
        } else {
          setError("获取风险分析失败");
        }
      }
    },
    [projectId, context, enabled],
  );

  const refresh = useCallback(() => fetch(true), [fetch]);

  return { data, isLoading, error, refresh };
}

// ── Phase 2: 自然语言查询 Hook ────────────────────

interface UseNLQueryReturn {
  answer: string | null;
  isLoading: boolean;
  error: string | null;
  costEstimate: number;
  queryLogId: number | null;
  ask: (query: string, contexts: ProjectContextInput[]) => void;
}

/**
 * 自然语言查询项目数据。
 * 通过 ask() 方法发起查询，结果通过 answer 返回。
 */
export function useNLQuery(): UseNLQueryReturn {
  const [answer, setAnswer] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [costEstimate, setCostEstimate] = useState(0);
  const [queryLogId, setQueryLogId] = useState<number | null>(null);
  const abortRef = useRef(false);
  const costCtx = useAICostOptional();

  const ask = useCallback(
    (query: string, contexts: ProjectContextInput[]) => {
      const config = buildProviderConfig();
      if (!config || !query.trim()) return;

      abortRef.current = false;
      setIsLoading(true);
      setError(null);
      setAnswer(null);
      setCostEstimate(0);
      setQueryLogId(null);

      queryProjectsNL(config, contexts, query).then((result) => {
        if (!abortRef.current) {
          setIsLoading(false);
          if (result) {
            setAnswer(result.answer);
            setCostEstimate(result.cost_estimate);
            setQueryLogId(result.query_log_id ?? null);
            costCtx?.recordCall({
              cost: result.cost_estimate,
              tokens: result.tokens_used,
              insightType: "nl_query",
              isCached: false,
            });
          } else {
            setError("查询失败，请稍后重试");
          }
        }
      });
    },
    [costCtx],
  );

  useEffect(() => {
    return () => {
      abortRef.current = true;
    };
  }, []);

  return { answer, isLoading, error, costEstimate, queryLogId, ask };
}

// ── F-P2-1: Agent 配置就绪度 Hook ──────────────────

interface UseAgentReadinessReturn {
  data: AgentReadinessResult | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
  /** 触发生效态扫描（库内容 vs CLI 磁盘文件） */
  scanEffective: () => void;
}

/**
 * 获取项目的 Agent 配置就绪度评分（满分 100，9 项检查）。
 * 只需 projectPath，无需 ProjectContextInput。
 * providerConfig 可选 — 有则生成 LLM 补充建议。
 * scanEffectiveOnLoad：打开项目时自动做一次生效态扫描（节流由调用方控制）。
 */
export function useAgentReadiness(
  projectPath: string | null,
  enabled = true,
  targetApp?: string | null,
  scanEffectiveOnLoad = true,
): UseAgentReadinessReturn {
  const [data, setData] = useState<AgentReadinessResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef(false);
  const forceRefreshRef = useRef(false);
  const scanEffectiveRef = useRef(scanEffectiveOnLoad);
  const initialScanDoneRef = useRef(false);

  const fetch = useCallback(
    async (options?: { scanEffective?: boolean; forceRefresh?: boolean }) => {
      if (!enabled || !projectPath) return;
      const config = buildProviderConfig();

      const scanEffective =
        options?.scanEffective ?? scanEffectiveRef.current;
      const forceRefresh =
        options?.forceRefresh ?? forceRefreshRef.current;
      forceRefreshRef.current = false;

      abortRef.current = false;
      setIsLoading(true);
      setError(null);

      const result = await getAgentReadinessScore(
        projectPath,
        config,
        forceRefresh,
        targetApp,
        scanEffective,
      );

      if (!abortRef.current) {
        setIsLoading(false);
        if (result) {
          setData(result);
        } else {
          setError("获取 Agent 就绪度失败");
        }
      }
    },
    [projectPath, enabled, targetApp],
  );

  useEffect(() => {
    initialScanDoneRef.current = false;
    scanEffectiveRef.current = scanEffectiveOnLoad;
  }, [projectPath, scanEffectiveOnLoad]);

  useEffect(() => {
    const runInitial = !initialScanDoneRef.current;
    if (runInitial) {
      initialScanDoneRef.current = true;
    }
    void fetch({ scanEffective: scanEffectiveOnLoad && runInitial });
    return () => {
      abortRef.current = true;
    };
  }, [fetch, scanEffectiveOnLoad]);

  const refresh = useCallback(() => {
    forceRefreshRef.current = true;
    void fetch({ forceRefresh: true, scanEffective: false });
  }, [fetch]);

  const scanEffective = useCallback(() => {
    forceRefreshRef.current = true;
    void fetch({ forceRefresh: true, scanEffective: true });
  }, [fetch]);

  return { data, isLoading, error, refresh, scanEffective };
}
