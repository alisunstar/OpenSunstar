import type { AgentReadinessItem } from "@/api/aiInsight";

export type ConfiguredState = "configured" | "unconfigured";
export type EffectiveState =
  | "effective"
  | "drifted"
  | "unchecked"
  | "not_applicable";

/** 从 readiness 项推导配置态（兼容无 configured_state 的旧数据） */
export function resolveConfiguredState(
  item: AgentReadinessItem,
): ConfiguredState {
  if (item.configured_state === "configured") return "configured";
  if (item.configured_state === "unconfigured") return "unconfigured";
  return item.score > 0 ? "configured" : "unconfigured";
}

export function hasEffectiveScan(item: AgentReadinessItem): boolean {
  return Boolean(item.effective_state && item.effective_scanned_at);
}

export function effectiveBadgeTone(
  effectiveState: string | null | undefined,
): "success" | "warning" | "muted" | "none" {
  switch (effectiveState) {
    case "effective":
      return "success";
    case "drifted":
      return "warning";
    case "unchecked":
    case "not_applicable":
      return "muted";
    default:
      return "none";
  }
}
