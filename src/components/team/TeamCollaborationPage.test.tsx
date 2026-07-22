import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { TeamCollaborationPage } from "./TeamCollaborationPage";

const productAuthApiMock = vi.hoisted(() => ({
  getSession: vi.fn(),
  login: vi.fn(),
  cancelLogin: vi.fn(),
  logout: vi.fn(),
  createOrganization: vi.fn(),
  acceptInvite: vi.fn(),
  getOverview: vi.fn(),
  listMembers: vi.fn(),
  listInvites: vi.fn(),
  inviteMember: vi.fn(),
  removeMember: vi.fn(),
}));

vi.mock("@/lib/api/productAuth", () => ({
  productAuthApi: productAuthApiMock,
}));

describe("TeamCollaborationPage", () => {
  beforeEach(() => {
    productAuthApiMock.getSession.mockReset();
    productAuthApiMock.login.mockReset();
    productAuthApiMock.cancelLogin.mockReset();
    productAuthApiMock.logout.mockReset();
    productAuthApiMock.createOrganization.mockReset();
    productAuthApiMock.acceptInvite.mockReset();
    productAuthApiMock.getOverview.mockReset();
    productAuthApiMock.listMembers.mockReset();
    productAuthApiMock.listInvites.mockReset();
    productAuthApiMock.inviteMember.mockReset();
    productAuthApiMock.removeMember.mockReset();
    productAuthApiMock.getOverview.mockResolvedValue({
      membership: {
        orgId: "org_123",
        userId: "user_123",
        role: "owner",
        joinedAt: "2026-07-22T00:00:00.000Z",
      },
      seatUsage: 1,
      entitlement: null,
      access: { active: false, capabilities: [] },
    });
    productAuthApiMock.listMembers.mockResolvedValue({ members: [] });
    productAuthApiMock.listInvites.mockResolvedValue({ invites: [] });
  });

  it("shows an honest connection state when no product session exists", async () => {
    productAuthApiMock.getSession.mockResolvedValue({
      signed_in: false,
      user_id: null,
      email: null,
      organization_id: null,
      expires_at_unix: null,
    });

    render(<TeamCollaborationPage />);

    expect(
      await screen.findByText("尚未登录 OpenSunstar 账户"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "登录 / 注册" })).toBeEnabled();
    expect(screen.getByText("本机配置不会自动上传")).toBeInTheDocument();
  });

  it("shows organization onboarding after login without an organization", async () => {
    productAuthApiMock.getSession.mockResolvedValue({
      signed_in: true,
      user_id: "user_123",
      email: "founder@example.com",
      organization_id: null,
      expires_at_unix: 1_800_000_000,
    });

    render(<TeamCollaborationPage />);

    expect(await screen.findByText("founder@example.com")).toBeInTheDocument();
    expect(screen.getByText("创建组织或接受邀请")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "创建组织" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "接受邀请" })).toBeDisabled();
  });

  it("starts the native browser login flow and reloads the safe session summary", async () => {
    const user = userEvent.setup();
    productAuthApiMock.getSession
      .mockResolvedValueOnce({
        signed_in: false,
        user_id: null,
        email: null,
        organization_id: null,
        expires_at_unix: null,
      })
      .mockResolvedValueOnce({
        signed_in: true,
        user_id: "user_123",
        email: "founder@example.com",
        organization_id: null,
        expires_at_unix: 1_800_000_000,
      });
    productAuthApiMock.login.mockResolvedValue({ signed_in: true });

    render(<TeamCollaborationPage />);
    await user.click(
      await screen.findByRole("button", { name: "登录 / 注册" }),
    );

    await waitFor(() =>
      expect(productAuthApiMock.login).toHaveBeenCalledTimes(1),
    );
    expect(await screen.findByText("founder@example.com")).toBeInTheDocument();
  });

  it("shows the connected organization and clears the local session on logout", async () => {
    const user = userEvent.setup();
    productAuthApiMock.getSession
      .mockResolvedValueOnce({
        signed_in: true,
        user_id: "user_123",
        email: "admin@example.com",
        organization_id: "org_123",
        expires_at_unix: 1_800_000_000,
      })
      .mockResolvedValueOnce({
        signed_in: false,
        user_id: null,
        email: null,
        organization_id: null,
        expires_at_unix: null,
      });
    productAuthApiMock.logout.mockResolvedValue(undefined);

    render(<TeamCollaborationPage />);

    expect(await screen.findByText("org_123")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "退出登录" }));

    await waitFor(() =>
      expect(productAuthApiMock.logout).toHaveBeenCalledTimes(1),
    );
    expect(
      await screen.findByText("尚未登录 OpenSunstar 账户"),
    ).toBeInTheDocument();
  });

  it("keeps the page usable when the local session cannot be read", async () => {
    productAuthApiMock.getSession.mockRejectedValue(
      new Error("keychain unavailable"),
    );

    render(<TeamCollaborationPage />);

    expect(await screen.findByText("暂时无法读取账户状态")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重新检查" })).toBeEnabled();
  });
});
