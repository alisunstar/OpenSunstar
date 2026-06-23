import { describe, expect, it } from "vitest";
import {
  PORTFOLIO_COMMIT_WINDOW_DAYS,
  activityTier7d,
  currentWeekCommits,
} from "@/lib/portfolioMetrics";

describe("portfolioMetrics", () => {
  it("uses 7-day window constant", () => {
    expect(PORTFOLIO_COMMIT_WINDOW_DAYS).toBe(7);
  });

  it("reads current week from weekly array tail", () => {
    expect(currentWeekCommits([1, 2, 5])).toBe(5);
    expect(currentWeekCommits([])).toBe(0);
  });

  it("tiers 7-day commit counts", () => {
    expect(activityTier7d(0)).toBe(1);
    expect(activityTier7d(1)).toBe(2);
    expect(activityTier7d(3)).toBe(3);
    expect(activityTier7d(10)).toBe(4);
  });
});
