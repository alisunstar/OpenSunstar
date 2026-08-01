import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { AgentReadinessItem, AgentReadinessResult } from "@/api/aiInsight";
import { AgentReadinessPanel } from "@/components/kanban/AgentReadinessPanel";
import { renderWithProviders } from "../../../tests/renderWithProviders";

function item(
  label: string,
  overrides: Partial<AgentReadinessItem> = {},
): AgentReadinessItem {
  return {
    check_name: `${label.toLowerCase()}_enabled`,
    label,
    weight: 12,
    score: 0,
    detail: `${label} 尚未配置`,
    status: "missing",
    configured_state: "unconfigured",
    effective_state: "unchecked",
    ...overrides,
  };
}

const DATA: AgentReadinessResult = {
  score: 24,
  max_score: 100,
  details: [
    item("MCP", {
      check_name: "mcp_enabled",
      score: 12,
      status: "ready",
      configured_state: "configured",
      effective_state: "effective",
    }),
    item("Prompts", {
      check_name: "prompt_files",
      score: 12,
      status: "ready",
      configured_state: "configured",
      effective_state: "effective",
    }),
    item("Skills", {
      check_name: "skills_configured",
      status: "unhealthy",
      configured_state: "configured",
      effective_state: "drifted",
      effective_scanned_at: 1,
      effective_detail: "Skills 配置与预期不一致",
    }),
    item("Commands", { check_name: "commands_configured" }),
    item("Hooks", { check_name: "hooks_configured" }),
    item("Ignore", { check_name: "ignore_rules" }),
    item("Permissions", { check_name: "permissions" }),
    item("Subagents", { check_name: "subagents_configured" }),
  ],
  llm_suggestion: null,
  is_cached: false,
  target_app: "codex",
};

describe("AgentReadinessPanel compact 摘要", () => {
  it("只显示得分、分类计数、最高优先问题和单一 CTA", async () => {
    const user = userEvent.setup();
    const onOpenProjectAssets = vi.fn();

    renderWithProviders(
      <AgentReadinessPanel
        data={DATA}
        compact
        onRefresh={vi.fn()}
        onScanEffective={vi.fn()}
        onRepairDrift={vi.fn()}
        onNavigate={vi.fn()}
        onOpenProjectAssets={onOpenProjectAssets}
      />,
    );

    expect(screen.getByText("项目配置就绪度")).toBeInTheDocument();
    expect(screen.getByText("24")).toBeInTheDocument();
    expect(screen.getByText("2 项正常")).toBeInTheDocument();
    expect(screen.getByText("1 项不一致")).toBeInTheDocument();
    expect(screen.getByText("5 项缺失")).toBeInTheDocument();
    expect(screen.getByText(/优先处理：Skills 配置不一致/)).toBeInTheDocument();

    expect(
      screen.queryByRole("button", { name: "生效态扫描" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "修复配置" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "去配置" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "打开AI资产配置" }));
    expect(onOpenProjectAssets).toHaveBeenCalledTimes(1);
  });

  it("完整模式仍保留诊断与修复能力", () => {
    renderWithProviders(
      <AgentReadinessPanel
        data={DATA}
        onScanEffective={vi.fn()}
        onRepairDrift={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: "生效态扫描" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "修复配置" }),
    ).toBeInTheDocument();
  });

  it("完整模式可刷新、扫描、展开已完成项、跳转配置并确认修复", async () => {
    const user = userEvent.setup();
    const onRefresh = vi.fn();
    const onScanEffective = vi.fn();
    const onOpenProjectAssets = vi.fn();
    const onRepairDrift = vi.fn().mockResolvedValue(undefined);

    renderWithProviders(
      <AgentReadinessPanel
        data={{
          ...DATA,
          evaluated_at: 1,
          llm_suggestion: "建议先修复 Skills",
        }}
        onRefresh={onRefresh}
        onScanEffective={onScanEffective}
        onOpenProjectAssets={onOpenProjectAssets}
        onRepairDrift={onRepairDrift}
      />,
    );

    await user.click(screen.getByRole("button", { name: "刷新" }));
    await user.click(screen.getByRole("button", { name: "生效态扫描" }));
    expect(onRefresh).toHaveBeenCalledTimes(1);
    expect(onScanEffective).toHaveBeenCalledTimes(1);

    await user.click(screen.getAllByRole("button", { name: /去配置/ })[0]);
    expect(onOpenProjectAssets).toHaveBeenCalledWith("skill");

    await user.click(screen.getByRole("button", { name: "已完成 2 项" }));
    expect(screen.getByText("MCP")).toBeInTheDocument();
    expect(screen.getByText("建议先修复 Skills")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "修复配置" }));
    expect(
      screen.getByRole("heading", { name: "确认写回修复？" }),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "确认写回" }));
    expect(onRepairDrift).toHaveBeenCalledWith("skills_configured");
  });

  it("无缓存数据时分别显示加载态与空态", () => {
    const { rerender } = renderWithProviders(
      <AgentReadinessPanel data={null} isLoading />,
    );

    expect(screen.getByText("正在评估配置就绪度…")).toBeInTheDocument();

    rerender(<AgentReadinessPanel data={null} />);
    expect(screen.queryByText("正在评估配置就绪度…")).not.toBeInTheDocument();
  });
});
