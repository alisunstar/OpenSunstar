import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QuickStartPage } from "./QuickStartPage";

const listRecoverableQuickStartOperations = vi.fn();
const listRecentQuickStartOperations = vi.fn();
const rollbackQuickStartOperation = vi.fn();
const runQuickStartApplyPipeline = vi.fn();
const getQuickStartOperationEvents = vi.fn();

vi.mock("@/lib/quickStart", () => ({
  getCuratedPresetGroups: (appId: string) => [
    {
      category: "official",
      presets: [
        appId === "grokbuild"
          ? {
              name: "Grok Official",
              websiteUrl: "https://x.ai/grok",
              category: "official",
              icon: "grok",
              isOfficial: true,
              raw: {},
            }
          : {
              name: "Claude Official",
              websiteUrl: "https://claude.ai",
              category: "official",
              icon: "claude",
              isOfficial: true,
              raw: {},
            },
      ],
    },
    {
      category: "ai_china",
      presets: [
        {
          name: "DeepSeek",
          websiteUrl: "https://platform.deepseek.com",
          category: "ai_china",
          icon: "deepseek",
          isOfficial: false,
          raw: {},
        },
      ],
    },
    {
      category: "custom",
      presets: [
        {
          name: "__quickstart_custom__",
          category: "custom",
          icon: "",
          isOfficial: false,
          raw: {},
        },
      ],
      isCustomGroup: true,
    },
  ],
  defaultAdvancedFields: () => ({}),
  buildQuickStartProviderInput: () => ({
    name: "DeepSeek",
    settingsConfig: {},
  }),
  createQuickStartAttemptIdentity: () => ({
    providerId: "provider-id",
    idempotencyKey: "request-id",
  }),
  listRecoverableQuickStartOperations: (...args: unknown[]) =>
    listRecoverableQuickStartOperations(...args),
  listRecentQuickStartOperations: (...args: unknown[]) =>
    listRecentQuickStartOperations(...args),
  getQuickStartOperationEvents: (...args: unknown[]) =>
    getQuickStartOperationEvents(...args),
  rollbackQuickStartOperation: (...args: unknown[]) =>
    rollbackQuickStartOperation(...args),
  runQuickStartApplyPipeline: (...args: unknown[]) =>
    runQuickStartApplyPipeline(...args),
}));

vi.mock("./QuickStartVerifyBlock", () => ({
  QuickStartVerifyBlock: ({
    onVerificationChange,
  }: {
    onVerificationChange: (ok: boolean) => void;
  }) => (
    <button type="button" onClick={() => onVerificationChange(true)}>
      验证 Key
    </button>
  ),
}));

vi.mock("./QuickStartAdvancedPanel", () => ({
  QuickStartAdvancedPanel: () => <div>高级选项</div>,
}));

vi.mock("@/components/providers/forms/CodexOAuthSection", () => ({
  CodexOAuthSection: () => <div>Codex 设备授权流程</div>,
}));

vi.mock("@/components/providers/forms/XaiOAuthSection", () => ({
  XaiOAuthSection: () => <div>xAI 设备授权流程</div>,
}));

vi.mock("./QuickStartProviderList", () => ({
  QuickStartProviderList: () => <div>API Key 接入</div>,
}));

function renderPage(
  onOpenAuthSettings?: Parameters<
    typeof QuickStartPage
  >[0]["onOpenAuthSettings"],
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <QuickStartPage onOpenAuthSettings={onOpenAuthSettings} />
    </QueryClientProvider>,
  );
}

