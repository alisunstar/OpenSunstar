import type { AgentReadinessItem, ReadinessItemStatus } from "@/api/aiInsight";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";

/**
 * 组合层项目健康等级。
 *
 * 设计要点：**score 是采纳度指标，不是告警阈值。**
 * 一个刚加入、尚未通过 OpenSunstar 关联任何资产的项目分数必然接近 0，
 * 但那是「还没配」而不是「坏了」，不能渲染成红色告警。
 * 因此等级由条目 status 分布 + 真实漂移数驱动，score 仅作展示。
 *
 * 对应 Rust：`src-tauri/src/ai/agent_readiness.rs:11-19` 的 status 常量、
 * `:387-413` classify_unmanaged_readiness、`:415-421` readiness_item_is_actionable_gap。
 */
export type PortfolioHealthLevel =
  | "ok"
  | "warn"
  | "alert"
  | "unconfigured"
  | "unmanaged"
  | "unscanned";

/** 通过 OpenSunstar 真正采纳的状态。磁盘线索与全局默认都不算。 */
const ADOPTED_STATUSES = new Set<ReadinessItemStatus>(["ready", "partial"]);

/**
 * 不可判定状态：零计数不能证明缺失，或该能力对目标 CLI 不适用。
 * 与 `readiness_item_is_actionable_gap`（agent_readiness.rs:415-421）的排除集一致。
 */
const INDETERMINATE_STATUSES = new Set<ReadinessItemStatus>([
  "not_required",
  "unmanaged",
  "unknown",
]);

export function isAdoptedStatus(
  status: ReadinessItemStatus | null | undefined,
): boolean {
  return status != null && ADOPTED_STATUSES.has(status);
}

export function isIndeterminateStatus(
  status: ReadinessItemStatus | null | undefined,
): boolean {
  return status != null && INDETERMINATE_STATUSES.has(status);
}

/** 与 Rust `readiness_item_is_actionable_gap` 同口径的可行动缺口判定。 */
export function isActionableGap(item: AgentReadinessItem): boolean {
  return item.score < item.weight && !isIndeterminateStatus(item.status);
}

/** 后端 `assessment_state` 的未纳管取值（cli_api.rs:403 / commands/ai_insight.rs）。 */
export function isUnmanagedAssessment(
  assessmentState: string | null | undefined,
): boolean {
  return assessmentState === "unmanaged";
}

/**
 * 判定单个项目的健康等级。
 *
 * 优先级：未扫描 → 未纳管 → 真实漂移 → 尚未配置 → 有缺口 → 正常
 */
export function classifyReadinessLevel(
  readiness: AgentReadinessBatchEntry | undefined,
): PortfolioHealthLevel {
  if (!readiness) return "unscanned";

  if (isUnmanagedAssessment(readiness.assessmentState)) return "unmanaged";

  const details = readiness.details ?? [];
  if (details.length === 0) return "unscanned";

  // 漂移 = 库内容与目标 CLI 实际生效内容不一致，是唯一真正「坏了」的信号。
  // 与 governanceStats.ts:40 的口径统一，避免同屏出现两个互相矛盾的计数。
  if (readiness.driftCount > 0) return "alert";

  const determinate = details.filter((d) => !isIndeterminateStatus(d.status));
  // 全部条目都不可判定 —— 后端已表态「不能判定为缺失」
  if (determinate.length === 0) return "unmanaged";

  const adopted = determinate.filter((d) => isAdoptedStatus(d.status));
  if (adopted.length === 0) return "unconfigured";

  return determinate.some(isActionableGap) ? "warn" : "ok";
}

/** 未纳管项目不展示就绪分，与 CLI `score: None`（cli_api.rs:404）口径一致。 */
export function shouldShowReadinessScore(level: PortfolioHealthLevel): boolean {
  return level !== "unmanaged" && level !== "unscanned";
}
