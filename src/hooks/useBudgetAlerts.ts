import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

interface BudgetAlert {
  providerId: string;
  appType: string;
  providerName: string;
  alertLevel: "warning" | "critical" | "emergency";
  period: string;
  usageUsd: number;
  limitUsd: number;
  percentage: number;
}

const ALERT_TITLES: Record<string, string> = {
  warning: "⚠️ 预算预警",
  critical: "🚨 预算超限",
  emergency: "🔴 严重超限",
};

export function useBudgetAlerts() {
  useEffect(() => {
    const unlisten = listen<BudgetAlert>("budget-alert", (event) => {
      const alert = event.payload;
      const periodLabel = alert.period === "daily" ? "日" : "月";
      const title = ALERT_TITLES[alert.alertLevel] || "预算提醒";
      const message = `${alert.providerName} ${periodLabel}用量已达 ${alert.percentage.toFixed(0)}%（$${alert.usageUsd.toFixed(4)} / $${alert.limitUsd.toFixed(2)}）`;

      switch (alert.alertLevel) {
        case "emergency":
          toast.error(title, {
            description: message,
            duration: 10000,
          });
          break;
        case "critical":
          toast.error(title, {
            description: message,
            duration: 8000,
          });
          break;
        default:
          toast.warning(title, {
            description: message,
            duration: 5000,
          });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
