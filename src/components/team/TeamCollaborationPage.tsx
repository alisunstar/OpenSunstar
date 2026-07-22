import { useCallback, useEffect, useState } from "react";
import { motion } from "framer-motion";
import {
  Building2,
  Check,
  CircleDashed,
  KeyRound,
  Loader2,
  LogOut,
  RefreshCw,
  ShieldCheck,
  UserPlus,
  Users,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  productAuthApi,
  type ProductSessionSummary,
  type TeamMembership,
  type TeamOverview,
  type TeamInvite,
} from "@/lib/api/productAuth";
import { cn } from "@/lib/utils";

type AccountState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; session: ProductSessionSummary };

const collaborationSteps = [
  { label: "身份", icon: KeyRound },
  { label: "组织", icon: Building2 },
  { label: "策略", icon: ShieldCheck },
  { label: "发布", icon: Users },
];

function activeStepCount(session: ProductSessionSummary | null): number {
  if (!session?.signed_in) return 0;
  return session.organization_id ? 2 : 1;
}

function CollaborationRail({
  session,
}: {
  session: ProductSessionSummary | null;
}) {
  const activeCount = activeStepCount(session);
  return (
    <div className="grid grid-cols-4 overflow-hidden rounded-xl border border-border/60 bg-background/50">
      {collaborationSteps.map((step, index) => {
        const complete = index < activeCount;
        const Icon = step.icon;
        return (
          <div
            key={step.label}
            className={cn(
              "relative flex min-h-20 items-center gap-3 border-r border-border/50 px-4 last:border-r-0",
              complete && "bg-emerald-500/[0.06]",
            )}
          >
            <span
              className={cn(
                "grid size-8 place-items-center rounded-full border",
                complete
                  ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-500"
                  : "border-border bg-muted/30 text-muted-foreground",
              )}
            >
              {complete ? (
                <Check className="size-4" />
              ) : (
                <Icon className="size-4" />
              )}
            </span>
            <div>
              <p className="text-[10px] font-medium uppercase tracking-[0.2em] text-muted-foreground">
                0{index + 1}
              </p>
              <p className="text-sm font-semibold">{step.label}</p>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function TeamCollaborationPage() {
  const [accountState, setAccountState] = useState<AccountState>({
    kind: "loading",
  });
  const [loggingOut, setLoggingOut] = useState(false);
  const [working, setWorking] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [organizationName, setOrganizationName] = useState("");
  const [inviteToken, setInviteToken] = useState("");
  const [inviteEmail, setInviteEmail] = useState("");
  const [createdInviteToken, setCreatedInviteToken] = useState<string | null>(
    null,
  );
  const [overview, setOverview] = useState<TeamOverview | null>(null);
  const [members, setMembers] = useState<TeamMembership[]>([]);
  const [pendingInvites, setPendingInvites] = useState<TeamInvite[]>([]);
  const [teamDataError, setTeamDataError] = useState<string | null>(null);

  const loadSession = useCallback(async () => {
    setAccountState({ kind: "loading" });
    try {
      const session = await productAuthApi.getSession();
      setAccountState({ kind: "ready", session });
    } catch (error) {
      setAccountState({
        kind: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }, []);

  useEffect(() => {
    void loadSession();
  }, [loadSession]);

  const handleLogout = async () => {
    setLoggingOut(true);
    try {
      await productAuthApi.logout();
      await loadSession();
    } catch (error) {
      setAccountState({
        kind: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setLoggingOut(false);
    }
  };

  const session = accountState.kind === "ready" ? accountState.session : null;

  useEffect(() => {
    if (!session?.organization_id) {
      setOverview(null);
      setMembers([]);
      setPendingInvites([]);
      return;
    }
    let cancelled = false;
    setTeamDataError(null);
    void Promise.all([
      productAuthApi.getOverview(session.organization_id),
      productAuthApi.listMembers(session.organization_id),
      productAuthApi.listInvites(session.organization_id),
    ])
      .then(([nextOverview, memberResult, inviteResult]) => {
        if (!cancelled) {
          setOverview(nextOverview);
          setMembers(memberResult.members);
          setPendingInvites(inviteResult.invites);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setTeamDataError(
            error instanceof Error ? error.message : String(error),
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [session?.organization_id]);

  const runAction = async (action: () => Promise<unknown>) => {
    setWorking(true);
    setActionError(null);
    try {
      await action();
      await loadSession();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setWorking(false);
    }
  };

  const slugFromName = (name: string) =>
    name
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");

  return (
    <div className="h-full overflow-y-auto bg-[radial-gradient(circle_at_86%_8%,hsl(var(--primary)/0.07),transparent_28%)]">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.24 }}
        className="mx-auto max-w-6xl space-y-5 px-6 py-7"
      >
        <section className="flex items-end justify-between gap-6">
          <div>
            <p className="mb-2 text-[10px] font-semibold uppercase tracking-[0.28em] text-primary/80">
              Team control plane
            </p>
            <h2 className="text-2xl font-semibold tracking-tight">
              团队配置中心
            </h2>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
              管理成员身份、组织边界和团队级 AI
              配置交付。本机资产默认留在本机，只有明确发布的版本进入团队空间。
            </p>
          </div>
          {session?.signed_in && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => void handleLogout()}
              disabled={loggingOut}
            >
              {loggingOut ? (
                <Loader2 className="mr-2 size-4 animate-spin" />
              ) : (
                <LogOut className="mr-2 size-4" />
              )}
              退出登录
            </Button>
          )}
        </section>

        <CollaborationRail session={session} />

        {actionError && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/[0.04] px-4 py-3 text-sm text-destructive">
            操作未完成：{actionError}
          </div>
        )}

        {accountState.kind === "loading" && (
          <section className="grid min-h-64 place-items-center rounded-2xl border border-border/60 bg-card/70">
            <div className="text-center text-sm text-muted-foreground">
              <Loader2 className="mx-auto mb-3 size-5 animate-spin" />
              正在读取本机账户状态
            </div>
          </section>
        )}

        {accountState.kind === "error" && (
          <section className="rounded-2xl border border-destructive/30 bg-destructive/[0.04] p-6">
            <p className="text-base font-semibold">暂时无法读取账户状态</p>
            <p className="mt-2 text-sm text-muted-foreground">
              {accountState.message}
            </p>
            <Button
              className="mt-5"
              variant="outline"
              onClick={() => void loadSession()}
            >
              <RefreshCw className="mr-2 size-4" />
              重新检查
            </Button>
          </section>
        )}

        {accountState.kind === "ready" && !session?.signed_in && (
          <section className="grid gap-5 rounded-2xl border border-border/60 bg-card/75 p-6 shadow-sm md:grid-cols-[1.4fr_0.6fr]">
            <div className="flex flex-col justify-between">
              <div>
                <span className="inline-flex rounded-full border border-amber-500/30 bg-amber-500/10 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider text-amber-600 dark:text-amber-400">
                  身份未连接
                </span>
                <h3 className="mt-5 text-xl font-semibold">
                  尚未登录 OpenSunstar 账户
                </h3>
                <p className="mt-2 max-w-xl text-sm leading-6 text-muted-foreground">
                  使用系统浏览器完成 GitHub OAuth 或邮箱 Magic
                  Auth。登录后才会获取所属组织、团队席位和授权能力。
                </p>
              </div>
              <Button
                className="mt-8 w-fit"
                disabled={working}
                onClick={() => void runAction(() => productAuthApi.login())}
              >
                {working ? (
                  <Loader2 className="mr-2 size-4 animate-spin" />
                ) : (
                  <KeyRound className="mr-2 size-4" />
                )}
                登录 / 注册
              </Button>
              {working && (
                <Button
                  className="ml-3 mt-8 w-fit"
                  variant="outline"
                  onClick={() => void productAuthApi.cancelLogin()}
                >
                  取消登录
                </Button>
              )}
            </div>
            <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/[0.05] p-5">
              <ShieldCheck className="size-5 text-emerald-500" />
              <p className="mt-5 text-sm font-semibold">本机配置不会自动上传</p>
              <p className="mt-2 text-xs leading-5 text-muted-foreground">
                登录仅建立产品账户身份。项目文件、密钥和个人 AI
                配置仍遵循本地优先与显式发布原则。
              </p>
            </div>
          </section>
        )}

        {accountState.kind === "ready" &&
          session?.signed_in &&
          !session.organization_id && (
            <section className="rounded-2xl border border-border/60 bg-card/75 p-6 shadow-sm">
              <div className="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <p className="text-xs text-muted-foreground">已登录账户</p>
                  <p className="mt-1 font-medium">{session.email}</p>
                </div>
                <span className="rounded-full bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-600 dark:text-emerald-400">
                  身份已验证
                </span>
              </div>
              <div className="my-6 h-px bg-border/60" />
              <h3 className="text-lg font-semibold">创建组织或接受邀请</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                组织是成员、席位、团队配置版本与审计记录的归属边界。
              </p>
              <div className="mt-6 grid gap-4 md:grid-cols-2">
                <div className="space-y-3 rounded-xl border border-border/60 p-4">
                  <label
                    className="text-xs font-medium"
                    htmlFor="organization-name"
                  >
                    组织名称
                  </label>
                  <input
                    id="organization-name"
                    value={organizationName}
                    onChange={(event) =>
                      setOrganizationName(event.target.value)
                    }
                    placeholder="例如：Acme Engineering"
                    className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                  />
                  <Button
                    disabled={
                      working || slugFromName(organizationName).length < 3
                    }
                    onClick={() =>
                      void runAction(() =>
                        productAuthApi.createOrganization(
                          organizationName,
                          slugFromName(organizationName),
                        ),
                      )
                    }
                  >
                    <Building2 className="mr-2 size-4" />
                    创建组织
                  </Button>
                </div>
                <div className="space-y-3 rounded-xl border border-border/60 p-4">
                  <label className="text-xs font-medium" htmlFor="invite-token">
                    邀请令牌
                  </label>
                  <input
                    id="invite-token"
                    value={inviteToken}
                    onChange={(event) => setInviteToken(event.target.value)}
                    placeholder="粘贴一次性邀请令牌"
                    className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                  />
                  <Button
                    variant="outline"
                    disabled={working || inviteToken.trim().length === 0}
                    onClick={() =>
                      void runAction(() =>
                        productAuthApi.acceptInvite(inviteToken.trim()),
                      )
                    }
                  >
                    <UserPlus className="mr-2 size-4" />
                    接受邀请
                  </Button>
                </div>
              </div>
            </section>
          )}

        {accountState.kind === "ready" &&
          session?.signed_in &&
          session.organization_id && (
            <>
              <section className="rounded-2xl border border-emerald-500/20 bg-card/75 p-6 shadow-sm">
                <div className="flex flex-wrap items-start justify-between gap-5">
                  <div>
                    <p className="text-xs text-muted-foreground">当前组织</p>
                    <h3 className="mt-1 font-mono text-xl font-semibold tracking-tight">
                      {session.organization_id}
                    </h3>
                    <p className="mt-2 text-sm text-muted-foreground">
                      {session.email}
                    </p>
                  </div>
                  <span className="inline-flex items-center gap-2 rounded-full bg-emerald-500/10 px-3 py-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-400">
                    <Check className="size-3.5" />
                    组织已连接
                  </span>
                </div>
              </section>
              {teamDataError && (
                <section className="rounded-xl border border-amber-500/30 bg-amber-500/[0.06] p-4 text-sm">
                  控制面当前不可用，以下不显示为在线数据：{teamDataError}
                </section>
              )}
              {overview && (
                <section className="grid gap-4 md:grid-cols-3">
                  <div className="rounded-xl border border-border/60 bg-card/75 p-5">
                    <Users className="size-5 text-primary" />
                    <p className="mt-4 text-sm font-semibold">成员与角色</p>
                    <p className="mt-2 text-2xl font-semibold">
                      {members.length}
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      当前角色：{overview.membership.role}
                    </p>
                  </div>
                  <div className="rounded-xl border border-border/60 bg-card/75 p-5">
                    <ShieldCheck className="size-5 text-primary" />
                    <p className="mt-4 text-sm font-semibold">Team 权益</p>
                    <p className="mt-2 text-lg font-semibold">
                      {overview.access.active ? "已生效" : "未开通 / 已过期"}
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      席位 {overview.seatUsage}/
                      {overview.entitlement?.seatLimit ?? 1}
                    </p>
                    {overview.entitlement && (
                      <p className="mt-1 text-xs text-muted-foreground">
                        到期：
                        {new Date(
                          overview.entitlement.expiresAt,
                        ).toLocaleDateString()}
                      </p>
                    )}
                  </div>
                  <div className="rounded-xl border border-border/60 bg-card/75 p-5">
                    <CircleDashed className="size-5 text-primary" />
                    <p className="mt-4 text-sm font-semibold">授权能力</p>
                    <p className="mt-2 text-xs leading-5 text-muted-foreground">
                      {overview.access.capabilities.length > 0
                        ? overview.access.capabilities.join(" · ")
                        : "暂无 Team capability；请由运营管理员手动开通。"}
                    </p>
                  </div>
                </section>
              )}
              {overview &&
                ["owner", "admin"].includes(overview.membership.role) && (
                  <section className="rounded-xl border border-border/60 bg-card/75 p-5">
                    <h3 className="text-sm font-semibold">邀请成员</h3>
                    <p className="mt-1 text-xs text-muted-foreground">
                      原始令牌只显示一次，请通过可信渠道发送给受邀邮箱本人。
                    </p>
                    <div className="mt-4 flex flex-wrap gap-3">
                      <input
                        aria-label="成员邮箱"
                        type="email"
                        value={inviteEmail}
                        onChange={(event) => setInviteEmail(event.target.value)}
                        placeholder="member@example.com"
                        className="h-9 min-w-64 rounded-md border border-input bg-background px-3 text-sm"
                      />
                      <Button
                        disabled={working || !inviteEmail.includes("@")}
                        onClick={() => {
                          setWorking(true);
                          setActionError(null);
                          void productAuthApi
                            .inviteMember(
                              session.organization_id!,
                              inviteEmail.trim(),
                              "member",
                            )
                            .then((result) =>
                              setCreatedInviteToken(result.rawToken),
                            )
                            .catch((error: unknown) =>
                              setActionError(
                                error instanceof Error
                                  ? error.message
                                  : String(error),
                              ),
                            )
                            .finally(() => setWorking(false));
                        }}
                      >
                        <UserPlus className="mr-2 size-4" />
                        生成邀请
                      </Button>
                    </div>
                    {createdInviteToken && (
                      <code className="mt-4 block break-all rounded-md bg-muted p-3 text-xs">
                        {createdInviteToken}
                      </code>
                    )}
                  </section>
                )}
              {overview && (
                <section className="grid gap-4 lg:grid-cols-2">
                  <div className="rounded-xl border border-border/60 bg-card/75 p-5">
                    <h3 className="text-sm font-semibold">成员</h3>
                    <div className="mt-4 space-y-2">
                      {members.map((member) => (
                        <div
                          key={member.userId}
                          className="flex items-center justify-between rounded-lg border border-border/50 px-3 py-2"
                        >
                          <div>
                            <p className="font-mono text-xs">{member.userId}</p>
                            <p className="text-[11px] text-muted-foreground">
                              {member.role}
                            </p>
                          </div>
                          {["owner", "admin"].includes(
                            overview.membership.role,
                          ) &&
                            member.role !== "owner" && (
                              <Button
                                size="sm"
                                variant="ghost"
                                onClick={() => {
                                  setWorking(true);
                                  void productAuthApi
                                    .removeMember(member.orgId, member.userId)
                                    .then(() =>
                                      setMembers((current) =>
                                        current.filter(
                                          (item) =>
                                            item.userId !== member.userId,
                                        ),
                                      ),
                                    )
                                    .catch((error: unknown) =>
                                      setActionError(
                                        error instanceof Error
                                          ? error.message
                                          : String(error),
                                      ),
                                    )
                                    .finally(() => setWorking(false));
                                }}
                              >
                                移除
                              </Button>
                            )}
                        </div>
                      ))}
                    </div>
                  </div>
                  <div className="rounded-xl border border-border/60 bg-card/75 p-5">
                    <h3 className="text-sm font-semibold">待处理邀请</h3>
                    <div className="mt-4 space-y-2 text-xs">
                      {pendingInvites.length === 0 && (
                        <p className="text-muted-foreground">暂无待处理邀请</p>
                      )}
                      {pendingInvites.map((invite) => (
                        <div
                          key={`${invite.email}-${invite.expiresAt}`}
                          className="rounded-lg border border-border/50 px-3 py-2"
                        >
                          <p>{invite.email}</p>
                          <p className="mt-1 text-[11px] text-muted-foreground">
                            {invite.role} · 到期{" "}
                            {new Date(invite.expiresAt).toLocaleDateString()}
                          </p>
                        </div>
                      ))}
                    </div>
                  </div>
                </section>
              )}
            </>
          )}
      </motion.div>
    </div>
  );
}
