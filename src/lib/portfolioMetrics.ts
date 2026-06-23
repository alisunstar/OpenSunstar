/** 看板卡片、组合矩阵、AI 周报统一的 Git 活跃窗口（滚动 7 天） */
export const PORTFOLIO_COMMIT_WINDOW_DAYS = 7;

/** 项目总览可选统计窗口（方案 A：仅总览，不含 90 天） */
export type PortfolioOverviewWindowDays = 7 | 30;

export const PORTFOLIO_OVERVIEW_WINDOW_OPTIONS: PortfolioOverviewWindowDays[] = [
  7, 30,
];

/** 近 12 周数组最后一项 = 当前自然周提交数（git_weekly_commit_counts） */
export function currentWeekCommits(weekly: number[]): number {
  if (weekly.length === 0) return 0;
  return weekly[weekly.length - 1] ?? 0;
}

/** 7 天活跃度分级（平均活跃度卡片） */
export function activityTier7d(count: number): 1 | 2 | 3 | 4 {
  return activityTierForWindow(count, PORTFOLIO_COMMIT_WINDOW_DAYS);
}

/**
 * 按滚动窗口分级活跃度（7 天阈值为基准，其它窗口同比缩放）
 */
export function activityTierForWindow(
  commitCount: number,
  windowDays: PortfolioOverviewWindowDays,
): 1 | 2 | 3 | 4 {
  const scale = windowDays / PORTFOLIO_COMMIT_WINDOW_DAYS;
  const veryHigh = Math.round(10 * scale);
  const high = Math.round(3 * scale);
  const medium = Math.max(1, Math.round(1 * scale));
  if (commitCount >= veryHigh) return 4;
  if (commitCount >= high) return 3;
  if (commitCount >= medium) return 2;
  return 1;
}

/** 大数字紧凑展示（1.2K / 3.4M） */
export function formatCompactNumber(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1).replace(/\.0$/, "") + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(1).replace(/\.0$/, "") + "K";
  return n.toLocaleString();
}

/** 有限并发执行 async 任务 */
export async function mapWithConcurrency<T, R>(
  items: T[],
  limit: number,
  fn: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  if (items.length === 0) return [];
  const results = new Array<R>(items.length);
  let nextIndex = 0;

  const worker = async () => {
    while (nextIndex < items.length) {
      const i = nextIndex;
      nextIndex += 1;
      results[i] = await fn(items[i], i);
    }
  };

  const workers = Array.from(
    { length: Math.min(Math.max(1, limit), items.length) },
    () => worker(),
  );
  await Promise.all(workers);
  return results;
}
