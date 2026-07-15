import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Provider } from "@/types";
import { QuickStartProviderList } from "./QuickStartProviderList";

const switchProvider = vi.fn();
const addProvider = vi.fn();
const updateProvider = vi.fn();
const deleteProvider = vi.fn();
const saveUsageScript = vi.fn();
let providerListProps: Record<string, unknown> = {};

const provider: Provider = {
  id: "deepseek",
  name: "DeepSeek",
  category: "cn_official",
  websiteUrl: "https://platform.deepseek.com",
  settingsConfig: {},
};

const backupProvider: Provider = {
  id: "minimax",
  name: "MiniMax",
  category: "cn_official",
  settingsConfig: {},
};
let actionProvider = provider;

vi.mock("@/lib/query/queries", () => ({
  useProvidersQuery: () => ({
    data: {
      providers: { [provider.id]: provider },
      currentProviderId: provider.id,
    },
  }),
}));

vi.mock("@/hooks/useProxyStatus", () => ({
  useProxyStatus: () => ({
    takeoverStatus: { claude: true },
    isRunning: true,
  }),
}));

vi.mock("@/hooks/useProviderActions", () => ({
  useProviderActions: () => ({
    switchProvider,
    addProvider,
    updateProvider,
    deleteProvider,
    saveUsageScript,
    isLoading: false,
  }),
}));

vi.mock("@/components/providers/ProviderList", () => ({
  ProviderList: (props: Record<string, unknown>) => {
    providerListProps = props;
    return (
      <div>
        <button
          type="button"
          onClick={() =>
            (props.onDuplicate as (p: Provider) => void)(actionProvider)
          }
        >
          复制供应商
        </button>
        <button
          type="button"
          onClick={() =>
            (props.onDelete as (p: Provider) => void)(actionProvider)
          }
        >
          删除供应商
        </button>
        <button
          type="button"
          onClick={() =>
            (props.onConfigureUsage as (p: Provider) => void)(actionProvider)
          }
        >
          配置用量查询
        </button>
      </div>
    );
  },
}));

vi.mock("@/components/providers/EditProviderDialog", () => ({
  EditProviderDialog: () => null,
}));

function renderList() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <QuickStartProviderList appId="claude" onAddProvider={vi.fn()} />
    </QueryClientProvider>,
  );
}

describe("QuickStartProviderList", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    providerListProps = {};
    actionProvider = provider;
    addProvider.mockResolvedValue(undefined);
    deleteProvider.mockResolvedValue(undefined);
    saveUsageScript.mockResolvedValue(undefined);
  });

  it("hands the connected providers to the sortable management list with the active and route status", () => {
    renderList();

    expect(screen.getByText("我的供应商（1）")).toBeInTheDocument();
    expect(providerListProps.providers).toEqual({ [provider.id]: provider });
    expect(providerListProps.currentProviderId).toBe(provider.id);
    expect(providerListProps.isProxyTakeover).toBe(true);
  });

  it("duplicates a provider through the established provider action pipeline", async () => {
    renderList();

    fireEvent.click(screen.getByRole("button", { name: "复制供应商" }));

    await waitFor(() => {
      expect(addProvider).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "DeepSeek 副本",
          settingsConfig: provider.settingsConfig,
        }),
      );
    });
  });

  it("requires confirmation before deleting a non-current supplier", async () => {
    renderList();
    actionProvider = backupProvider;

    fireEvent.click(screen.getByRole("button", { name: "删除供应商" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(deleteProvider).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    await waitFor(() =>
      expect(deleteProvider).toHaveBeenCalledWith(backupProvider.id),
    );
  });

  it("opens a usage configuration and persists it through the existing audit-aware action", async () => {
    renderList();

    fireEvent.click(screen.getByRole("button", { name: "配置用量查询" }));
    fireEvent.change(screen.getByLabelText("查询脚本"), {
      target: { value: "return { success: true, data: [] };" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存用量配置" }));

    await waitFor(() => {
      expect(saveUsageScript).toHaveBeenCalledWith(
        provider,
        expect.objectContaining({
          code: "return { success: true, data: [] };",
        }),
      );
    });
  });
});
