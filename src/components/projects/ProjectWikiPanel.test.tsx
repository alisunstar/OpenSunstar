import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { ProjectWikiPanel } from "@/components/projects/ProjectWikiPanel";
import type {
  WikiCandidate,
  WikiDocument,
  WikiScanResult,
} from "@/types/projectWiki";
import { renderWithProviders } from "../../../tests/renderWithProviders";

const mocks = vi.hoisted(() => ({
  scanData: null as WikiScanResult | null,
  aiConfigured: true,
  aiConfigLoading: false,
  generate: vi.fn(),
  accept: vi.fn(),
  refreshConfig: vi.fn(),
  candidates: [] as WikiCandidate[],
  document: null as WikiDocument | null,
  openDocument: vi.fn(),
  openWikiFolder: vi.fn(),
}));

vi.mock("@/lib/api/projectWiki", () => ({
  projectWikiApi: {
    openFolder: mocks.openWikiFolder,
  },
}));

vi.mock("@/hooks/useAIConfig", () => ({
  useAIConfig: () => ({
    aiConfigured: mocks.aiConfigured,
    aiConfigLoading: mocks.aiConfigLoading,
    refreshConfig: mocks.refreshConfig,
  }),
}));

vi.mock("@/hooks/useProjectWiki", () => ({
  useProjectWikiScan: () => ({
    data: mocks.scanData,
    loading: false,
    refresh: vi.fn(),
  }),
  useProjectWikiInit: () => ({
    plan: null,
    preview: vi.fn(),
    confirm: vi.fn(),
    installing: false,
  }),
  useProjectWikiLint: () => ({ result: null, loading: false, lint: vi.fn() }),
  useProjectWikiChangedFiles: () => ({
    data: null,
    loading: false,
    refresh: vi.fn(),
  }),
  useProjectWikiAcceptance: () => ({ loading: false, accept: mocks.accept }),
  useProjectWikiCandidates: () => ({
    data: mocks.candidates,
    loading: false,
    importingId: null,
    refresh: vi.fn(),
    importCandidate: vi.fn(),
  }),
  useProjectWikiComparison: () => ({
    data: null,
    loading: false,
    compare: vi.fn(),
  }),
  useProjectWikiGenerator: () => ({
    loading: false,
    generate: mocks.generate,
  }),
  useProjectWikiDocument: () => ({
    data: mocks.document,
    loading: false,
    error: null,
    open: mocks.openDocument,
    close: vi.fn(),
  }),
}));

function scan(phase: WikiScanResult["lifecycle"]["phase"]): WikiScanResult {
  return {
    projectId: "project-1",
    wikiRoot: "E:/projects/demo/wiki",
    exists: phase !== "uninitialized" && phase !== "pendingGeneration",
    baseStatus: "missing",
    qualityLevel: "N/A",
    pageCount: 0,
    corePageCoverage: {
      hasIndex: false,
      hasOverview: false,
      hasSourceMap: false,
      hasLog: false,
      hasSchema: phase !== "uninitialized",
      componentPages: 0,
      flowPages: 0,
      apiPages: 0,
      runbookPages: 0,
    },
    sourceRefCount: 0,
    questionCount: 0,
    latestMtime: null,
    contentSha256: null,
    lastLintPassed: null,
    lastLintAt: null,
    sourceBaseline: {
      hasGitCommit: true,
      snapshotSha256: null,
      snapshotFileCount: null,
      snapshotRecordedAt: null,
    },
    lifecycle: {
      phase,
      baselineCommit: null,
      baselineContentSha256: null,
      engine: null,
      updatedAt: 1,
      lastError: null,
    },
    checkedAt: 1,
  };
}

