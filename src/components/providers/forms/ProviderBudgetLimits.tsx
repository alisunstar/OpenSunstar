import { useTranslation } from "react-i18next";
import { DollarSign } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export interface ProviderBudgetLimitsConfig {
  limitDailyUsd?: string;
  limitMonthlyUsd?: string;
}

interface ProviderBudgetLimitsProps {
  config: ProviderBudgetLimitsConfig;
  onChange: (config: ProviderBudgetLimitsConfig) => void;
}

export function ProviderBudgetLimits({
  config,
  onChange,
}: ProviderBudgetLimitsProps) {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border border-border/50 bg-muted/20 p-4 space-y-4">
      <div className="flex items-center gap-3">
        <DollarSign className="h-4 w-4 text-muted-foreground" />
        <div>
          <p className="font-medium">
            {t("providerBudget.title", { defaultValue: "用量预算" })}
          </p>
          <p className="text-sm text-muted-foreground">
            {t("providerBudget.description", {
              defaultValue:
                "设置日/月美元预算上限，超出后将触发预警通知（留空表示不限制）",
            })}
          </p>
        </div>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="limit-daily-usd">
            {t("providerBudget.dailyLimit", { defaultValue: "日预算 (USD)" })}
          </Label>
          <Input
            id="limit-daily-usd"
            type="number"
            step="0.01"
            min="0"
            inputMode="decimal"
            value={config.limitDailyUsd || ""}
            onChange={(e) =>
              onChange({
                ...config,
                limitDailyUsd: e.target.value || undefined,
              })
            }
            placeholder={t("providerBudget.dailyLimitPlaceholder", {
              defaultValue: "例如 0.01",
            })}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="limit-monthly-usd">
            {t("providerBudget.monthlyLimit", { defaultValue: "月预算 (USD)" })}
          </Label>
          <Input
            id="limit-monthly-usd"
            type="number"
            step="0.01"
            min="0"
            inputMode="decimal"
            value={config.limitMonthlyUsd || ""}
            onChange={(e) =>
              onChange({
                ...config,
                limitMonthlyUsd: e.target.value || undefined,
              })
            }
            placeholder={t("providerBudget.monthlyLimitPlaceholder", {
              defaultValue: "例如 10.00",
            })}
          />
        </div>
      </div>
    </div>
  );
}
