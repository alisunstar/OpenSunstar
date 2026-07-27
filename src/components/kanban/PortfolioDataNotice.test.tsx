import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { PortfolioDataNotice } from "@/components/kanban/PortfolioDataNotice";
import { renderWithProviders } from "../../../tests/renderWithProviders";

/**
 * 组合层数据不完整时的告警条（审查报告 §5.6 的 UI 落点）。
 *
 * 两个 hook 现在会上报失败，但如果没人渲染它，「满屏未扫」照旧会被当成
 * 治理结论。这个条子存在的唯一理由是：让「读不到」和「真的没配」在
 * 视觉上分开。
 */

describe("PortfolioDataNotice", () => {
  it("数据完整时不渲染任何东西（不能变成常驻噪音）", () => {
    const { container } = renderWithProviders(
      <PortfolioDataNotice
        assetError={null}
        readinessFailedCount={0}
        totalProjects={5}
        onRetry={() => {}}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("资产扫描失败要说明「未扫不代表没配置」并带上原始原因", () => {
    renderWithProviders(
      <PortfolioDataNotice
        assetError="ipc timeout"
        readinessFailedCount={0}
        totalProjects={5}
        onRetry={() => {}}
      />,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText(/不代表/)).toBeInTheDocument();
    // 原始原因必须可见：429 / 401 / 超时不能被折叠成同一句话
    expect(screen.getByText(/ipc timeout/)).toBeInTheDocument();
  });

  it("就绪度部分失败要报出失败项目数", () => {
    renderWithProviders(
      <PortfolioDataNotice
        assetError={null}
        readinessFailedCount={3}
        totalProjects={5}
        onRetry={() => {}}
      />,
    );

    expect(screen.getByRole("alert").textContent).toMatch(/3\s*\/\s*5/);
  });

  it("重试按钮回调必须接上", async () => {
    const onRetry = vi.fn();
    renderWithProviders(
      <PortfolioDataNotice
        assetError="ipc timeout"
        readinessFailedCount={0}
        totalProjects={5}
        onRetry={onRetry}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: /重试/ }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("正在刷新时重试按钮禁用，避免叠加请求", () => {
    renderWithProviders(
      <PortfolioDataNotice
        assetError="ipc timeout"
        readinessFailedCount={0}
        totalProjects={5}
        refreshing
        onRetry={() => {}}
      />,
    );

    expect(screen.getByRole("button", { name: /重试/ })).toBeDisabled();
  });
});
