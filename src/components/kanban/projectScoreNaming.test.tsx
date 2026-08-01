import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";

import type { AgentReadinessItem } from "@/api/aiInsight";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import { ProjectCard } from "@/components/kanban/ProjectCard";
import { PortfolioHealthSummary } from "@/components/kanban/PortfolioHealthSummary";
import { ProjectAssetsMatrix } from "@/components/kanban/ProjectAssetsMatrix";
import { PROJECT_SCORE_META } from "@/lib/kanban/projectScores";
import { makeProjectView } from "../../../tests/projectViewFactory";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 「一个分数，三个名字，旁边还有一个不同的分数」（审查报告 §5.2）。
 *
 * 同一个 `agentReadinessMap.get(id).score` 被画在多个地方：项目卡片、组合健康
 * 清单、AI 资产矩阵。各处各写各的 —— 卡片挂了个 `title`，其余是**光秃秃一个
 * 数字加一个盾牌图标**，鼠标悬停什么也不说，读屏器念出来就是「42」。而卡片上
 * 紧挨着它还有另一个 0-100 的数字（AI 健康评分，来源完全不同），两者只靠
 * 「圆点 vs 盾牌」区分。
 *
 * 根因不是「忘了写 tooltip」，是 §6.1 说的那件事：14 个平行 Map 谁都能
 * `.get(id)`，每个消费方各自起名，**没有任何地方能发现它们说的是同一件事**。
 * 所以修法不是各处补 `title`，是让各处都从 `PROJECT_SCORE_META` 取名字 ——
 * 下次改名只有一处可改，想改歪都难。
 *
 * 断言用 `getByTitle` 而不是 `getByText`：数字本身在各处长得一样（都是「42」），
 * 能区分「说清楚了」和「没说清楚」的只有那个名字。
 *
 * ── 曾经是四处，现在是三处 ──
 * 第四处是「今日告警」里的待办行。它整块被删了（§3.1）：那份「建议优先处理」
 * 和同一屏的 `PortfolioHealthSummary` 读同一个 `agentReadinessMap`，却各算各的
 * 理由、各排各的序，于是同一个项目在上下两块里能给出不一样的说法。少一处画这个
 * 分数是**修好了**，不是丢了覆盖 —— 剩下三处仍然共用同一个 `PROJECT_SCORE_META`，
 * 这个文件守的正是这件事。TodayWorkspace 现在只出「平均就绪分」这类聚合数、
 * 不出项目级分数，它的用例在 `TodayWorkspace.test.tsx`。
 */

const PROJECT: Project = {
  id: "p1",
  name: "alpha",
  path: "E:/projects/alpha",
  addedAt: new Date("2026-07-01").toISOString(),
};

function item(partial: Partial<AgentReadinessItem> = {}): AgentReadinessItem {
  return {
    check_name: "mcp_enabled",
    label: "MCP 服务器",
    weight: 15,
    score: 0,
    detail: "",
    status: "missing",
    ...partial,
  };
}

/**
 * 已采纳一部分、还差一部分 —— 判定为 `warn`。
 * 这是四处唯一都会把分数显示出来的等级：`unmanaged` / `unscanned` 按
 * `shouldShowReadinessScore` 一律不出分。
 */
const PARTIALLY_READY: AgentReadinessItem[] = [
  item({ check_name: "mcp_enabled", weight: 15, score: 15, status: "ready" }),
  item({ check_name: "permissions", weight: 10, score: 0, status: "missing" }),
];

const READINESS: AgentReadinessBatchEntry = {
  score: 42,
  driftCount: 0,
  scannedAt: 1,
  details: PARTIALLY_READY,
};

const READINESS_MAP = new Map([["p1", READINESS]]);

/** 就绪分的规范名，四处必须一字不差地共用。 */
const READINESS_NAME = PROJECT_SCORE_META.agentReadiness.label;
/** 紧挨着它的那个「另一个分数」的规范名。 */
const HEALTH_NAME = PROJECT_SCORE_META.aiHealth.label;

describe("项目卡片上并排的两个分数", () => {
  it("P0 回归：各自说出自己是哪个分数，不能只靠图标形状区分", () => {
    renderWithProviders(
      <ProjectCard
        view={makeProjectView(PROJECT, {
          aiHealthScore: 88,
          // 就绪分从 `readiness.score` 来（42），不再是一个平行 prop：
          // 两个分数在同一个对象里各占一个字段，混不到一起去（§6.1）。
          readiness: READINESS,
        })}
        onClick={vi.fn()}
        onRemove={vi.fn()}
      />,
    );

    expect(
      screen.getByTitle(new RegExp(`${HEALTH_NAME} 88/100`)),
    ).toBeInTheDocument();
    expect(
      screen.getByTitle(new RegExp(`${READINESS_NAME} 42/100`)),
    ).toBeInTheDocument();
  });
});

describe("同一个就绪分在三处必须同名", () => {
  it("P0 回归：组合健康清单里的就绪分不是无名裸数字", () => {
    renderWithProviders(
      <PortfolioHealthSummary
        projects={[PROJECT]}
        agentReadinessMap={READINESS_MAP}
        assetMap={new Map()}
        onOpenProject={vi.fn()}
      />,
    );

    expect(
      screen.getByTitle(new RegExp(`${READINESS_NAME} 42/100`)),
    ).toBeInTheDocument();
  });

  it("P0 回归：AI 资产矩阵里的就绪分不是无名裸数字", () => {
    renderWithProviders(
      <ProjectAssetsMatrix
        projects={[PROJECT]}
        getStage={() => "mvp"}
        agentReadinessMap={READINESS_MAP}
        onOpenProject={vi.fn()}
        onOpenProjectAiConfig={vi.fn()}
      />,
    );

    expect(
      screen.getByTitle(new RegExp(`${READINESS_NAME} 42/100`)),
    ).toBeInTheDocument();
  });
});

describe("名字只有一个来源", () => {
  it("两个分数的规范名互不重叠 —— 重叠了就等于没区分", () => {
    expect(READINESS_NAME).not.toBe(HEALTH_NAME);
    expect(READINESS_NAME).not.toContain(HEALTH_NAME);
    expect(HEALTH_NAME).not.toContain(READINESS_NAME);
  });
});
