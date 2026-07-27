import { describe, expect, it } from "vitest";

import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import {
  buildProjectViews,
  indexProjectViews,
  pickProjectViews,
  type ProjectViewSources,
} from "@/lib/kanban/projectView";

/**
 * `ProjectView` 聚合对象（审查报告 §6.1）。
 *
 * `KanbanPage.tsx` 里按 `project.id` 索引的 Map 有 14 个，散在五个 hook 的返回值
 * 里，没有任何东西把它们合起来。后果不是「代码丑」：每个消费方各自
 * `xxxMap.get(id)`、各自起名、各自决定缺失时怎么办 —— §5.2 那个「一个分数三个
 * 名字」就是这么来的。
 *
 * 这里守三件事：
 *
 * 1. **不串号。** 14 次 `.get(id)` 收敛成一次，id 只出现一处，串号这类错误
 *    在类型层面消失。
 * 2. **缺失就是缺失。** 「还没扫到」必须留 `undefined` 一路传到 UI，不能在聚合
 *    这一层顺手 `?? 0` —— 那样「没扫到」会被画成「0 行代码 / 0 次提交」，
 *    和 §5.6「查询失败不等于 ¥0」是同一个错误。
 * 3. **两个分数两个字段。** AI 健康评分与 Agent 配置就绪分来源完全不同，
 *    聚合对象必须让它们叫不同的名字，否则下游想混也拦不住。
 */

function project(id: string, name: string): Project {
  return {
    id,
    name,
    path: `E:/projects/${name}`,
    addedAt: new Date("2026-07-01").toISOString(),
  };
}

const ALPHA = project("p1", "alpha");
const BETA = project("p2", "beta");

function readiness(score: number): AgentReadinessBatchEntry {
  return { score, driftCount: 0, scannedAt: 1, details: [] };
}

/** 全空的数据源；每个用例只填自己关心的那几张表。 */
function sources(
  partial: Partial<ProjectViewSources> = {},
): ProjectViewSources {
  return {
    projects: [ALPHA, BETA],
    getStage: () => "mvp",
    progressMap: new Map(),
    codeLinesMap: new Map(),
    versionMap: new Map(),
    gitInfoMap: new Map(),
    commits7dMap: new Map(),
    commits30dMap: new Map(),
    contributorsMap: new Map(),
    weeklyCommitsMap: new Map(),
    aiSummaryMap: new Map(),
    aiLoadingMap: new Map(),
    aiHealthMap: new Map(),
    aiTrendInsightMap: new Map(),
    agentReadinessMap: new Map(),
    assetMap: new Map(),
    projectContextsMap: new Map(),
    ...partial,
  };
}

describe("buildProjectViews 收敛 14 个平行 Map", () => {
  it("每个项目只拿自己那一行，不串号", () => {
    const views = buildProjectViews(
      sources({
        getStage: (id) => (id === "p1" ? "rapid" : "stable"),
        commits7dMap: new Map([
          ["p1", 12],
          ["p2", 3],
        ]),
        versionMap: new Map([
          ["p1", "1.1.9"],
          ["p2", "0.2.0"],
        ]),
        agentReadinessMap: new Map([
          ["p1", readiness(42)],
          ["p2", readiness(91)],
        ]),
      }),
    );

    expect(views).toHaveLength(2);
    expect(views[0]).toMatchObject({
      id: "p1",
      stage: "rapid",
      commits7d: 12,
      version: "1.1.9",
    });
    expect(views[0].readiness?.score).toBe(42);
    expect(views[1]).toMatchObject({
      id: "p2",
      stage: "stable",
      commits7d: 3,
      version: "0.2.0",
    });
    expect(views[1].readiness?.score).toBe(91);
  });

  it("顺序跟着 projects 走 —— 下游排序自己负责，聚合不擅自重排", () => {
    const views = buildProjectViews(sources({ projects: [BETA, ALPHA] }));
    expect(views.map((v) => v.id)).toEqual(["p2", "p1"]);
  });

  it("P0 回归：没扫到就留 undefined，不许在聚合层补 0", () => {
    // 「还没扫到」和「扫到了，是 0」是两件事。这里补一个 0，UI 就会理直气壮地
    // 画出「0 行代码 / 近 7 天 0 次提交」，用户据此判断项目已死 —— 与
    // §5.6「查询失败不等于 ¥0」同一类谎。
    const [view] = buildProjectViews(sources({ projects: [ALPHA] }));

    expect(view.codeLines).toBeUndefined();
    expect(view.commits7d).toBeUndefined();
    expect(view.commits30d).toBeUndefined();
    expect(view.contributors).toBeUndefined();
    expect(view.weeklyCommits).toBeUndefined();
    expect(view.version).toBeUndefined();
    expect(view.gitInfo).toBeUndefined();
    expect(view.progress).toBeUndefined();
    expect(view.readiness).toBeUndefined();
    expect(view.assets).toBeUndefined();
    expect(view.aiHealthScore).toBeUndefined();
  });

  it("aiSummaryLoading 缺省是 false —— 「没在转圈」是确定的，不必留 undefined", () => {
    const [view] = buildProjectViews(sources({ projects: [ALPHA] }));
    expect(view.aiSummaryLoading).toBe(false);
  });

  it("P0 回归：两个分数占两个字段，名字不重叠", () => {
    // 88 是 AI 给的健康评分，42 是配置扫描出来的就绪分，来源毫不相干。
    // 聚合对象把它们放进同名字段（或只留一个 `score`）就等于承认可以互换。
    const [view] = buildProjectViews(
      sources({
        projects: [ALPHA],
        aiHealthMap: new Map([["p1", 88]]),
        agentReadinessMap: new Map([["p1", readiness(42)]]),
      }),
    );

    expect(view.aiHealthScore).toBe(88);
    expect(view.readiness?.score).toBe(42);
  });
});

describe("indexProjectViews / pickProjectViews", () => {
  it("按 id 建索引，供抽屉这类「只要一个」的消费方直接取", () => {
    const map = indexProjectViews(buildProjectViews(sources()));
    expect(map.get("p2")?.project.name).toBe("beta");
    expect(map.get("nope")).toBeUndefined();
  });

  it("按给定项目顺序取回视图，索引里没有的静默跳过", () => {
    // 搜索/分组过滤后的 `Project[]` 与视图表可能短暂不同步（例如项目刚被移除、
    // 视图还没重建）。这里丢一行，总比抛异常把整块看板打空好。
    const map = indexProjectViews(buildProjectViews(sources()));
    const picked = pickProjectViews(map, [BETA, project("ghost", "ghost")]);

    expect(picked.map((v) => v.id)).toEqual(["p2"]);
  });
});
