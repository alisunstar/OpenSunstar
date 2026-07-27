import { describe, expect, it } from "vitest";

import type { AgentReadinessItem } from "@/api/aiInsight";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import {
  classifyReadinessLevel,
  isActionableGap,
  isAdoptedStatus,
  isIndeterminateStatus,
} from "@/lib/portfolioHealth";

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
  return {
    score: 0,
    driftCount: 0,
    scannedAt: 1,
    details: [],
    ...partial,
  };
}

describe("status 谓词", () => {
  it("只有 ready / partial 算作已采纳", () => {
    expect(isAdoptedStatus("ready")).toBe(true);
    expect(isAdoptedStatus("partial")).toBe(true);
    // 磁盘上发现文件、或使用全局默认，都不是「已通过 OpenSunstar 采纳」
    expect(isAdoptedStatus("detected_only")).toBe(false);
    expect(isAdoptedStatus("global_only")).toBe(false);
    expect(isAdoptedStatus("missing")).toBe(false);
  });

  it("unmanaged / unknown / not_required 为不可判定，不得计为缺失", () => {
    expect(isIndeterminateStatus("unmanaged")).toBe(true);
    expect(isIndeterminateStatus("unknown")).toBe(true);
    expect(isIndeterminateStatus("not_required")).toBe(true);
    expect(isIndeterminateStatus("missing")).toBe(false);
    expect(isIndeterminateStatus(null)).toBe(false);
  });

  it("isActionableGap 与 Rust readiness_item_is_actionable_gap 口径一致", () => {
    // agent_readiness.rs:415-421 —— score < weight 且 status 不属于三种不可判定态
    expect(
      isActionableGap(item({ score: 0, weight: 15, status: "missing" })),
    ).toBe(true);
    expect(
      isActionableGap(item({ score: 15, weight: 15, status: "ready" })),
    ).toBe(false);
    expect(
      isActionableGap(item({ score: 0, weight: 15, status: "unmanaged" })),
    ).toBe(false);
    expect(
      isActionableGap(item({ score: 0, weight: 15, status: "unknown" })),
    ).toBe(false);
    // 目标 CLI 不支持的能力后端给满分 + not_required（agent_readiness.rs:85-87）
    expect(
      isActionableGap(item({ score: 15, weight: 15, status: "not_required" })),
    ).toBe(false);
  });
});

describe("classifyReadinessLevel", () => {
  it("没有就绪度数据 → unscanned", () => {
    expect(classifyReadinessLevel(undefined)).toBe("unscanned");
  });

  it("后端判定为 unmanaged → unmanaged，不看分数", () => {
    expect(
      classifyReadinessLevel(
        entry({
          assessmentState: "unmanaged",
          score: 0,
          details: [item({ status: "unmanaged" })],
        }),
      ),
    ).toBe("unmanaged");
  });

  it("P0 回归：已注册但零配置的新项目是 unconfigured，不是 alert", () => {
    // 这正是「刚加进来的项目全红」的场景：8 项全 missing，总分 0
    const details = [
      item({ check_name: "mcp_enabled", weight: 15, score: 0 }),
      item({ check_name: "skills_configured", weight: 12, score: 0 }),
      item({ check_name: "prompt_files", weight: 12, score: 0 }),
      item({ check_name: "commands_configured", weight: 10, score: 0 }),
    ];
    expect(
      classifyReadinessLevel(entry({ score: 0, driftCount: 0, details })),
    ).toBe("unconfigured");
  });

  it("只在磁盘上发现线索、尚未纳管，仍是 unconfigured", () => {
    const details = [
      item({ check_name: "mcp_enabled", score: 6, status: "detected_only" }),
      item({ check_name: "ignore_rules", score: 4, status: "global_only" }),
      item({ check_name: "skills_configured", score: 0, status: "missing" }),
    ];
    expect(classifyReadinessLevel(entry({ score: 10, details }))).toBe(
      "unconfigured",
    );
  });

  it("存在真实漂移 → alert，且优先于其他判定", () => {
    const details = [
      item({ score: 15, status: "ready", effective_state: "drifted" }),
    ];
    expect(
      classifyReadinessLevel(entry({ score: 15, driftCount: 1, details })),
    ).toBe("alert");
  });

  it("已采纳但仍有可行动缺口 → warn", () => {
    const details = [
      item({
        check_name: "mcp_enabled",
        weight: 15,
        score: 15,
        status: "ready",
      }),
      item({
        check_name: "skills_configured",
        weight: 12,
        score: 0,
        status: "missing",
      }),
    ];
    expect(classifyReadinessLevel(entry({ score: 15, details }))).toBe("warn");
  });

  it("全部到位 → ok；分数低于 100 不影响", () => {
    const details = [
      item({
        check_name: "mcp_enabled",
        weight: 15,
        score: 15,
        status: "ready",
      }),
      item({
        check_name: "skills_configured",
        weight: 12,
        score: 12,
        status: "ready",
      }),
      // 目标 CLI 不支持：满分 + not_required，不应拉低等级
      item({
        check_name: "subagents_configured",
        weight: 12,
        score: 12,
        status: "not_required",
      }),
    ];
    expect(classifyReadinessLevel(entry({ score: 39, details }))).toBe("ok");
  });

  it("低分本身不再触发 alert（score 只是采纳度，不是告警阈值）", () => {
    const details = [
      item({
        check_name: "mcp_enabled",
        weight: 15,
        score: 15,
        status: "ready",
      }),
      item({
        check_name: "skills_configured",
        weight: 12,
        score: 0,
        status: "missing",
      }),
    ];
    // 总分 15，远低于 READINESS_WARN_THRESHOLD=50，但没有漂移
    expect(
      classifyReadinessLevel(entry({ score: 15, driftCount: 0, details })),
    ).not.toBe("alert");
  });

  it("所有条目都不可判定 → unmanaged", () => {
    const details = [
      item({ status: "unmanaged", score: 0 }),
      item({ check_name: "skills_configured", status: "unknown", score: 0 }),
    ];
    expect(classifyReadinessLevel(entry({ details }))).toBe("unmanaged");
  });

  it("扫描过但 details 为空 → unscanned", () => {
    expect(classifyReadinessLevel(entry({ details: [] }))).toBe("unscanned");
  });
});
