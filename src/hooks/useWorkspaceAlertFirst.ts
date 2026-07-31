import { useCallback, useEffect, useState } from "react";

const STORAGE_KEY = "opensunstar:workspace-alert-first";

/**
 * Feature Flag：工作区告警首屏（工作区重构 2026-07-30）。
 *
 * - `true`（默认）：「今日工作台」首屏展示 `TodayAlertsPanel` 告警卡片。
 * - `false`：保持老用户习惯，首屏不突出告警，告警仅通过托盘/系统通知触达。
 *
 * 默认开启以便内部测试与灰度验证；如需回退老版首屏，可在设置中手动关闭。
 */
export function useWorkspaceAlertFirst() {
  const [enabled, setEnabled] = useState<boolean>(() => {
    if (typeof window === "undefined") {
      return true;
    }
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) {
        return true;
      }
      return raw === "true";
    } catch {
      return true;
    }
  });

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    try {
      window.localStorage.setItem(STORAGE_KEY, String(enabled));
    } catch {
      // localStorage 不可用时静默放弃
    }
  }, [enabled]);

  const toggle = useCallback(() => {
    setEnabled((prev) => !prev);
  }, []);

  return { enabled, setEnabled, toggle };
}
