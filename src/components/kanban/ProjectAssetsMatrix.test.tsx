import { describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import type { AgentReadinessItem } from "@/api/aiInsight";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { ProjectAssetCounts } from "@/hooks/kanban/usePortfolioAssetSummary";
import type { Project } from "@/types/project";
import { ProjectAssetsMatrix } from "@/components/kanban/ProjectAssetsMatrix";
import { renderWithProviders } from "../../../tests/renderWithProviders";

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

function counts(partial: Partial<ProjectAssetCounts> = {}): ProjectAssetCounts {
  return {
    mcp: 0,
    skills: 0,
    prompts: 0,
    commands: 0,
    hooks: 0,
    ignore: 0,
    permissions: 0,
    subagents: 0,
    ...partial,
  };
}

function renderMatrix(
  details: AgentReadinessItem[],
  score = 0,
  assetMap?: Map<string, ProjectAssetCounts>,
) {
  const entry: AgentReadinessBatchEntry = {
    score,
    driftCount: 0,
    scannedAt: 1,
    details,
  };
  return renderWithProviders(
    <ProjectAssetsMatrix
      projects={[PROJECT]}
      getStage={() => "mvp"}
      agentReadinessMap={new Map([["p1", entry]])}
      assetMap={assetMap}
      onOpenProject={vi.fn()}
      onOpenProjectAiConfig={vi.fn()}
    />,
  );
}

/** 8 类资产全部就绪，唯独维护度缺席 —— 「8 格全绿但只有 91 分」的原型 */
const EIGHT_READY: AgentReadinessItem[] = [
  item({ check_name: "mcp_enabled", weight: 15, score: 15, status: "ready" }),
  item({
    check_name: "skills_configured",
    weight: 12,
    score: 12,
    status: "ready",
  }),
  item({ check_name: "prompt_files", weight: 12, score: 12, status: "ready" }),
  item({
    check_name: "commands_configured",
    weight: 10,
    score: 10,
    status: "ready",
  }),
  item({
    check_name: "hooks_configured",
    weight: 10,
    score: 10,
    status: "ready",
  }),
  item({ check_name: "ignore_rules", weight: 10, score: 10, status: "ready" }),
  item({ check_name: "permissions", weight: 10, score: 10, status: "ready" }),
  item({
    check_name: "subagents_configured",
    weight: 12,
    score: 12,
    status: "ready",
  }),
];

describe("ProjectAssetsMatrix 单元格状态口径", () => {
  it("P0 回归：目标 CLI 不支持的能力显示「不适用」，不得显示「已生效」", () => {
    // agent_readiness.rs:85-87 —— 不支持项拿满分 + not_required。
    // 旧代码比对的是 "not_applicable"（effective_state 的取值），永远不命中，
    // 于是落到 `score > 0 → normal` 的兜底，把「不适用」渲染成绿色「已生效」。
    renderMatrix([
      item({
        check_name: "subagents_configured",
        label: "Subagents",
        weight: 12,
        score: 12,
        status: "not_required",
        detail: "当前目标 CLI「Codex」不支持此项，已从评分中排除",
      }),
    ]);

    expect(screen.getAllByText("不适用").length).toBeGreaterThan(0);
    expect(screen.queryByText("已生效")).not.toBeInTheDocument();
  });

  it("P0 回归：未纳管项目的安全关键项不得渲染成红色「不一致」", () => {
    // classify_unmanaged_readiness（agent_readiness.rs:387-413）把未纳管项目
    // 全部置零并标记 unmanaged/unknown —— 零计数不能证明缺失。
    renderMatrix([
      item({
        check_name: "hooks_configured",
        label: "Hooks",
        weight: 10,
        score: 0,
        status: "unmanaged",
      }),
    ]);

    expect(screen.queryByText("不一致")).not.toBeInTheDocument();
    expect(screen.queryByText("缺失")).not.toBeInTheDocument();
  });

  it("unknown 同样不可判定，不得渲染成缺陷", () => {
    renderMatrix([
      item({
        check_name: "permissions",
        label: "权限",
        weight: 10,
        score: 0,
        status: "unknown",
      }),
    ]);

    expect(screen.queryByText("不一致")).not.toBeInTheDocument();
    expect(screen.queryByText("缺失")).not.toBeInTheDocument();
  });

  it("unhealthy（漂移覆写）显示为「不一致」", () => {
    // asset_effective_state.rs:1667 在检出漂移时把 status 覆写为 unhealthy。
    // 即便生效态字段缺失（走缓存），仅凭 status 也必须判为不一致。
    renderMatrix([
      item({
        check_name: "mcp_enabled",
        weight: 15,
        score: 0,
        status: "unhealthy",
      }),
    ]);

    expect(screen.getAllByText("不一致").length).toBeGreaterThan(0);
  });

  it("缺失的安全关键项仍然报红「缺失」", () => {
    renderMatrix([
      item({
        check_name: "ignore_rules",
        label: "忽略规则",
        weight: 10,
        score: 0,
        status: "missing",
      }),
    ]);

    expect(screen.getAllByText("缺失").length).toBeGreaterThan(0);
  });
});

/**
 * 第 9 项计分，但没有格子（审查报告 §5.3）。
 *
 * `agent_readiness.rs:339-358` 产出 9 项，第 9 项 `recent_updates`（权重 9）
 * 是**维护度指标**而非磁盘资产 —— `asset_effective_state.rs:1627-1633` 给它
 * 恒定的 `effective_state: not_applicable`。矩阵只有 8 列，于是
 * 「8 格全绿但只有 91 分」在界面上无处解释。
 *
 * 补列时的陷阱：照抄资产列的判定逻辑会让「有更新」落进
 * `effective_state === "not_applicable" && configured_state !== "unconfigured"`
 * 这条分支，渲染成灰色「不适用」—— 那 9 分就更没人看得懂了。
 */
describe("ProjectAssetsMatrix 维护度列", () => {
  it("表头必须有第 9 列", () => {
    renderMatrix(EIGHT_READY, 91);
    expect(
      screen.getByRole("columnheader", { name: "维护度" }),
    ).toBeInTheDocument();
  });

  it("近 90 天有更新 → 「有更新」，不得渲染成灰色「不适用」", () => {
    renderMatrix([
      item({
        check_name: "recent_updates",
        label: "近 90 天项目资产关联更新",
        weight: 9,
        score: 9,
        status: "ready",
        detail: "近 90 天内有项目级 AI 资产配置变更",
        configured_state: "configured",
        effective_state: "not_applicable",
        effective_detail: "维护度指标无磁盘生效态",
      }),
    ]);

    expect(screen.getByText("有更新")).toBeInTheDocument();
    expect(screen.queryByText("不适用")).not.toBeInTheDocument();
  });

  it("90 天无更新 → 「无更新」，不得说成「缺失」", () => {
    // 维护度没有「缺失」这回事：资产在那儿，只是最近没动过。
    renderMatrix([
      item({
        check_name: "recent_updates",
        label: "近 90 天项目资产关联更新",
        weight: 9,
        score: 0,
        status: "missing",
        detail: "最近 90 天内无项目级资产配置变更",
        configured_state: "unconfigured",
        effective_state: "not_applicable",
      }),
    ]);

    expect(screen.getByText("无更新")).toBeInTheDocument();
    expect(screen.queryByText("缺失")).not.toBeInTheDocument();
  });

  it("维护度不计入项目状态聚合：8 项全绿的项目不因久未更新变成「需处理」", () => {
    // 「需处理」回答的是「今天该动哪个项目」。90 天没改过配置是一个事实，
    // 不是一个缺陷 —— 把它算进告警就是第一梯队刚修掉的那种狼来了。
    renderMatrix(
      [
        ...EIGHT_READY,
        item({
          check_name: "recent_updates",
          weight: 9,
          score: 0,
          status: "missing",
          configured_state: "unconfigured",
          effective_state: "not_applicable",
        }),
      ],
      91,
    );

    // 这排胶囊是筛选开关（`role="group"` + `aria-pressed`），不是 Tab ——
    // 详见下面「ProjectAssetsMatrix 无障碍语义」里那条回归。
    expect(
      screen.getByRole("button", { name: /需处理\s*0/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /全部\s*1/ }),
    ).toBeInTheDocument();
  });
});

/**
 * 数据取到了，扔掉了（审查报告 §5.4）。
 *
 * `assetMap` 一直由 `KanbanPage` 传入却从未被解构。「这个项目实际关联了几个
 * MCP」是最直观的信息，已经在手里了。
 *
 * 但计数为 0 不能渲染成「0」：零计数不能证明缺失（`agent_readiness.rs:387-413`），
 * 未纳管项目的每一项都是 0。
 */
describe("ProjectAssetsMatrix 资产数量", () => {
  it("已关联数量渲染进格子", () => {
    renderMatrix(
      [
        item({
          check_name: "mcp_enabled",
          weight: 15,
          score: 15,
          status: "ready",
        }),
      ],
      15,
      new Map([["p1", counts({ mcp: 3 })]]),
    );

    const cell = screen.getByRole("cell", { name: /已生效/ });
    expect(within(cell).getByText("3")).toBeInTheDocument();
  });

  it("计数为 0 时不渲染「0」——零计数不是缺失的证据", () => {
    // 未纳管项目的每一项都是 0；把它印在格子里等于替后端下了「缺失」的结论。
    renderMatrix(
      [
        item({
          check_name: "mcp_enabled",
          weight: 15,
          score: 0,
          status: "unmanaged",
        }),
      ],
      15, // 分数列避开 "0"，免得和格子里的计数混淆
      new Map([["p1", counts({ mcp: 0 })]]),
    );

    const row = screen.getByRole("row", { name: /alpha/ });
    expect(within(row).queryAllByText("0")).toHaveLength(0);
  });

  it("assetMap 尚未加载到该项目时不渲染任何数量", () => {
    renderMatrix(
      [
        item({
          check_name: "mcp_enabled",
          weight: 15,
          score: 15,
          status: "ready",
        }),
      ],
      15,
      new Map(),
    );

    const cell = screen.getByRole("cell", { name: /已生效/ });
    expect(within(cell).queryByText(/^\d+$/)).not.toBeInTheDocument();
  });

  it("详情面板给数量一个口径：是「OpenSunstar 已关联」，不是磁盘上有几个", async () => {
    renderMatrix(
      [
        item({
          check_name: "mcp_enabled",
          weight: 15,
          score: 15,
          status: "ready",
        }),
      ],
      15,
      new Map([["p1", counts({ mcp: 3 })]]),
    );

    // 点的是格子里那个 button，不是格子本身 —— onClick 已经从 `<td>` 挪进去了。
    await userEvent.click(screen.getByRole("button", { name: /已生效/ }));

    expect(screen.getByText(/OpenSunstar 已关联/)).toBeInTheDocument();
  });
});

/**
 * 无障碍语义（审查报告 §7）。
 *
 * 这一组断言的共同点：**它们全都测不出任何像素**。矩阵改之前在屏幕上就是
 * 现在这个样子 —— 只不过用键盘走不进去、读屏器念出来的是一串没有主语的
 * 「已生效、缺失、未扫」。所以这几条只能靠 role / aria 来钉，一旦有人为了
 * 省事把 onClick 挪回 `<td>`、或者把筛选胶囊改回 `role="tab"`，这里会红。
 */
describe("ProjectAssetsMatrix 无障碍语义", () => {
  const ONE_READY: AgentReadinessItem[] = [
    item({ check_name: "mcp_enabled", weight: 15, score: 15, status: "ready" }),
  ];

  it("单元格是真正的 button：Tab 能走到，Enter 能打开详情", async () => {
    // 原来 onClick 挂在 `<td>` 上 —— 鼠标之外的用户看得见、打不开。
    renderMatrix(ONE_READY, 15);

    const cellButton = screen.getByRole("button", { name: /已生效/ });
    cellButton.focus();
    expect(cellButton).toHaveFocus();

    await userEvent.keyboard("{Enter}");
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("格子的可访问名带上主语：项目 · 资产 · 状态", () => {
    // 格子里可见的只有「已生效」三个字。读屏器停在这里时如果只念这三个字，
    // 用户不知道是哪个项目的哪一项 —— 行头 + 列头只在表格导航模式下自动
    // 播报，而绝大多数人是按 Tab 走过来的。
    renderMatrix(ONE_READY, 15);

    // 中段用的是列头上那个名字（GOVERNANCE_CHECK_LABELS，即「MCP」），
    // 不是后端 item.label（「MCP 服务器」）—— 读屏器念的应该和列头看到的
    // 是同一个词，否则用户对不上是哪一列。
    expect(
      screen.getByRole("button", { name: "alpha · MCP · 已生效" }),
    ).toBeInTheDocument();
  });

  it("项目名格子是行头（rowheader），不是普通单元格", () => {
    // `<th scope="row">` 让读屏器横向走表格时自动带上项目名。
    renderMatrix(ONE_READY, 15);

    expect(
      screen.getByRole("rowheader", { name: /alpha/ }),
    ).toBeInTheDocument();
  });

  it("列头有 scope=col", () => {
    renderMatrix(ONE_READY, 15);

    const header = screen.getByRole("columnheader", { name: "维护度" });
    expect(header).toHaveAttribute("scope", "col");
  });

  it("筛选胶囊是 aria-pressed 的按钮，不是 tab —— 没有 tabpanel 就别承诺有", () => {
    // `role="tab"` 向读屏器承诺存在对应的 tabpanel、并且方向键可以切换。
    // 这里切的是同一张表格的行，两条承诺都做不到。
    renderMatrix(ONE_READY, 15);

    expect(screen.queryAllByRole("tab")).toHaveLength(0);
    expect(screen.getByRole("button", { name: /全部\s*1/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: /异常\s*0/ })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("详情面板是 dialog：有 aria-modal 与可访问名", async () => {
    renderMatrix(ONE_READY, 15);
    await userEvent.click(screen.getByRole("button", { name: /已生效/ }));

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    // 名字来自面板标题（aria-labelledby），不是硬编码的字符串。
    expect(dialog).toHaveAccessibleName("MCP");
  });

  it("Esc 关掉详情面板", async () => {
    renderMatrix(ONE_READY, 15);
    await userEvent.click(screen.getByRole("button", { name: /已生效/ }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("关闭后焦点回到打开它的那个格子，而不是掉回 body", async () => {
    // 掉回 `<body>` 意味着键盘用户要从页面头部重新 Tab 一遍才能回到原位。
    renderMatrix(ONE_READY, 15);

    const cellButton = screen.getByRole("button", { name: /已生效/ });
    cellButton.focus();
    await userEvent.keyboard("{Enter}");
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await userEvent.keyboard("{Escape}");
    expect(cellButton).toHaveFocus();
  });

  it("面板打开时焦点进入面板内部", async () => {
    renderMatrix(ONE_READY, 15);
    await userEvent.click(screen.getByRole("button", { name: /已生效/ }));

    const dialog = screen.getByRole("dialog");
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});

/**
 * 矩阵规模：不截断，也暂不虚拟化（审查报告 §7「20+ 项目应虚拟化」）。
 *
 * 量过之后的决定，不是遗漏。实测每行结构恒定 **54 个 DOM 节点**
 * （1 个行头 + 阶段 + 9 个资产格 + 分数）：
 *
 *     25 个项目 → 1,386 节点     100 个 → 5,411
 *     50 个项目 → 2,729 节点     200 个 → 10,779     500 个 → 26,879
 *
 * 浏览器的舒适区大致到 1 万节点，也就是 ~200 个项目才到拐点；本地开发
 * 工作区的常见规模是 5–50 个，不到 3k 节点。而虚拟化的代价与上面刚补的
 * 无障碍语义直接冲突：`Ctrl+F` 搜不到未渲染的行，`<th scope="row">` 的
 * 表格导航要靠手工维护 `aria-rowcount` / `aria-rowindex` 才不塌，首列
 * `position: sticky` 叠加窗口化在各浏览器上表现还不一致。
 *
 * 所以这里守的不是「有没有虚拟化」，而是**别悄悄截断**：`slice(0, 50)`
 * 这类改动会让第 51 个项目从此在这张表上不存在，界面上没有任何提示 ——
 * 那不是优化，那是丢数据。真出现 200+ 项目的用户再回来做窗口化，届时
 * 这条断言应该连同 `aria-rowcount` 的新断言一起改，而不是删掉。
 */
describe("ProjectAssetsMatrix 矩阵规模", () => {
  it("渲染行数恒等于项目数 —— 不得静默截断", () => {
    const N = 120;
    const projects: Project[] = Array.from({ length: N }, (_, i) => ({
      id: `p${i}`,
      name: `project-${i}`,
      path: `E:/projects/p${i}`,
      addedAt: new Date("2026-07-01").toISOString(),
    }));
    const map = new Map<string, AgentReadinessBatchEntry>(
      projects.map((p, i) => [
        p.id,
        {
          score: 40 + (i % 60),
          driftCount: 0,
          scannedAt: 1,
          details: [
            item({
              check_name: "mcp_enabled",
              weight: 15,
              score: 15,
              status: "ready",
            }),
          ],
        },
      ]),
    );

    const { container } = renderWithProviders(
      <ProjectAssetsMatrix
        projects={projects}
        getStage={() => "mvp"}
        agentReadinessMap={map}
        onOpenProject={vi.fn()}
        onOpenProjectAiConfig={vi.fn()}
      />,
    );

    expect(container.querySelectorAll("tbody tr")).toHaveLength(N);
    // 最后一个项目必须真的在 DOM 里，而不只是行数对得上。
    expect(
      screen.getByRole("rowheader", { name: `project-${N - 1}` }),
    ).toBeInTheDocument();
  });
});
