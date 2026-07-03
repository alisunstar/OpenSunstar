import type { AgentReadinessItem } from "@/api/aiInsight";

/** 组合层批量就绪度条目（S2-01/02） */
export interface AgentReadinessBatchEntry {
  score: number;
  driftCount: number;
  /** Unix 秒；来自 evaluated_at 或生效态扫描 */
  scannedAt: number | null;
  details: AgentReadinessItem[];
}

export function countDriftItems(details: AgentReadinessItem[]): number {
  return details.filter((d) => d.effective_state === "drifted").length;
}

export function pickScannedAt(
  evaluatedAt?: number | null,
  details?: AgentReadinessItem[],
): number | null {
  if (evaluatedAt != null) return evaluatedAt;
  const fromItems = details
    ?.map((d) => d.effective_scanned_at)
    .filter((t): t is number => typeof t === "number");
  if (!fromItems?.length) return null;
  return Math.max(...fromItems);
}
