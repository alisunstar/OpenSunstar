import {
  createContext,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from "react";

export interface LastAICall {
  cost: number;
  tokens: number;
  insightType: string;
  isCached: boolean;
  at: number;
}

interface AICostContextValue {
  lastCall: LastAICall | null;
  recordCall: (call: Omit<LastAICall, "at">) => void;
  refreshToken: number;
  bumpRefresh: () => void;
}

const AICostContext = createContext<AICostContextValue | null>(null);

export function AICostProvider({ children }: { children: ReactNode }) {
  const [lastCall, setLastCall] = useState<LastAICall | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  const recordCall = useCallback((call: Omit<LastAICall, "at">) => {
    setLastCall({ ...call, at: Date.now() });
    setRefreshToken((t) => t + 1);
  }, []);

  const bumpRefresh = useCallback(() => {
    setRefreshToken((t) => t + 1);
  }, []);

  return (
    <AICostContext.Provider
      value={{ lastCall, recordCall, refreshToken, bumpRefresh }}
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
