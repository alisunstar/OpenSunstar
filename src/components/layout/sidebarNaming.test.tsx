import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";

import { Sidebar } from "@/components/layout/Sidebar";
import zh from "@/i18n/locales/zh.json";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 侧栏命名（审查报告 §2.3 + §2.5）。
 *
 * 报告数出四处「分组标题 = 唯一菜单项」的空嵌套，最露骨的是跨 Agent 配置：
 * `Sidebar.tsx:349` 与 `:365` **用的是字面同一个 i18n key**，于是同一个词在
 * 屏幕上连着印两遍，中间什么也没有。这不是排版瑕疵 —— 分组层存在的意义是
 * 「底下有好几个东西需要归拢」，只有一个孩子时它没有在表达任何信息，只是把
 * 每个真分组之间的视觉间距撑大了一倍，反而让唯一一个真分组（AI模型，3 个
 * 子项）看起来和其它四个「假分组」一样重。
 *
 * 所以这里的断言是结构性的而不是逐字符的：**一个名字在侧栏里只能出现一次**。
 * 出现两次，要么是空嵌套复活了，要么是有人又给分组和菜单起了同一个名 ——
 * 两种都是同一个病。
 *
 * 另一半断言针对一个更阴的失败模式：`t(key, { defaultValue })` 里的兜底文案
 * 与 `zh.json` 里的真资源可以各写各的。测试环境的 i18n 资源是空的
 * （`tests/setupTests.ts`），渲染时拿到的**永远是 defaultValue**；而用户在
 * 真实应用里看到的**永远是 zh.json**。两者漂移时，测试全绿，界面是另一套
 * 名字 —— 本次改名前 `methodology.sidebar` 就正处在这个状态：defaultValue
 * 写着「工作流与治理」，`zh.json:3754` 写着「项目治理」。因此下面对 zh.json
 * 直接断言，把两边钉在一起。
 */

const NAMES = {
  /** 回到 `docs/kanban.md` 规格：规格里没有「跨项目」三个字。 */
  workspace: "工作区",
  /** 与 `GovernanceDashboard` 的「治理」解耦，各自说清自己是什么。 */
  methodology: "流程与方法论",
  /**
   * 全局 CRUD 作用域。§2.5 建议改名「资产库」，产品决定留旧名，只去掉
   * 「跨Agent配置」的「跨」字前缀 —— 它和「跨项目工作区」的「跨」一样，
   * 描述的是实现而不是用户的事。
   *
   * 注意不能写成「侧栏里没有任何名字以『跨』开头」：「跨设备云同步」的
   * 「跨」说的是用户真会做的事（在两台机器间同步），该留。所以这里靠下面
   * 对 `zh.json` 的逐键断言把「跨」前缀是否复活钉死，而不是靠一条正则。
   */
  assetLibrary: "Agent 配置",
  /** 项目级关联作用域。同上，§2.5 的「项目落地」未采纳。 */
  projectLanding: "AI 资产总览",
} as const;

function renderSidebar() {
  return renderWithProviders(
    <Sidebar activeView="kanban" onNavigate={vi.fn()} projects={[]} />,
  );
}

describe("侧栏没有「分组标题 = 唯一菜单项」的空嵌套", () => {
  it("P0 回归：一个名字在侧栏里只出现一次 —— 出现两次就是空嵌套", () => {
    renderSidebar();

    for (const [slot, name] of Object.entries(NAMES)) {
      const hits = screen.queryAllByText(name);
      expect(
        hits,
        `「${name}」(${slot}) 出现了 ${hits.length} 次`,
      ).toHaveLength(1);
    }
  });

  it("「跨项目」前缀已从侧栏移除 —— 规格里没有这三个字", () => {
    renderSidebar();

    expect(screen.queryAllByText(/跨项目/)).toHaveLength(0);
  });

  it("每个分组标题下都不止一个菜单项 —— 否则这一层没在表达任何信息", () => {
    renderSidebar();

    // 「AI模型」是唯一保留的分组：快速接入 / Context / AI Tokens 三个子项。
    expect(screen.getByText("AI模型")).toBeInTheDocument();
    for (const child of ["快速接入", "Context", "AI Tokens"]) {
      expect(screen.getByText(child)).toBeInTheDocument();
    }

    // 「跨设备云同步」此前没有分组标题终结「AI模型」那一组，于是它和
    // 「团队协作配置」在视觉上被吞进 AI 模型里（§2.3 附带的真 bug）。
    // 修法不是给团队协作配置单独补一个标题（那又是一个空嵌套），
    // 是给这两个本来就该在一起的条目一个共同的家。
    expect(screen.getByText("同步与协作")).toBeInTheDocument();
    for (const child of ["跨设备云同步", "团队协作配置"]) {
      expect(screen.getByText(child)).toBeInTheDocument();
    }
  });
});

describe("i18n 资源与组件兜底文案必须一致", () => {
  it("P0 回归：zh.json 与 defaultValue 说的是同一个名字", () => {
    expect(zh.workspace.title).toBe(NAMES.workspace);
    expect(zh.methodology.sidebar).toBe(NAMES.methodology);
    expect(zh.methodology.title).toBe(NAMES.methodology);
    expect(zh.sidebar.agentConfig).toBe(NAMES.assetLibrary);
    expect(zh.workspace.tabs.assetsMatrix).toBe(NAMES.projectLanding);
  });

  it("空嵌套用掉的两个 key 已删除 —— 留着就是等人再挂回去", () => {
    // `kanban.subtitle`「多项目组合矩阵…」就是这么活下来的：无人渲染，
    // 却一直躺在 zh.json 里冒充规格（§2.2）。
    expect("section" in zh.workspace.sidebar).toBe(false);
    expect("sidebarSection" in zh.methodology).toBe(false);
  });
});

describe("副标题只说代码里有的东西（§2.4）", () => {
  /**
   * 原文「监控汇总项目风险、进度与 AI 配置状态，帮助你先处理关键问题」逐词
   * 核查后有两个词没有实现：**风险**在工作区层面根本不存在（只有单项目按需的
   * `AIRiskAnalysis`），**进度**只在 `TodayWorkspace.tsx:169-175` 生成一条
   * `stage==="mvp" && progress<50` 的文本，四张 SummaryCard 没有一张是进度。
   * 报告的 B 方案：删掉这两个词，别的先不动。
   */
  it("P0 回归：不再承诺工作区层面没有的「风险」与「进度」", () => {
    expect(zh.workspace.subtitle).not.toContain("风险");
    expect(zh.workspace.subtitle).not.toContain("进度");
  });

  it("副标题仍然说清这个页面是干什么的 —— 删词不等于删空", () => {
    expect(zh.workspace.subtitle.length).toBeGreaterThan(10);
    expect(zh.workspace.subtitle).toContain("配置");
  });
});
