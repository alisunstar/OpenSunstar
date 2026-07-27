import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { AgentReadinessItem } from "@/api/aiInsight";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { Project } from "@/types/project";
import { PortfolioHealthSummary } from "@/components/kanban/PortfolioHealthSummary";
import { renderWithProviders } from "../../../tests/renderWithProviders";

function project(id: string, name: string): Project {
  return {
    id,
    name,
    path: `E:/projects/${name}`,
    addedAt: new Date("2026-07-01").toISOString(),
  };
}

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

function entry(
  partial: Partial<AgentReadinessBatchEntry> = {},
): AgentReadinessBatchEntry {
  return { score: 0, driftCount: 0, scannedAt: 1, details: [], ...partial };
}

/** 刚加入、尚未通过 OpenSunstar 关联任何资产的项目：8 项全 missing，总分 0 */
function freshlyAddedEntry(): AgentReadinessBatchEntry {
  return entry({
    score: 0,
    driftCount: 0,
    details: [
      item({ check_name: "mcp_enabled", weight: 15 }),
      item({ check_name: "skills_configured", weight: 12 }),
      item({ check_name: "prompt_files", weight: 12 }),
      item({ check_name: "commands_configured", weight: 10 }),
      item({ check_name: "hooks_configured", weight: 10 }),
      item({ check_name: "ignore_rules", weight: 10 }),
      item({ check_name: "permissions", weight: 10 }),
      item({ check_name: "subagents_configured", weight: 12 }),
    ],
  });
}

function renderSummary(
  projects: Project[],
  readiness: Array<[string, AgentReadinessBatchEntry]>,
) {
  return renderWithProviders(
    <PortfolioHealthSummary
      projects={projects}
      agentReadinessMap={new Map(readiness)}
      assetMap={new Map()}
      onOpenProject={vi.fn()}
    />,
  );
}

describe("PortfolioHealthSummary 非空态", () => {
  it("P0 回归：刚加入且零配置的项目显示为「尚未配置」，不得计入「异常」", () => {
    renderSummary([project("p1", "alpha")], [["p1", freshlyAddedEntry()]]);

    expect(screen.getByText("尚未配置")).toBeInTheDocument();
    expect(screen.queryByText("异常")).not.toBeInTheDocument();
    expect(
      screen.getByText(/尚未通过 OpenSunstar 关联任何 AI 资产/),
    ).toBeInTheDocument();
  });

  it("只有真实漂移才算「异常」", () => {
    renderSummary(
      [project("p1", "alpha")],
      [
        [
          "p1",
          entry({
            score: 60,
            driftCount: 2,
            details: [
              item({ score: 15, status: "ready", effective_state: "drifted" }),
            ],
          }),
        ],
      ],
    );

    expect(screen.getByText("异常")).toBeInTheDocument();
    expect(screen.getByText(/2 处配置与预期不一致/)).toBeInTheDocument();
  });

  it("已采纳但仍有缺口 → 需关注，不是异常", () => {
    renderSummary(
      [project("p1", "alpha")],
      [
        [
          "p1",
          entry({
            score: 15,
            driftCount: 0,
            details: [
              item({ weight: 15, score: 15, status: "ready" }),
              item({
                check_name: "skills_configured",
                label: "Skills",
                weight: 12,
                score: 0,
                status: "missing",
              }),
            ],
          }),
        ],
      ],
    );

    expect(screen.getByText("需关注")).toBeInTheDocument();
    expect(screen.queryByText("异常")).not.toBeInTheDocument();
  });

  it("后端判定 unmanaged 时显示「未纳管」且不展示分数", () => {
    renderSummary(
      [project("p1", "alpha")],
      [
        [
          "p1",
          entry({
            score: 0,
            assessmentState: "unmanaged",
            details: [item({ status: "unmanaged" })],
          }),
        ],
      ],
    );

    expect(screen.getByText("未纳管")).toBeInTheDocument();
    expect(
      screen.getByText(/项目尚未纳入 OpenSunstar，不能判定为缺失/),
    ).toBeInTheDocument();
    // 未纳管项目不得展示就绪分（与 CLI `score: null` 口径一致）
    expect(screen.queryByText("0")).not.toBeInTheDocument();
  });

  it("多项目：等级计数互不串味", () => {
    renderSummary(
      [
        project("p1", "fresh"),
        project("p2", "drifted"),
        project("p3", "healthy"),
      ],
      [
        ["p1", freshlyAddedEntry()],
        [
          "p2",
          entry({
            score: 60,
            driftCount: 1,
            details: [
              item({ score: 15, status: "ready", effective_state: "drifted" }),
            ],
          }),
        ],
        [
          "p3",
          entry({
            score: 27,
            details: [
              item({ weight: 15, score: 15, status: "ready" }),
              item({
                check_name: "skills_configured",
                weight: 12,
                score: 12,
                status: "ready",
              }),
            ],
          }),
        ],
      ],
    );

    expect(screen.getByText("尚未配置")).toBeInTheDocument();
    expect(screen.getByText("异常")).toBeInTheDocument();
    expect(screen.getByText("正常")).toBeInTheDocument();
    // 未出现在问题列表里的只有健康项目
    expect(screen.getByText("fresh")).toBeInTheDocument();
    expect(screen.getByText("drifted")).toBeInTheDocument();
    expect(screen.queryByText("healthy")).not.toBeInTheDocument();
  });

  it("全部就绪时给出 all-clear", () => {
    renderSummary(
      [project("p1", "alpha")],
      [
        [
          "p1",
          entry({
            score: 27,
            details: [
              item({ weight: 15, score: 15, status: "ready" }),
              item({
                check_name: "subagents_configured",
                weight: 12,
                score: 12,
                status: "not_required",
              }),
            ],
          }),
        ],
      ],
    );

    expect(screen.getByText("所有项目 AI 配置状态正常。")).toBeInTheDocument();
  });

  it("没有就绪度数据 → 未扫描", () => {
    renderSummary([project("p1", "alpha")], []);
    expect(screen.getByText("未扫描")).toBeInTheDocument();
    expect(screen.getByText("尚未完成 AI 配置状态扫描")).toBeInTheDocument();
  });
});

