import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import {
  ProjectWikiViewer,
  resolveWikiLinkTarget,
} from "@/components/projects/ProjectWikiViewer";
import type { WikiDocument } from "@/types/projectWiki";
import { renderWithProviders } from "../../../tests/renderWithProviders";

const wikiDocument: WikiDocument = {
  candidateId: null,
  pages: [
    {
      path: "index.md",
      title: "项目 Wiki",
      pageType: "overview",
      status: "active",
      sourceFiles: [],
      content: [
        "# 项目 Wiki",
        "",
        "- [项目概览](overview.md)",
        "- [组件目录](components/)",
      ].join("\n"),
    },
    {
      path: "components/architecture.md",
      title: "系统架构",
      pageType: "component",
      status: "active",
      sourceFiles: ["src/main.ts"],
      content: "# 系统架构\n\n[返回概览](../overview.md)",
    },
    {
      path: "overview.md",
      title: "项目概览",
      pageType: "overview",
      status: "active",
      sourceFiles: ["README.md"],
      content: "# 项目概览\n\n正文",
    },
  ],
};

describe("ProjectWikiViewer 页面导航", () => {
  it("点击 index.md 的 Markdown 链接进入对应 Wiki 页面", async () => {
    renderWithProviders(
      <ProjectWikiViewer
        open
        title="项目 Wiki"
        document={wikiDocument}
        loading={false}
        error={null}
        onOpenChange={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole("link", { name: "项目概览" }));

    expect(
      screen.getByText("overview.md · overview · active"),
    ).toBeInTheDocument();
    expect(screen.getByText("README.md")).toBeInTheDocument();
  });

  it("目录链接进入目录中的首个 Wiki 页面", async () => {
    renderWithProviders(
      <ProjectWikiViewer
        open
        title="项目 Wiki"
        document={wikiDocument}
        loading={false}
        error={null}
        onOpenChange={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole("link", { name: "组件目录" }));

    expect(
      screen.getByText("components/architecture.md · component · active"),
    ).toBeInTheDocument();
  });

  it("从子目录页面解析 ../ 相对链接", () => {
    expect(
      resolveWikiLinkTarget("components/architecture.md", "../overview.md", [
        "index.md",
        "overview.md",
        "components/architecture.md",
      ]),
    ).toEqual({ path: "overview.md", anchor: undefined });
  });
});
