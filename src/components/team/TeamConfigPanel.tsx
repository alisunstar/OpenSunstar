/**
 * TeamConfigPanel — Git MVP Local Alpha 只读闭环
 *
 * 流程：连接源 → 浏览 Profile → 有效配置解释 → Release Diff
 * 独立于云平台登录状态，纯本地操作。
 */
import { useCallback, useState } from "react";
import { motion } from "framer-motion";
import {
  FolderOpen,
  GitBranch,
  ShieldCheck,
  ShieldAlert,
  FileDiff,
  Layers,
  ChevronRight,
  RefreshCw,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Plus,
  Minus,
  Pencil,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  teamConfigApi,
  type TeamConnectResponse,
  type TeamProfileSummary,
  type EffectiveConfig,
  type ReleaseDiff,
  type TeamValidateResponse,
} from "@/lib/api/teamConfig";

type PanelTab = "connect" | "profiles" | "effective" | "diff";

export function TeamConfigPanel({ onConnected }: { onConnected?: (path: string) => void }) {
  const [activeTab, setActiveTab] = useState<PanelTab>("connect");
  const [sourcePath, setSourcePath] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [connectResult, setConnectResult] = useState<TeamConnectResponse | null>(null);
  const [connectError, setConnectError] = useState<string | null>(null);

  // Profiles
  const [profiles, setProfiles] = useState<TeamProfileSummary[]>([]);

  // Effective config
  const [effectiveConfig, setEffectiveConfig] = useState<EffectiveConfig | null>(null);
  const [effectiveLoading, setEffectiveLoading] = useState(false);
  const [effectiveError, setEffectiveError] = useState<string | null>(null);
  const [targetApp, setTargetApp] = useState("claude_code");

  // Validation
  const [validation, setValidation] = useState<TeamValidateResponse | null>(null);

  // Diff
  const [releaseDiff, setReleaseDiff] = useState<ReleaseDiff | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [diffError, setDiffError] = useState<string | null>(null);

  const handleConnect = useCallback(async () => {
    if (!sourcePath.trim()) return;
    setConnecting(true);
    setConnectError(null);
    setConnectResult(null);
    try {
      const result = await teamConfigApi.connect(sourcePath.trim());
      setConnectResult(result);
      onConnected?.(sourcePath.trim());
      // 连接成功后自动加载 profiles
      const profs = await teamConfigApi.listProfiles(sourcePath.trim());
      setProfiles(profs);
      // 静默校验
      const val = await teamConfigApi.validate(sourcePath.trim(), false);
      setValidation(val);
      setActiveTab("profiles");
    } catch (e: unknown) {
      setConnectError(typeof e === "string" ? e : (e as Error)?.message ?? "连接失败");
    } finally {
      setConnecting(false);
    }
  }, [sourcePath]);

  const handleExplain = useCallback(async () => {
    if (!sourcePath.trim()) return;
    setEffectiveLoading(true);
    setEffectiveError(null);
    try {
      const config = await teamConfigApi.getEffectiveState(sourcePath.trim(), targetApp);
      setEffectiveConfig(config);
      setActiveTab("effective");
    } catch (e: unknown) {
      setEffectiveError(typeof e === "string" ? e : (e as Error)?.message ?? "编译失败");
    } finally {
      setEffectiveLoading(false);
    }
  }, [sourcePath, targetApp]);

  const handleDiff = useCallback(async () => {
    if (!sourcePath.trim()) return;
    setDiffLoading(true);
    setDiffError(null);
    setReleaseDiff(null);
    try {
      const diff = await teamConfigApi.getReleaseDiff(sourcePath.trim());
      setReleaseDiff(diff);
      setActiveTab("diff");
    } catch (e: unknown) {
      setDiffError(typeof e === "string" ? e : (e as Error)?.message ?? "Diff 失败");
    } finally {
      setDiffLoading(false);
    }
  }, [sourcePath]);

  return (
    <div className="space-y-4">
      {/* 连接区域 */}
      <section className="rounded-xl border border-border/60 bg-card/75 p-5">
        <div className="flex items-center gap-2 mb-3">
          <FolderOpen className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-medium">团队配置源</h3>
          {connectResult && (
            <span className="ml-auto inline-flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-[11px] text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 className="h-3 w-3" />
              已连接
            </span>
          )}
        </div>

        <div className="flex gap-2">
          <input
            type="text"
            value={sourcePath}
            onChange={(e) => setSourcePath(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleConnect()}
            placeholder="团队配置包路径（本地目录或 Git 仓库）"
            className="flex-1 rounded-lg border border-border/60 bg-background/50 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-primary/20"
          />
          <Button size="sm" onClick={handleConnect} disabled={connecting || !sourcePath.trim()}>
            {connecting ? <RefreshCw className="h-3.5 w-3.5 animate-spin" /> : "连接"}
          </Button>
        </div>

        {connectError && (
          <p className="mt-2 text-xs text-destructive">{connectError}</p>
        )}

        {connectResult && (
          <motion.div
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-3 grid grid-cols-2 gap-2 text-xs text-muted-foreground sm:grid-cols-4"
          >
            <InfoChip label="名称" value={connectResult.name} />
            <InfoChip
              label="类型"
              value={connectResult.sourceKind === "git" ? "Git 仓库" : "本地目录"}
            />
            <InfoChip label="Profiles" value={String(connectResult.profilesCount)} />
            <InfoChip label="策略" value={String(connectResult.policiesCount)} />
            {connectResult.branch && (
              <InfoChip label="分支" value={connectResult.branch} icon={<GitBranch className="h-3 w-3" />} />
            )}
            {validation && (
              <InfoChip
                label="校验"
                value={validation.passed ? "通过" : `${validation.errors.length} 错误`}
                icon={
                  validation.passed ? (
                    <ShieldCheck className="h-3 w-3 text-emerald-500" />
                  ) : (
                    <ShieldAlert className="h-3 w-3 text-amber-500" />
                  )
                }
              />
            )}
          </motion.div>
        )}

        {connectResult?.warnings && connectResult.warnings.length > 0 && (
          <div className="mt-2 space-y-1">
            {connectResult.warnings.map((w, i) => (
              <p key={i} className="flex items-center gap-1 text-[11px] text-amber-600 dark:text-amber-400">
                <AlertTriangle className="h-3 w-3 shrink-0" />
                {w}
              </p>
            ))}
          </div>
        )}
      </section>

      {/* 连接后：Tabs 展示 Profiles / 有效配置 / Diff */}
      {connectResult && (
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as PanelTab)}>
          <TabsList className="w-full justify-start">
            <TabsTrigger value="profiles" className="text-xs">
              <Layers className="mr-1 h-3 w-3" />
              Profiles
            </TabsTrigger>
            <TabsTrigger value="effective" className="text-xs">
              <ChevronRight className="mr-1 h-3 w-3" />
              有效配置
            </TabsTrigger>
            <TabsTrigger value="diff" className="text-xs">
              <FileDiff className="mr-1 h-3 w-3" />
              Release Diff
            </TabsTrigger>
          </TabsList>

          {/* Profiles Tab */}
          <TabsContent value="profiles" className="mt-3">
            <div className="space-y-2">
              {profiles.map((p) => (
                <div
                  key={p.profileId}
                  className="rounded-lg border border-border/40 bg-card/50 px-4 py-3"
                >
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{p.name}</span>
                    <span className="text-[11px] text-muted-foreground">
                      {p.assetsCount} 资产 · {p.credentialSlotsCount} 凭据槽
                    </span>
                  </div>
                  {p.description && (
                    <p className="mt-1 text-xs text-muted-foreground">{p.description}</p>
                  )}
                  <p className="mt-1 text-[10px] text-muted-foreground/60 font-mono">{p.profileId}</p>
                </div>
              ))}
              {profiles.length === 0 && (
                <p className="text-xs text-muted-foreground py-4 text-center">
                  未找到 Profile 定义
                </p>
              )}
            </div>
          </TabsContent>

          {/* Effective Config Tab */}
          <TabsContent value="effective" className="mt-3">
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <select
                  value={targetApp}
                  onChange={(e) => setTargetApp(e.target.value)}
                  className="rounded-lg border border-border/60 bg-background/50 px-2 py-1.5 text-xs outline-none"
                >
                  <option value="claude_code">Claude Code</option>
                  <option value="codex">Codex</option>
                </select>
                <Button size="sm" variant="outline" onClick={handleExplain} disabled={effectiveLoading}>
                  {effectiveLoading ? (
                    <RefreshCw className="h-3 w-3 animate-spin" />
                  ) : (
                    "编译有效配置"
                  )}
                </Button>
              </div>

              {effectiveError && <p className="text-xs text-destructive">{effectiveError}</p>}

              {effectiveConfig && (
                <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="space-y-2">
                  <p className="text-[11px] text-muted-foreground font-mono">
                    SHA-256: {effectiveConfig.configSha256.slice(0, 16)}…
                  </p>
                  <div className="space-y-1.5">
                    {effectiveConfig.items.map((item, i) => (
                      <div
                        key={i}
                        className="flex items-start gap-2 rounded-lg border border-border/30 px-3 py-2 text-xs"
                      >
                        <DecisionIcon decision={item.decision} />
                        <div className="flex-1 min-w-0">
                          <span className="font-mono text-[11px]">
                            {item.assetType}:{item.assetId}
                          </span>
                          {item.provenance.length > 0 && (
                            <p className="mt-0.5 text-[10px] text-muted-foreground">
                              {item.provenance[item.provenance.length - 1].explanation}
                            </p>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                  {effectiveConfig.conflicts.length > 0 && (
                    <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3">
                      <p className="text-xs font-medium text-amber-600 dark:text-amber-400 mb-1">
                        冲突
                      </p>
                      {effectiveConfig.conflicts.map((c, i) => (
                        <p key={i} className="text-[11px] text-amber-600/80 dark:text-amber-400/80">
                          {c.assetId}: {c.message}
                        </p>
                      ))}
                    </div>
                  )}
                </motion.div>
              )}
            </div>
          </TabsContent>

          {/* Release Diff Tab */}
          <TabsContent value="diff" className="mt-3">
            <div className="space-y-3">
              <Button size="sm" variant="outline" onClick={handleDiff} disabled={diffLoading}>
                {diffLoading ? (
                  <RefreshCw className="h-3 w-3 animate-spin" />
                ) : (
                  <FileDiff className="mr-1 h-3 w-3" />
                )}
                比对 lock.json 基线
              </Button>

              {diffError && <p className="text-xs text-destructive">{diffError}</p>}

              {releaseDiff && (
                <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="space-y-2">
                  {!releaseDiff.summary.hasChanges ? (
                    <p className="flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400 py-2">
                      <CheckCircle2 className="h-3.5 w-3.5" />
                      与基线 {releaseDiff.baseRef} 一致，无变更
                    </p>
                  ) : (
                    <>
                      <div className="flex gap-3 text-[11px] text-muted-foreground">
                        <span className="text-emerald-600 dark:text-emerald-400">
                          +{releaseDiff.summary.addedCount} 新增
                        </span>
                        <span className="text-red-600 dark:text-red-400">
                          -{releaseDiff.summary.removedCount} 删除
                        </span>
                        <span className="text-amber-600 dark:text-amber-400">
                          ~{releaseDiff.summary.modifiedCount} 修改
                        </span>
                        <span>={releaseDiff.summary.unchangedCount} 不变</span>
                      </div>
                      <div className="space-y-1 max-h-64 overflow-y-auto">
                        {releaseDiff.added.map((e) => (
                          <DiffRow key={e.path} icon={<Plus className="h-3 w-3 text-emerald-500" />} path={e.path} detail={`${e.newSize ?? 0} B`} color="text-emerald-600 dark:text-emerald-400" />
                        ))}
                        {releaseDiff.removed.map((e) => (
                          <DiffRow key={e.path} icon={<Minus className="h-3 w-3 text-red-500" />} path={e.path} detail="" color="text-red-600 dark:text-red-400" />
                        ))}
                        {releaseDiff.modified.map((e) => (
                          <DiffRow key={e.path} icon={<Pencil className="h-3 w-3 text-amber-500" />} path={e.path} detail={`${e.oldSize ?? 0} → ${e.newSize ?? 0} B`} color="text-amber-600 dark:text-amber-400" />
                        ))}
                      </div>
                    </>
                  )}
                </motion.div>
              )}
            </div>
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}

// ─── 辅助组件 ───────────────────────────────────────────────────────────────

function InfoChip({
  label,
  value,
  icon,
}: {
  label: string;
  value: string;
  icon?: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-1 rounded-lg bg-muted/40 px-2 py-1">
      {icon}
      <span className="text-muted-foreground">{label}:</span>
      <span className="font-medium text-foreground truncate">{value}</span>
    </div>
  );
}

function DecisionIcon({ decision }: { decision: string }) {
  switch (decision) {
    case "enabled":
      return <CheckCircle2 className="h-3.5 w-3.5 text-emerald-500 shrink-0 mt-0.5" />;
    case "denied":
      return <XCircle className="h-3.5 w-3.5 text-red-500 shrink-0 mt-0.5" />;
    case "conflicted":
      return <AlertTriangle className="h-3.5 w-3.5 text-amber-500 shrink-0 mt-0.5" />;
    default:
      return <span className="h-3.5 w-3.5 rounded-full border border-muted-foreground/30 shrink-0 mt-0.5" />;
  }
}

function DiffRow({
  icon,
  path,
  detail,
  color,
}: {
  icon: React.ReactNode;
  path: string;
  detail: string;
  color: string;
}) {
  return (
    <div className="flex items-center gap-2 rounded-md px-2 py-1 text-[11px] hover:bg-muted/30">
      {icon}
      <span className={`font-mono ${color}`}>{path}</span>
      {detail && <span className="ml-auto text-muted-foreground">{detail}</span>}
    </div>
  );
}
