import { useCallback, useEffect, useState } from "react";
import { getAIRoiReport, type AIRoiReport } from "@/api/aiInsight";

export function useAIRoiReport(rangeDays: number, refreshToken = 0) {
  const [report, setReport] = useState<AIRoiReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    const data = await getAIRoiReport(rangeDays);
    if (data) {
      setReport(data);
    } else {
      setError("无法加载 AI 投入报告");
    }
    setLoading(false);
  }, [rangeDays]);

  useEffect(() => {
    void reload();
  }, [reload, refreshToken]);

  return { report, loading, error, reload };
}
