import { motion } from "framer-motion";
import { useTranslation } from "react-i18next";
import { UsageDashboard } from "./UsageDashboard";

export function TokenStatsPage() {
  const { t } = useTranslation();

  return (
    <motion.div
      className="flex-1 flex flex-col min-h-0"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
    >
      <div className="shrink-0 px-6 pt-6 pb-2">
        <h2 className="text-lg font-semibold text-foreground">
          {t("sidebar.tokenStats", { defaultValue: "Tokens 统计" })}
        </h2>
        <p className="text-sm text-muted-foreground mt-1">
          {t("tokenStats.description", {
            defaultValue: "Token 用量统计与费用分析",
          })}
        </p>
      </div>

      <div className="flex-1 min-h-0 px-6 pb-6">
        <UsageDashboard />
      </div>
    </motion.div>
  );
}
