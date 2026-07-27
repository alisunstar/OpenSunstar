import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import { usePortfolioDerivedMetrics } from "@/hooks/kanban/usePortfolioDerivedMetrics";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { CodeLineResult } from "@/api/codeMetrics";
import type { Project } from "@/types/project";

/**
 * 组合矩阵的数据来源回归（审查报告 §2.5 + §5.6）。
 *
 * 改动前这里叠了两个互相掩护的缺陷：
 *
 * 1. `if (!aiConfigured) return []` —— 报告只点了组件侧的 `aiConfigured &&`，
 *    数据侧还藏着第二道门。矩阵不调用任何模型，纯 recharts 画本地数字，
 *    没配 API Key 的用户被挡在一张纯本地图表外面。
 * 2. `fallbackHealth = codeLines > 0 ? (activity > 0 ? 52 : 42) : ...`
 *    —— 缺 AI 健康分时**编一个数**当 Y 坐标。四个兜底值全部低于矩阵的 60 分
 *    横切线，「还没算出来」于是被画成「所有项目都在下半区」。
 *
 * 只删第 1 条会把第 2 条的后果推给全体无 Key 用户，所以两条必须一起守住：
 * **门控不得回来，编造值也不得回来。**
 */

const T = ((key: string, opts?: Record<string, unknown>) =>
  (opts?.defaultValue as string) ?? key) as unknown as TFunction;

/** 曾经被写死进代码的四个兜底健康分。任何一个再出现在图上都是回归。 */
const FABRICATED_SCORES = [52, 42, 48, 35];

function project(id: string): Project {
  return { id, name: id, path: `E:/repos/${id}` } as Project;
}

/**
 * 必须给全字段：同一个 hook 里的 `projectContextsMap` 会把它交给
 * `buildProjectContext`，那里直接 `languages.slice(0, 5)`（aiInsight.ts:341）。
 * 用 `as CodeLineResult` 硬凑一个残缺对象，崩的是 React 渲染，报错指向
 * react-dom 内部，和被测逻辑毫无关系。
 */
function codeMetrics(codeLines: number): CodeLineResult {
  return {
    total_lines: codeLines,
    code_lines: codeLines,
    comment_lines: 0,
    blank_lines: 0,
    files: 1,
    languages: [],
  };
}

function readiness(
  score: number,
  patch: Partial<AgentReadinessBatchEntry> = {},
): AgentReadinessBatchEntry {
  return {
    score,
    driftCount: 0,
    scannedAt: 1_760_000_000,
    assessmentState: "managed",
    details: [
      {
        check_name: "mcp_enabled",
        label: "MCP 服务器",
        weight: 15,
        score: 15,
        detail: "",
        status: "ready",
      },
    ],
    ...patch,
  };
}

interface Overrides {
  projects?: Project[];
  aiConfigured?: boolean;
  scanning?: boolean;
  aiHealthMap?: Map<string, number>;
  agentReadinessMap?: Map<string, AgentReadinessBatchEntry>;
  commits7dMap?: Map<string, number>;
  codeLinesMap?: Map<string, CodeLineResult>;
}

function renderMatrix(overrides: Overrides = {}) {
  const { result } = renderHook(() =>
    usePortfolioDerivedMetrics({
      projects: overrides.projects ?? [project("p1"), project("p2")],
      codeLinesMap: overrides.codeLinesMap ?? new Map(),
      gitInfoMap: new Map(),
      commits7dMap: overrides.commits7dMap ?? new Map(),
      commits30dMap: new Map(),
      weeklyCommitsMap: new Map(),
      contributorsMap: new Map(),
      versionMap: new Map(),
      progressMap: new Map(),
      aiHealthMap: overrides.aiHealthMap ?? new Map(),
      agentReadinessMap:
        overrides.agentReadinessMap ??
        new Map([
          ["p1", readiness(72)],
          ["p2", readiness(38)],
        ]),
      aiConfigured: overrides.aiConfigured ?? false,
      scanning: overrides.scanning ?? false,
      overviewWindowDays: 7,
      getStage: () => "mvp",
      t: T,
    }),
  );
  return result.current;
}

