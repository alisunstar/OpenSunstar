import { invoke } from "@tauri-apps/api/core";

export interface ProductSessionSummary {
  signed_in: boolean;
  user_id: string | null;
  email: string | null;
  organization_id: string | null;
  expires_at_unix: number | null;
}

export type TeamRole = "owner" | "admin" | "member" | "viewer";

export interface TeamMembership {
  orgId: string;
  userId: string;
  role: TeamRole;
  joinedAt: string;
}

export interface TeamEntitlement {
  id: string;
  orgId: string;
  plan: "team_design_partner";
  status: "active" | "suspended" | "expired";
  seatLimit: number;
  capabilities: string[];
  issuedAt: string;
  expiresAt: string;
}

export interface TeamOverview {
  membership: TeamMembership;
  seatUsage: number;
  entitlement: TeamEntitlement | null;
  access: { active: boolean; capabilities: string[] };
}

export interface TeamInvite {
  orgId: string;
  email: string;
  role: Exclude<TeamRole, "owner">;
  status: string;
  expiresAt: string;
}

export async function getProductSession(): Promise<ProductSessionSummary> {
  return invoke<ProductSessionSummary>("product_auth_get_session");
}

export async function logoutProductSession(): Promise<void> {
  return invoke("product_auth_logout");
}

export async function loginProductSession(): Promise<ProductSessionSummary> {
  return invoke<ProductSessionSummary>("product_auth_login");
}

export async function cancelProductLogin(): Promise<void> {
  return invoke("product_auth_cancel_login");
}

export async function createTeamOrganization(name: string, slug: string) {
  return invoke<{ organization: { id: string; name: string; slug: string } }>(
    "product_team_create_organization",
    { name, slug },
  );
}

export async function acceptTeamInvite(rawToken: string) {
  return invoke<{ membership: TeamMembership }>("product_team_accept_invite", {
    rawToken,
  });
}

export async function getTeamOverview(orgId: string): Promise<TeamOverview> {
  return invoke<TeamOverview>("product_team_get_overview", { orgId });
}

export async function listTeamMembers(orgId: string) {
  return invoke<{ members: TeamMembership[] }>("product_team_list_members", {
    orgId,
  });
}

export async function listTeamInvites(orgId: string) {
  return invoke<{ invites: TeamInvite[] }>("product_team_list_invites", {
    orgId,
  });
}

export async function inviteTeamMember(
  orgId: string,
  email: string,
  role: Exclude<TeamRole, "owner">,
) {
  return invoke<{ rawToken: string }>("product_team_invite_member", {
    orgId,
    email,
    role,
  });
}

export async function removeTeamMember(orgId: string, userId: string) {
  return invoke<void>("product_team_remove_member", { orgId, userId });
}

export const productAuthApi = {
  getSession: getProductSession,
  login: loginProductSession,
  cancelLogin: cancelProductLogin,
  logout: logoutProductSession,
  createOrganization: createTeamOrganization,
  acceptInvite: acceptTeamInvite,
  getOverview: getTeamOverview,
  listMembers: listTeamMembers,
  listInvites: listTeamInvites,
  inviteMember: inviteTeamMember,
  removeMember: removeTeamMember,
};
