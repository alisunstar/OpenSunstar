import { toast } from "sonner";
import type { TFunction } from "i18next";

import type {
  RepairAssetDriftResult,
  RepairProjectDriftResult,
} from "@/api/aiInsight";

export function showRepairAssetFeedback(
  result: RepairAssetDriftResult | null,
  t: TFunction,
): boolean {
  if (!result) {
    toast.error(
      t("kanban.readiness.repairError", {
        defaultValue: "修复请求失败，请稍后重试",
      }),
    );
    return false;
  }

  if (result.repaired) {
    toast.success(
      t("kanban.readiness.repairSuccess", {
        defaultValue: "已写回并验证生效",
      }),
    );
    return true;
  }

  if (result.before_state === "drifted" && result.after_state === "drifted") {
    toast.warning(
      t("kanban.readiness.repairFailed", {
        defaultValue: "修复后仍存在漂移",
      }),
      {
        description: result.effective_detail ?? undefined,
      },
    );
    return false;
  }

  toast.info(
    t("kanban.readiness.repairNoop", {
      defaultValue: "当前项无需修复",
    }),
  );
  return false;
}

export function showRepairProjectFeedback(
  result: RepairProjectDriftResult | null,
  t: TFunction,
): boolean {
  if (!result) {
    toast.error(
      t("kanban.readiness.repairError", {
        defaultValue: "修复请求失败，请稍后重试",
      }),
    );
    return false;
  }

  if (result.repaired_count > 0 && result.still_drifted_count === 0) {
    toast.success(
      t("kanban.portfolioDrift.repairAllSuccess", {
        count: result.repaired_count,
        defaultValue: `已修复 ${result.repaired_count} 项漂移并验证生效`,
      }),
    );
    return true;
  }

  if (result.repaired_count > 0 && result.still_drifted_count > 0) {
    toast.warning(
      t("kanban.portfolioDrift.repairPartial", {
        fixed: result.repaired_count,
        remaining: result.still_drifted_count,
        defaultValue: `已修复 ${result.repaired_count} 项，仍有 ${result.still_drifted_count} 项漂移`,
      }),
    );
    return false;
  }

  toast.info(
    t("kanban.portfolioDrift.repairNone", {
      defaultValue: "未发现可修复的漂移项",
    }),
  );
  return false;
}