describe("ProjectWikiPanel 内置生成闭环", () => {
  beforeEach(() => {
    mocks.aiConfigured = true;
    mocks.aiConfigLoading = false;
    mocks.generate.mockReset();
    mocks.generate.mockResolvedValue({ candidate: { pageCount: 8 } });
    mocks.accept.mockReset();
    mocks.accept.mockResolvedValue(null);
    mocks.refreshConfig.mockReset();
    mocks.refreshConfig.mockResolvedValue(undefined);
    mocks.candidates = [];
    mocks.document = null;
    mocks.openDocument.mockReset();
    mocks.openDocument.mockResolvedValue(null);
    mocks.openWikiFolder.mockReset();
    mocks.openWikiFolder.mockResolvedValue(undefined);
  });

  it("初始化只创建 Schema 后直接引导生成，不再次显示初始化按钮", () => {
    mocks.scanData = scan("pendingGeneration");
    renderWithProviders(<ProjectWikiPanel projectId="project-1" />);

    expect(
      screen.getByRole("button", { name: "生成项目 Wiki" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "初始化 Wiki" }),
    ).not.toBeInTheDocument();
    // P1-b 引擎选择器（a7dd171）：pendingGeneration 阶段允许切换引擎，
    // 但默认必须停在内置引擎，不主动把用户引向外部 openwiki。
    // 旧断言 queryByText(/OpenWiki/i) 会误匹配选择器里的 openwiki 选项。
    expect(screen.getByRole("combobox", { name: "生成引擎" })).toHaveValue(
      "builtin",
    );
  });

  it("未配置 Provider 时给出设置入口，不诱导安装 CLI", async () => {
    mocks.scanData = scan("pendingGeneration");
    mocks.aiConfigured = false;
    const onOpenAiProviderSettings = vi.fn();
    renderWithProviders(
      <ProjectWikiPanel
        projectId="project-1"
        onOpenAiProviderSettings={onOpenAiProviderSettings}
      />,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "配置 AI 提供方" }),
    );
    expect(onOpenAiProviderSettings).toHaveBeenCalledOnce();
    expect(screen.queryByText(/CLI/)).not.toBeInTheDocument();
  });

  it("确认后使用 builtin 引擎，并明确费用、源码发送和待验收边界", async () => {
    mocks.scanData = scan("pendingGeneration");
    renderWithProviders(<ProjectWikiPanel projectId="project-1" />);

    await userEvent.click(
      screen.getByRole("button", { name: "生成项目 Wiki" }),
    );
    expect(screen.getByText(/设置.*AI 提供方/)).toBeInTheDocument();
    expect(screen.getByText(/源码片段.*发送/)).toBeInTheDocument();
    expect(screen.getAllByText(/待验收/).length).toBeGreaterThan(0);

    await userEvent.click(screen.getByRole("button", { name: "确认生成" }));
    expect(mocks.generate).toHaveBeenCalledWith("builtin");
  });

  it("正式 Wiki 区分容器预览和本机目录查看，待验收时明确先预览再验收", async () => {
    mocks.scanData = scan("pendingAcceptance");
    renderWithProviders(<ProjectWikiPanel projectId="project-1" />);

    expect(
      screen.getAllByRole("button", { name: "预览 Wiki" }).length,
    ).toBeGreaterThan(0);
    await userEvent.click(
      screen.getAllByRole("button", { name: "查看 Wiki" })[0],
    );
    expect(mocks.openWikiFolder).toHaveBeenCalledWith("project-1");
    expect(screen.getByText(/先查看生成内容/)).toBeInTheDocument();
    expect(
      screen.getAllByRole("button", { name: "验收并建立基线" }).length,
    ).toBeGreaterThan(0);
  });

  it("无 Git 基线时先引导记录源码快照，再执行验收", async () => {
    mocks.scanData = {
      ...scan("pendingAcceptance"),
      sourceBaseline: {
        hasGitCommit: false,
        snapshotSha256: null,
        snapshotFileCount: null,
        snapshotRecordedAt: null,
      },
    };
    mocks.accept.mockResolvedValue({ phase: "syncedToSnapshot" });
    renderWithProviders(<ProjectWikiPanel projectId="project-1" />);

    await userEvent.click(
      screen.getAllByRole("button", { name: "验收并建立基线" })[0],
    );
    expect(screen.getByText("记录源码快照基线")).toBeInTheDocument();

    await userEvent.click(
      screen.getByRole("button", { name: "记录快照并验收" }),
    );
    expect(mocks.accept).toHaveBeenCalledOnce();
  });

  it("候选版本可以在导入前预览正文", () => {
    mocks.scanData = scan("changesDetected");
    mocks.candidates = [
      {
        id: "builtin-1",
        engine: "builtin",
        createdAt: 1,
        pageCount: 8,
        hasIndex: true,
        path: "E:/demo/candidate",
        sourceCommit: "abc123",
        model: "deepseek-chat",
        generationSeconds: 12,
      },
    ];
    renderWithProviders(<ProjectWikiPanel projectId="project-1" />);

    expect(
      screen.getByRole("button", { name: "预览候选" }),
    ).toBeInTheDocument();
  });

  it("同步更新使用同步专属确认语义", async () => {
    mocks.scanData = scan("changesDetected");
    renderWithProviders(<ProjectWikiPanel projectId="project-1" />);

    await userEvent.click(
      screen.getByRole("button", { name: "同步更新 Wiki" }),
    );
    expect(screen.getByText(/基于当前 Git HEAD 重新生成/)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "确认同步" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "确认生成" }),
    ).not.toBeInTheDocument();
  });
});
