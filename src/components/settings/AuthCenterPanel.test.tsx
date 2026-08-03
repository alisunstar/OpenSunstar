import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AuthCenterPanel } from "./AuthCenterPanel";

vi.mock("@/components/providers/forms/CopilotAuthSection", () => ({
  CopilotAuthSection: () => <div data-testid="copilot-auth-section" />,
}));

vi.mock("@/components/providers/forms/CodexOAuthSection", () => ({
  CodexOAuthSection: () => <div data-testid="codex-oauth-section" />,
}));

vi.mock("@/components/providers/forms/XaiOAuthSection", () => ({
  XaiOAuthSection: () => <div data-testid="xai-oauth-section" />,
}));

vi.mock("@/components/settings/SubscriptionAccountsPanel", () => ({
  SubscriptionAccountsPanel: () => (
    <div data-testid="subscription-accounts-panel" />
  ),
}));

vi.mock("@/components/settings/LocalCliAuthStatusPanel", () => ({
  LocalCliAuthStatusPanel: () => <div data-testid="local-cli-auth-panel" />,
}));

describe("AuthCenterPanel", () => {
  it("renders the xAI / Grok OAuth management entry alongside existing accounts", () => {
    render(<AuthCenterPanel />);

    expect(screen.getByText("GitHub Copilot")).toBeInTheDocument();
    expect(screen.getByText("ChatGPT (Codex OAuth)")).toBeInTheDocument();
    expect(screen.getByText("xAI (Grok OAuth)")).toBeInTheDocument();
    expect(screen.getByTestId("xai-oauth-section")).toBeInTheDocument();
  });
});
