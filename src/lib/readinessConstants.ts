/** Agent 配置就绪度满分（与后端 AGENT_READINESS_MAX_SCORE 一致） */
export const AGENT_READINESS_MAX = 100;

/** 达到此分数视为「配置良好」（约 75% 满分，对齐旧 60/80） */
export const READINESS_OK_THRESHOLD = 75;

/** 警告区间下限（约 50% 满分，对齐旧 40/80） */
export const READINESS_WARN_THRESHOLD = 50;

export function readinessMaxScore(
  maxScore?: number | null,
): number {
  return maxScore && maxScore > 0 ? maxScore : AGENT_READINESS_MAX;
}

export function readinessScoreTone(
  score: number,
  maxScore: number = AGENT_READINESS_MAX,
): string {
  if (score >= READINESS_OK_THRESHOLD && maxScore === AGENT_READINESS_MAX) {
    return "text-emerald-500";
  }
  if (score >= READINESS_WARN_THRESHOLD && maxScore === AGENT_READINESS_MAX) {
    return "text-amber-500";
  }
  // 兼容旧缓存 80 分制
  if (maxScore === 80) {
    if (score >= 60) return "text-emerald-500";
    if (score >= 40) return "text-amber-500";
    return "text-zinc-400";
  }
  const ratio = maxScore > 0 ? score / maxScore : 0;
  if (ratio >= 0.75) return "text-emerald-500";
  if (ratio >= 0.5) return "text-amber-500";
  return "text-zinc-400";
}

export function isReadinessOk(
  score: number,
  maxScore: number = AGENT_READINESS_MAX,
): boolean {
  if (maxScore === AGENT_READINESS_MAX) return score >= READINESS_OK_THRESHOLD;
  if (maxScore === 80) return score >= 60;
  return maxScore > 0 && score / maxScore >= 0.75;
}