describe("QuickStartPage single-page provider workbench", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listRecoverableQuickStartOperations.mockResolvedValue([]);
    listRecentQuickStartOperations.mockResolvedValue([]);
    getQuickStartOperationEvents.mockResolvedValue([]);
    runQuickStartApplyPipeline.mockResolvedValue({
      operation: {
        id: "operation-id",
        appType: "claude",
        status: "succeeded",
        revision: 3,
      },
    });
  });

  it("uses one workbench with an application scope switcher, not page tabs or a three-step rail", async () => {
    renderPage();

    expect(
      await screen.findByRole("heading", { name: "模型接入" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Claude Code/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /OpenCode/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /OpenClaw/ }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Hermes/ })).toBeInTheDocument();
    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
    expect(screen.queryByTestId("quick-start-stepper")).not.toBeInTheDocument();
  });

  it("keeps the default workbench focused on the matching subscription account", async () => {
    renderPage();

    expect(
      await screen.findByRole("heading", { name: /Claude Official/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("凭据由 Claude Code 管理；OpenSunstar 读取状态和额度。"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /DeepSeek/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /添加供应商/ }),
    ).toBeInTheDocument();
    expect(screen.queryByText("代理与故障设置")).not.toBeInTheDocument();
  });

  it("opens the curated supplier library only from the add provider entry", async () => {
    renderPage();

    fireEvent.click(screen.getByRole("button", { name: /添加供应商/ }));

    expect(
      await screen.findByRole("dialog", { name: /添加供应商/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /DeepSeek/ }),
    ).toBeInTheDocument();
  });

  it("routes account actions to the exact authentication section", async () => {
    const onOpenAuthSettings = vi.fn();
    renderPage(onOpenAuthSettings);

    fireEvent.click(await screen.findByRole("button", { name: "登录状态" }));
    expect(onOpenAuthSettings).toHaveBeenLastCalledWith({
      tab: "auth",
      targetId: "local-cli-auth",
    });

    fireEvent.click(screen.getByRole("button", { name: /Codex/ }));
    fireEvent.click(
      await screen.findByRole("button", { name: "ChatGPT 订阅登录" }),
    );
    expect(
      await screen.findByRole("dialog", { name: "ChatGPT 订阅登录" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", { name: "ChatGPT 订阅登录" }),
      ).not.toBeInTheDocument();
    });
    fireEvent.click(await screen.findByRole("button", { name: "账号管理" }));
    expect(onOpenAuthSettings).toHaveBeenLastCalledWith({
      tab: "auth",
      targetId: "codex-oauth",
    });

    fireEvent.click(screen.getByRole("button", { name: /Grok Build/ }));
    expect(
      await screen.findByRole("heading", { name: /Grok Official/ }),
    ).toBeInTheDocument();
    fireEvent.click(
      await screen.findByRole("button", { name: "Grok 订阅登录" }),
    );
    expect(
      await screen.findByRole("dialog", { name: "Grok 订阅登录" }),
    ).toBeInTheDocument();
    expect(screen.getByText("xAI 设备授权流程")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", { name: "Grok 订阅登录" }),
      ).not.toBeInTheDocument();
    });
    fireEvent.click(await screen.findByRole("button", { name: "账号管理" }));
    expect(onOpenAuthSettings).toHaveBeenLastCalledWith({
      tab: "auth",
      targetId: "xai-oauth",
    });
  });

  it("requires a successful connectivity check before enabling a curated provider", async () => {
    getQuickStartOperationEvents.mockResolvedValue([
      {
        sequence: 7,
        eventType: "upstream_verification_succeeded",
        step: "upstream_verified",
        detailJson: JSON.stringify({
          protocol: "openai",
          endpointHost: "api.example.test",
          modelCount: 2,
          providerFingerprint: "sha256:receipt",
        }),
      },
    ]);
    renderPage();

    fireEvent.click(screen.getByRole("button", { name: /添加供应商/ }));
    fireEvent.click(await screen.findByRole("button", { name: /DeepSeek/ }));
    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "sk-test" },
    });
    const apply = screen.getByRole("button", { name: "连接并启用" });
    expect(apply).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "验证 Key" }));
    expect(apply).toBeEnabled();
    fireEvent.click(apply);

    await waitFor(() => {
      expect(runQuickStartApplyPipeline).toHaveBeenCalledOnce();
    });
    expect(screen.getByText("DeepSeek 已连接")).toBeInTheDocument();
    expect(
      screen.queryByTestId("quick-start-result-page"),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "查看审计记录" }));
    await waitFor(() => {
      expect(getQuickStartOperationEvents).toHaveBeenCalledWith("operation-id");
    });
    expect(screen.getByTestId("quick-start-audit-events")).toBeInTheDocument();
    expect(
      screen.getByText(/上游验证：openai · api\.example\.test · 2 个模型/),
    ).toBeInTheDocument();
  });

  it("offers an executable recovery action for an interrupted operation", async () => {
    listRecoverableQuickStartOperations.mockResolvedValue([
      {
        id: "recoverable-operation",
        appType: "codex",
        status: "applying",
        revision: 4,
      },
    ]);
    rollbackQuickStartOperation.mockResolvedValue({
      id: "recoverable-operation",
      appType: "codex",
      status: "rolled_back",
      revision: 5,
    });
    renderPage();

    const recovery = await screen.findByRole("button", {
      name: "恢复未完成操作",
    });
    fireEvent.click(recovery);

    await waitFor(() => {
      expect(rollbackQuickStartOperation).toHaveBeenCalledWith(
        expect.objectContaining({ id: "recoverable-operation" }),
      );
    });
  });

  it("keeps completed operations discoverable after a page restart", async () => {
    listRecentQuickStartOperations.mockResolvedValue([
      {
        id: "completed-operation",
        appType: "claude",
        providerId: "provider-id",
        status: "succeeded",
        revision: 8,
      },
    ]);
    renderPage();

    expect(
      await screen.findByTestId("quick-start-operation-history"),
    ).toBeInTheDocument();
    expect(screen.getByText("completed-operation")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "查看审计记录" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "撤销本次接入" }),
    ).toBeInTheDocument();
  });
});
