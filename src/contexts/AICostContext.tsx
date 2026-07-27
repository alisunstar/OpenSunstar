import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";

import { getAICostSummary, type AICostSummary } from "@/api/aiInsight";

export interface LastAICall {
  cost: number;
  tokens: number;
  insightType: string;
  isCached: boolean;
  /**
   * `cost` 的单价是否可信（后端 `pricing_known`）。
   *
   * 缺省按可信处理 —— 未接这个字段的调用方不该被一律标成「单价未知」。
   */
  pricingKnown?: boolean;
  at: number;
}

/**
 * 成本汇总的三态。
 *
 * 不能塌缩成「summary 是否为 null」：`getAICostSummary` 在 API 层统一
 * `catch → return null`，把「没查到」渲染成「¥0.00」是最坏的一种谎 ——
 * 用户据此认为 AI 没花钱，而真实账单可能不是 0（审查报告 §5.6）。
 */
export type AICostSummaryState = "loading" | "loaded" | "failed";

/** 成本汇总的统计窗口，全站唯一来源。 */
export const AI_COST_SUMMARY_RANGE_DAYS = 30;

interface AICostContextValue {
  lastCall: LastAICall | null;
  recordCall: (call: Omit<LastAICall, "at">) => void;
  refreshToken: number;
  bumpRefresh: () => void;
  /** 近 `summaryRangeDays` 天的成本汇总。全站单次拉取，由 Provider 持有。 */
  summary: AICostSummary | null;
  summaryState: AICostSummaryState;
  summaryRangeDays: number;
}

const AICostContext = createContext<AICostContextValue | null>(null);

interface AICostProviderProps {
  children: ReactNode;
  /**
   * 未配置 AI 时不发请求。
   *
   * 那一趟往返没有意义，而且会把「未配置」和「查询失败」混成同一个空状态。
   */
  aiConfigured?: boolean;
  rangeDays?: number;
}

/**
 * AI 成本的单一数据源。
 *
 * 这里持有 `summary` 而不只是 `refreshToken`，是因为原来 `AICostStrip` 和
 * `AINLQueryBar` 各写了一份 `getAICostSummary(30)` 的 `useEffect`，而这两个
 * 组件在「今日工作台」和「项目看板」下都挂着 —— 同一份数据每个 Tab 拉两次
 * （审查报告 §3.1）。Context 早就在了，两处也都消费了它的 `refreshToken` 来决定
 * **什么时候重拉**，却谁都没把 **数据本身** 提上来：同步了时钟，没同步内容。
 */
export function AICostProvider({
  children,
  aiConfigured = false,
  rangeDays = AI_COST_SUMMARY_RANGE_DAYS,
}: AICostProviderProps) {
  const [lastCall, setLastCall] = useState<LastAICall | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);
  const [summary, setSummary] = useState<AICostSummary | null>(null);
  const [summaryState, setSummaryState] =
    useState<AICostSummaryState>("loading");

  const recordCall = useCallback((call: Omit<LastAICall, "at">) => {
    setLastCall({ ...call, at: Date.now() });
    setRefreshToken((t) => t + 1);
  }, []);

  const bumpRefresh = useCallback(() => {
    setRefreshToken((t) => t + 1);
  }, []);

  useEffect(() => {
    if (!aiConfigured) {
      setSummary(null);
      setSummaryState("loading");
      return;
    }
    let cancelled = false;
    setSummaryState("loading");
    void getAICostSummary(rangeDays).then((s) => {
      if (cancelled) return;
      setSummary(s ?? null);
      setSummaryState(s ? "loaded" : "failed");
    });
    return () => {
      cancelled = true;
    };
  }, [aiConfigured, rangeDays, refreshToken]);

  return (
    <AICostContext.Provider
      value={{
        lastCall,
        recordCall,
        refreshToken,
        bumpRefresh,
        summary,
        summaryState,
        summaryRangeDays: rangeDays,
      }}
    >
      {children}
    </AICostContext.Provider>
  );
}

export function useAICost(): AICostContextValue {
  const ctx = useContext(AICostContext);
  if (!ctx) {
    throw new Error("useAICost must be used within AICostProvider");
  }
  return ctx;
}

/** 在 Provider 外安全 no-op（测试/Story 用） */
export function useAICostOptional(): AICostContextValue | null {
  return useContext(AICostContext);
}