/**
 * 「查看修复」不修复（审查报告 §4.3）。
 *
 * 原来六个等级共用一个 `onOpenProject(project, { assetsTab: true })`，其中
 * `alert` 的标签写「查看修复」—— 点下去只是打开抽屉，什么都没修。真正的
 * 修复链路在 `PortfolioDriftSummary.tsx:138-161 → KanbanPage.tsx
 * handleRepairProjectDrift`。
 *
 * 按钮文案是一种承诺。要么兑现，要么改口。
 */
describe("PortfolioHealthSummary 修复动作", () => {
  const DRIFTED: Array<[string, AgentReadinessBatchEntry]> = [
    [
      "p1",
      entry({
        score: 60,
        driftCount: 2,
        details: [
          item({ score: 15, status: "ready", effective_state: "drifted" }),
        ],
      }),
    ],
  ];

  function renderWithRepair(
    overrides: {
      onRepairProject?: (p: Project) => void;
      onOpenProject?: (p: Project) => void;
      onOpenProjectAiConfig?: (p: Project) => void;
      repairingProjectId?: string | null;
    } = {},
  ) {
    const onOpenProject = overrides.onOpenProject ?? vi.fn();
    const onOpenProjectAiConfig = overrides.onOpenProjectAiConfig ?? vi.fn();
    return {
      onOpenProject,
      onOpenProjectAiConfig,
      ...renderWithProviders(
        <PortfolioHealthSummary
          projects={[project("p1", "alpha")]}
          agentReadinessMap={new Map(DRIFTED)}
          assetMap={new Map()}
          onOpenProject={onOpenProject}
          onOpenProjectAiConfig={onOpenProjectAiConfig}
          onRepairProject={overrides.onRepairProject}
          repairingProjectId={overrides.repairingProjectId ?? null}
        />,
      ),
    };
  }

  it("异常项目的按钮必须触发真修复，而不是只打开抽屉", async () => {
    const onRepairProject = vi.fn();
    const { onOpenProject, onOpenProjectAiConfig } = renderWithRepair({
      onRepairProject,
    });

    await userEvent.click(screen.getByRole("button", { name: /修复/ }));

    expect(onRepairProject).toHaveBeenCalledTimes(1);
    expect(onRepairProject.mock.calls[0]?.[0]?.id).toBe("p1");
    // 修复入口不能顺手把抽屉/配置页也打开：预览确认框会被盖住
    expect(onOpenProject).not.toHaveBeenCalled();
    expect(onOpenProjectAiConfig).not.toHaveBeenCalled();
  });

  it("没有修复回调时文案必须退回「查看详情」，不得写「修复」", () => {
    renderWithRepair();

    expect(screen.queryByRole("button", { name: /修复/ })).toBeNull();
    expect(
      screen.getByRole("button", { name: "查看详情" }),
    ).toBeInTheDocument();
  });

  it("该项目正在修复时按钮禁用，避免重复提交", () => {
    renderWithRepair({ onRepairProject: vi.fn(), repairingProjectId: "p1" });

    expect(screen.getByRole("button", { name: /修复/ })).toBeDisabled();
  });

  it("「去配置」跳配置页而不是开抽屉，也不误接修复", async () => {
    const onRepairProject = vi.fn();
    const onOpenProject = vi.fn();
    const onOpenProjectAiConfig = vi.fn();

    renderWithProviders(
      <PortfolioHealthSummary
        projects={[project("p1", "alpha")]}
        agentReadinessMap={new Map([["p1", freshlyAddedEntry()]])}
        assetMap={new Map()}
        onOpenProject={onOpenProject}
        onOpenProjectAiConfig={onOpenProjectAiConfig}
        onRepairProject={onRepairProject}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "去配置" }));

    expect(onRepairProject).not.toHaveBeenCalled();
    expect(onOpenProjectAiConfig).toHaveBeenCalledWith(
      expect.objectContaining({ id: "p1" }),
    );
    expect(onOpenProject).not.toHaveBeenCalled();
  });

  /**
   * 按钮文案 = 承诺，落点必须兑现（审查报告 §4.3）。
   *
   * 上一条盯的是「说配置就去配置页」；这一条盯反方向 —— 说「查看项目」就只是
   * 打开抽屉看看，不能把人丢到一个要动手勾选的配置页上。六个等级以前共用
   * `{ assetsTab: true }`，两个方向里错了一个。
   */
  it("「查看项目」开抽屉，不跳配置页", async () => {
    const onOpenProject = vi.fn();
    const onOpenProjectAiConfig = vi.fn();

    renderWithProviders(
      <PortfolioHealthSummary
        projects={[project("p1", "alpha")]}
        agentReadinessMap={new Map()}
        assetMap={new Map()}
        onOpenProject={onOpenProject}
        onOpenProjectAiConfig={onOpenProjectAiConfig}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "查看项目" }));

    expect(onOpenProject).toHaveBeenCalledWith(
      expect.objectContaining({ id: "p1" }),
    );
    expect(onOpenProjectAiConfig).not.toHaveBeenCalled();
  });
});
