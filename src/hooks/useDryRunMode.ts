import { useCallback, useEffect, useState } from "react";
import { dryRunApi } from "@/lib/api/prompts";

export function useDryRunMode() {
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const mode = await dryRunApi.getMode();
      setEnabled(mode);
    } catch {
      setEnabled(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const setMode = useCallback(async (next: boolean) => {
    await dryRunApi.setMode(next);
    setEnabled(next);
  }, []);

  return { enabled, loading, setMode, reload };
}