describe("矩阵不再被 aiConfigured 门在外面", () => {
  it("P0 回归：没配 API Key 也能拿到点 —— 这张图从头到尾没调过模型", () => {
    const { portfolioPoints, portfolioScoreKind } = renderMatrix({
      aiConfigured: false,
    });

    expect(portfolioPoints).toHaveLength(2);
    expect(portfolioScoreKind).toBe("agentReadiness");
    expect(portfolioPoints.map((p) => p.score)).toEqual([72, 38]);
  });

  it("扫描进行中仍然不出图 —— 那时活跃度只填了一半，会边看边跳", () => {
    expect(renderMatrix({ scanning: true }).portfolioPoints).toEqual([]);
  });
});

describe("Y 轴只画真实测量出来的分数", () => {
  it("P0 回归：拿不到分数的项目不上图，而不是被塞一个编造的坐标", () => {
    const { portfolioPoints } = renderMatrix({
      agentReadinessMap: new Map([["p1", readiness(72)]]),
    });

    expect(portfolioPoints.map((p) => p.projectId)).toEqual(["p1"]);
  });

  it("P0 回归：四个写死的兜底健康分不得再出现在任何点上", () => {
    // 这两项曾经决定编哪个数：有代码 + 有提交 → 52，都没有 → 35。
    const { portfolioPoints } = renderMatrix({
      agentReadinessMap: new Map(),
      commits7dMap: new Map([["p1", 9]]),
      codeLinesMap: new Map([["p1", codeMetrics(12000)]]),
    });

    expect(portfolioPoints).toEqual([]);
    for (const fake of FABRICATED_SCORES) {
      expect(portfolioPoints.map((p) => p.score)).not.toContain(fake);
    }
  });

  it("未纳管项目不上图 —— 与后端 `score: None` 同口径，零分不代表没配", () => {
    const { portfolioPoints } = renderMatrix({
      agentReadinessMap: new Map([
        ["p1", readiness(72)],
        ["p2", readiness(0, { assessmentState: "unmanaged" })],
      ]),
    });

    expect(portfolioPoints.map((p) => p.projectId)).toEqual(["p1"]);
  });

  it("从未扫描过的项目不上图 —— 空 details 是「不知道」，不是「零」", () => {
    const { portfolioPoints } = renderMatrix({
      agentReadinessMap: new Map([
        ["p1", readiness(72)],
        ["p2", readiness(0, { details: [] })],
      ]),
    });

    expect(portfolioPoints.map((p) => p.projectId)).toEqual(["p1"]);
  });
});

describe("两个 0-100 分数永不混画在同一条 Y 轴上", () => {
  it("有 AI 健康分时整张图切到健康分", () => {
    const { portfolioPoints, portfolioScoreKind } = renderMatrix({
      aiConfigured: true,
      aiHealthMap: new Map([
        ["p1", 88],
        ["p2", 64],
      ]),
    });

    expect(portfolioScoreKind).toBe("aiHealth");
    expect(portfolioPoints.map((p) => p.score)).toEqual([88, 64]);
  });

  it("P0 回归：健康分模式下缺分的项目不得被就绪分顶替上去", () => {
    // p2 有就绪分 38、没有健康分。若把 38 画进健康分那条轴，读图的人会拿
    // 「配置只配了 38%」当成「工程健康度 38」—— §5.2「一个分数，三个名字」
    // 升级成一张会误导决策的图。
    const { portfolioPoints, portfolioScoreKind } = renderMatrix({
      aiConfigured: true,
      aiHealthMap: new Map([["p1", 88]]),
    });

    expect(portfolioScoreKind).toBe("aiHealth");
    expect(portfolioPoints.map((p) => p.projectId)).toEqual(["p1"]);
    expect(portfolioPoints.map((p) => p.score)).not.toContain(38);
  });

  it("配了 Key 但一份健康分都还没算出来时，退回就绪分而不是空图", () => {
    const { portfolioScoreKind, portfolioPoints } = renderMatrix({
      aiConfigured: true,
      aiHealthMap: new Map(),
    });

    expect(portfolioScoreKind).toBe("agentReadiness");
    expect(portfolioPoints).toHaveLength(2);
  });
});
