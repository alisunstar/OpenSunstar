import { describe, expect, it, vi } from "vitest";

import { PortfolioMatrix } from "@/components/kanban/PortfolioMatrix";
import { CommitTrendChart } from "@/components/kanban/CommitTrendChart";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * Rules of Hooks 回归（审查报告 §6.3）。
 *
 * 两个组件都把「提前 return」写在了 Hook 调用**之前**，于是空数据那一帧
 * 少调用了 Hook。数据一到就多出一个 Hook，React 抛
 * 「Rendered more hooks than during the previous render」整棵子树崩掉。
 *
 * 这是真实路径：扫描完成前 weeklyCommits / points 都是空的，扫描完成后填充。
 */

// recharts 在 jsdom 里量不到容器尺寸，真的 ResponsiveContainer 会算出
// width=0 然后什么都不画（`<linearGradient>` 也就不存在）。替身照它的真实职责
// 把尺寸注入子元素，让图形真正渲染出来。
vi.mock("recharts", async () => {
  const actual = await vi.importActual<typeof import("recharts")>("recharts");
  const React = await import("react");
  return {
    ...actual,
    ResponsiveContainer: ({ children }: { children?: React.ReactNode }) => (
      <div data-testid="chart">
        {React.isValidElement(children)
          ? React.cloneElement(
              children as React.ReactElement<{
                width?: number;
                height?: number;
              }>,
              { width: 400, height: 120 },
            )
          : children}
      </div>
    ),
  };
});

describe("CommitTrendChart Hook 顺序", () => {
  function gradientId(container: HTMLElement): string | null {
    return (
      container.querySelector("linearGradient")?.getAttribute("id") ?? null
    );
  }

  /**
   * `useId` 是这个组件唯一的 Hook，所以「空数据帧少调一个 Hook」不会触发 React
   * 的数量校验（上一帧 0 个 Hook → React 走 mount 分支）。它是**潜伏**违规：
   * 今天不崩，加第二个 Hook 就崩。
   *
   * 能确定性抓住它的可观测量是 useId 的稳定性 —— Rules of Hooks 保证同一个
   * 组件实例上的 Hook 不会被重新 mount，违规时每次重入非空分支都会拿到新 id，
   * SVG 的 `fill="url(#...)"` 引用随之漂移。
   */
  it("空↔非空来回切换后 useId 必须稳定（Hook 不得被重新 mount）", () => {
    const { container, rerender } = renderWithProviders(
      <CommitTrendChart weeklyCommits={[4, 5]} projectName="alpha" />,
    );
    const first = gradientId(container);
    expect(first).toBeTruthy();

    rerender(<CommitTrendChart weeklyCommits={[]} projectName="alpha" />);
    rerender(<CommitTrendChart weeklyCommits={[4, 5]} projectName="alpha" />);

    expect(gradientId(container)).toBe(first);
  });

  it("空数据时仍渲染占位而不是崩溃", () => {
    const { rerender } = renderWithProviders(
      <CommitTrendChart weeklyCommits={[]} projectName="alpha" />,
    );

    expect(() =>
      rerender(
        <CommitTrendChart weeklyCommits={[1, 2, 3]} projectName="alpha" />,
      ),
    ).not.toThrow();
  });
});

describe("PortfolioMatrix Hook 顺序", () => {
  const POINT = {
    projectId: "p1",
    name: "alpha",
    stage: "mvp" as const,
    activity: 5,
    score: 80,
    codeLines: 1200,
  };

  it("从空数据切到有数据不得因 Hook 顺序变化崩溃", () => {
    const { rerender } = renderWithProviders(
      <PortfolioMatrix points={[]} scoreKind="aiHealth" />,
    );

    expect(() =>
      rerender(<PortfolioMatrix points={[POINT]} scoreKind="aiHealth" />),
    ).not.toThrow();
  });

  it("从有数据切回空数据同样不得崩溃", () => {
    const { rerender } = renderWithProviders(
      <PortfolioMatrix points={[POINT]} scoreKind="aiHealth" />,
    );

    expect(() =>
      rerender(<PortfolioMatrix points={[]} scoreKind="aiHealth" />),
    ).not.toThrow();
  });
});
