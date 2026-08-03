import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UsageDashboard } from "./UsageDashboard";

const syncSessionUsage = vi.fn();

vi.mock("@/lib/api/usage", () => ({
  usageApi: {
    syncSessionUsage: (...args: unknown[]) => syncSessionUsage(...args),
  },
}));

vi.mock("@/hooks/useUsageEventBridge", () => ({
  useUsageEventBridge: () => undefined,
}));

vi.mock("./UsageHero", () => ({ UsageHero: () => <div>usage-hero</div> }));
vi.mock("./UsageTrendChart", () => ({
  UsageTrendChart: () => <div>usage-trend</div>,
}));
vi.mock("./RequestLogTable", () => ({
  RequestLogTable: () => <div>request-logs</div>,
}));
vi.mock("./ProviderStatsTable", () => ({
  ProviderStatsTable: () => <div>provider-stats</div>,
}));
vi.mock("./ModelStatsTable", () => ({
  ModelStatsTable: () => <div>model-stats</div>,
}));
vi.mock("./PricingConfigPanel", () => ({
  PricingConfigPanel: () => <div>pricing</div>,
}));
vi.mock("./UsageDateRangePicker", () => ({
  UsageDateRangePicker: () => <button type="button">date-range</button>,
}));
vi.mock("./ExportMenu", () => ({ ExportMenu: () => <div>export</div> }));

function renderDashboard() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <UsageDashboard />
    </QueryClientProvider>,
  );
}

describe("UsageDashboard", () => {
  beforeEach(() => {
    syncSessionUsage.mockReset();
    syncSessionUsage.mockResolvedValue({
      imported: 1,
      skipped: 0,
      filesScanned: 1,
      errors: [],
    });
  });

  it("syncs local session usage when the dashboard opens and refreshes", async () => {
    renderDashboard();

    expect(
      screen.getByRole("button", { name: "usage.appFilter.grokbuild" }),
    ).toBeInTheDocument();

    await waitFor(() => expect(syncSessionUsage).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getByTitle("刷新"));

    await waitFor(() => expect(syncSessionUsage).toHaveBeenCalledTimes(2));
  });
});
