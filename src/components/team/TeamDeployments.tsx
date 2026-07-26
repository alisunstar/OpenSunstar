/**
 * TeamDeployments — Git MVP 写入闭环 UI
 *
 * 独立维度：生成计划、预览确认、执行部署、偏差检测、回滚。
 * 各操作按数据就绪情况逐步可用，无强制先后依赖。
 * 嵌入 TeamCollaborationPage 的"团队部署"section。
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Rocket,
  FileSearch,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  RotateCcw,
  ShieldAlert,
  Plus,
  Minus,
  Pencil,
  Eye,
  Equal,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  teamConfigApi,
  type DeploymentPlan,
  type DeploymentReceipt,
  type DriftReport,
  type RollbackReport,
} from "@/lib/api/teamConfig";

type DeployPhase = "idle" | "plan" | "receipt" | "drift" | "rollback";

interface TeamDeploymentsProps {
  sourcePath: string;
  projectRoot?: string;
  targetApp?: string;
}

/** 从 unknown 错误中提取可读消息（C4 修复） */
function extractErrorMessage(e: unknown, fallback: string): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return fallback;
}

export function TeamDeployments({
  sourcePath,
  projectRoot: projectRootProp = "",
  targetApp = "claude_code",
}: TeamDeploymentsProps) {
  const [phase, setPhase] = useState<DeployPhase>("idle");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [localProjectRoot, setLocalProjectRoot] = useState("");

  const projectRoot = projectRootProp || localProjectRoot;

  const [plan, setPlan] = useState<DeploymentPlan | null>(null);
  const [receipt, setReceipt] = useState<DeploymentReceipt | null>(null);
  const [drift, setDrift] = useState<DriftReport | null>(null);
  const [rollback, setRollback] = useState<RollbackReport | null>(null);

  // C3 修复：组件卸载后不再 setState
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  const handleGeneratePlan = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await teamConfigApi.generatePlan(
        sourcePath,
        targetApp,
        projectRoot
      );
      if (!mountedRef.current) return;
      setPlan(result);
      setPhase("plan");
    } catch (e: unknown) {
      if (!mountedRef.current) return;
      setError(extractErrorMessage(e, "生成部署计划失败"));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [sourcePath, targetApp, projectRoot]);

  const handleDeploy = useCallback(async (dryRun = false) => {
    setLoading(true);
    setError(null);
    try {
      const result = await teamConfigApi.executeDeployment(
        sourcePath,
        targetApp,
        projectRoot,
        undefined,
        dryRun
      );
      if (!mountedRef.current) return;
      setReceipt(result);
      setPhase("receipt");
    } catch (e: unknown) {
      if (!mountedRef.current) return;
      setError(extractErrorMessage(e, "部署执行失败"));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [sourcePath, targetApp, projectRoot]);

  const handleCheckDrift = useCallback(async () => {
    if (!receipt) return;
    setLoading(true);
    setError(null);
    try {
      const result = await teamConfigApi.checkDrift(
        JSON.stringify(receipt),
        projectRoot
      );
      if (!mountedRef.current) return;
      setDrift(result);
      setPhase("drift");
    } catch (e: unknown) {
      if (!mountedRef.current) return;
      setError(extractErrorMessage(e, "偏差检测失败"));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [receipt, projectRoot]);

  const handleRollback = useCallback(async () => {
    if (!receipt || !drift) return;
    setLoading(true);
    setError(null);
    try {
      const result = await teamConfigApi.rollback(
        JSON.stringify(receipt),
        JSON.stringify(drift),
        projectRoot
      );
      if (!mountedRef.current) return;
      setRollback(result);
      setPhase("rollback");
    } catch (e: unknown) {
      if (!mountedRef.current) return;
      setError(extractErrorMessage(e, "回滚失败"));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [receipt, drift, projectRoot]);

  const reset = useCallback(() => {
    setPhase("idle");
    setPlan(null);
    setReceipt(null);
    setDrift(null);
    setRollback(null);
    setError(null);
  }, []);

  return (
    <div className="space-y-4">
      {/* 项目路径输入（C1 修复：组件自给自足） */}
      {!projectRootProp && (
        <div className="flex items-center gap-2">
          <input
            type="text"
            value={localProjectRoot}
            onChange={(e) => setLocalProjectRoot(e.target.value)}
            placeholder="目标项目路径（部署写入的项目根目录）"
            aria-label="目标项目路径"
            className="flex-1 rounded-md border border-border/60 bg-background px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
      )}

      {/* 未连接提示 */}
      {!sourcePath && (
        <p className="text-xs text-muted-foreground">
          请先在上方「团队配置包」中连接配置源。
        </p>
      )}

      {/* 错误提示 */}
      {error && (
        <div role="alert" className="flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
          <XCircle className="h-4 w-4 shrink-0" aria-hidden="true" />
          <span className="break-all">{error}</span>
        </div>
      )}

      {/* 操作栏 */}
      <div className="flex flex-wrap items-center gap-2">
        {phase === "idle" && (
          <Button size="sm" onClick={handleGeneratePlan} disabled={loading || !sourcePath || !projectRoot}>
            {loading ? <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> : <FileSearch className="mr-1 h-3.5 w-3.5" />}
            生成部署计划
          </Button>
        )}
        {phase === "plan" && plan && (
          <>
            <Button size="sm" onClick={() => handleDeploy(false)} disabled={loading || plan.summary.writeCount === 0}>
              {loading ? <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> : <Rocket className="mr-1 h-3.5 w-3.5" />}
              确认部署 ({plan.summary.writeCount} 项写入)
            </Button>
            <Button size="sm" variant="outline" onClick={() => handleDeploy(true)} disabled={loading}>
              <Eye className="mr-1 h-3.5 w-3.5" />
              预演
            </Button>
            <Button size="sm" variant="ghost" onClick={reset}>
              取消
            </Button>
          </>
        )}
        {phase === "receipt" && (
          <>
            <Button size="sm" variant="outline" onClick={handleCheckDrift} disabled={loading}>
              {loading ? <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> : <FileSearch className="mr-1 h-3.5 w-3.5" />}
              检测偏差
            </Button>
            <Button size="sm" variant="ghost" onClick={reset}>
              完成
            </Button>
          </>
        )}
        {phase === "drift" && drift && (
          <>
            {drift.summary.hasDrift && drift.summary.rollbackEligibleCount > 0 && (
              <Button size="sm" variant="destructive" onClick={handleRollback} disabled={loading}>
                {loading ? <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> : <RotateCcw className="mr-1 h-3.5 w-3.5" />}
                回滚 ({drift.summary.rollbackEligibleCount} 项)
              </Button>
            )}
            <Button size="sm" variant="ghost" onClick={reset}>
              完成
            </Button>
          </>
        )}
        {phase === "rollback" && (
          <Button size="sm" variant="ghost" onClick={reset}>
            完成
          </Button>
        )}
      </div>

      {/* 内容区域 */}
      <AnimatePresence mode="wait">
        {phase === "plan" && plan && (
          <motion.div
            key="plan"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            className="space-y-3"
          >
            <PlanPreview plan={plan} />
          </motion.div>
        )}
        {phase === "receipt" && receipt && (
          <motion.div
            key="receipt"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
          >
            <ReceiptView receipt={receipt} />
          </motion.div>
        )}
        {phase === "drift" && drift && (
          <motion.div
            key="drift"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
          >
            <DriftView drift={drift} />
          </motion.div>
        )}
        {phase === "rollback" && rollback && (
          <motion.div
            key="rollback"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
          >
            <RollbackView rollback={rollback} />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ─── 子组件 ──────────────────────────────────────────────────────────────────

function ActionIcon({ action }: { action: string }) {
  switch (action) {
    case "create":
      return <Plus className="h-3.5 w-3.5 text-green-600" />;
    case "update":
      return <Pencil className="h-3.5 w-3.5 text-amber-600" />;
    case "remove":
      return <Minus className="h-3.5 w-3.5 text-red-600" />;
    case "skip":
      return <Equal className="h-3.5 w-3.5 text-muted-foreground" />;
    default:
      return <Eye className="h-3.5 w-3.5 text-muted-foreground" />;
  }
}

function RiskBadge({ level }: { level: string }) {
  if (level === "safe") return null;
  const colors: Record<string, string> = {
    low: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
    medium: "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400",
    high: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
    requiresTrust: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400",
  };
  return (
    <span className={`inline-block rounded px-1 py-0.5 text-[10px] font-medium ${colors[level] || ""}`}>
      {level}
    </span>
  );
}

function PlanPreview({ plan }: { plan: DeploymentPlan }) {
  return (
    <div className="rounded-lg border bg-card p-4 text-sm">
      <div className="mb-3 flex items-center justify-between">
        <h4 className="font-medium">部署计划</h4>
        <span className="font-mono text-xs text-muted-foreground">
          {plan.planSha256.slice(0, 12)}...
        </span>
      </div>

      {/* 汇总 */}
      <div className="mb-3 flex flex-wrap gap-3 text-xs text-muted-foreground">
        <span className="text-green-600">+{plan.summary.createCount} 新建</span>
        <span className="text-amber-600">~{plan.summary.updateCount} 更新</span>
        <span className="text-red-600">-{plan.summary.removeCount} 移除</span>
        <span>={plan.summary.skipCount} 跳过</span>
        <span>{plan.summary.displayOnlyCount} 仅展示</span>
      </div>

      {/* 警告 */}
      {plan.warnings.length > 0 && (
        <div className="mb-3 space-y-1">
          {plan.warnings.map((w, i) => (
            <div key={i} className="flex items-center gap-1.5 text-xs text-amber-600">
              <ShieldAlert className="h-3 w-3 shrink-0" />
              <span>{w.message}</span>
            </div>
          ))}
        </div>
      )}

      {/* 步骤列表 */}
      <div className="max-h-64 space-y-1 overflow-y-auto">
        {plan.steps.map((step, i) => (
          <div key={i} className="flex items-center gap-2 rounded px-2 py-1 hover:bg-muted/50">
            <ActionIcon action={step.action} />
            <span className="font-mono text-xs">
              [{step.assetType}:{step.assetId}]
            </span>
            <RiskBadge level={step.riskLevel} />
            <span className="ml-auto truncate text-xs text-muted-foreground">
              {step.targetPath}
            </span>
          </div>
        ))}
      </div>

      {plan.summary.hasHighRisk && (
        <div className="mt-3 flex items-center gap-1.5 rounded bg-red-50 px-2 py-1.5 text-xs text-red-700 dark:bg-red-900/20 dark:text-red-400">
          <AlertTriangle className="h-3.5 w-3.5" />
          包含高风险资产，请仔细确认后再部署
        </div>
      )}
    </div>
  );
}

function ReceiptView({ receipt }: { receipt: DeploymentReceipt }) {
  return (
    <div className="rounded-lg border bg-card p-4 text-sm">
      <div className="mb-3 flex items-center gap-2">
        {receipt.summary.allSuccess ? (
          <CheckCircle2 className="h-4 w-4 text-green-600" />
        ) : (
          <AlertTriangle className="h-4 w-4 text-amber-600" />
        )}
        <h4 className="font-medium">
          {receipt.summary.allSuccess ? "部署完成" : "部署完成（有错误）"}
        </h4>
        <span className="ml-auto text-xs text-muted-foreground">
          {receipt.summary.successCount} 成功 / {receipt.summary.failureCount} 失败
        </span>
      </div>

      <div className="max-h-48 space-y-1 overflow-y-auto">
        {receipt.steps
          .filter((s) => s.action !== "skip" && s.action !== "displayOnly")
          .map((step, i) => (
            <div key={i} className="flex items-center gap-2 rounded px-2 py-1">
              {step.success ? (
                <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
              ) : (
                <XCircle className="h-3.5 w-3.5 text-red-600" />
              )}
              <span className="font-mono text-xs">
                [{step.assetType}:{step.assetId}]
              </span>
              <span className="text-xs text-muted-foreground">{step.targetPath}</span>
              {step.error && !step.error.includes("dry-run") && (
                <span className="ml-auto text-xs text-red-500">{step.error}</span>
              )}
            </div>
          ))}
      </div>
    </div>
  );
}

function DriftView({ drift }: { drift: DriftReport }) {
  return (
    <div className="rounded-lg border bg-card p-4 text-sm">
      <div className="mb-3 flex items-center gap-2">
        {drift.summary.hasDrift ? (
          <AlertTriangle className="h-4 w-4 text-amber-600" />
        ) : (
          <CheckCircle2 className="h-4 w-4 text-green-600" />
        )}
        <h4 className="font-medium">
          {drift.summary.hasDrift
            ? `检测到 ${drift.summary.driftedCount} 项偏差`
            : "无偏差"}
        </h4>
      </div>

      {drift.summary.hasDrift && (
        <div className="space-y-1">
          {drift.entries
            .filter((e) => e.status !== "clean" && e.status !== "unknown")
            .map((entry, i) => (
              <div key={i} className="flex items-center gap-2 rounded px-2 py-1">
                <span className="text-xs font-medium text-amber-600">{entry.status}</span>
                <span className="font-mono text-xs">
                  [{entry.assetType}:{entry.assetId}]
                </span>
                <span className="text-xs text-muted-foreground">{entry.targetPath}</span>
                {entry.hasBackup && (
                  <span className="ml-auto text-[10px] text-green-600">可回滚</span>
                )}
              </div>
            ))}
        </div>
      )}
    </div>
  );
}

function RollbackView({ rollback }: { rollback: RollbackReport }) {
  return (
    <div className="rounded-lg border bg-card p-4 text-sm">
      <div className="mb-3 flex items-center gap-2">
        {rollback.summary.allSuccess ? (
          <CheckCircle2 className="h-4 w-4 text-green-600" />
        ) : (
          <AlertTriangle className="h-4 w-4 text-amber-600" />
        )}
        <h4 className="font-medium">
          {rollback.summary.allSuccess ? "回滚完成" : "回滚完成（有错误）"}
        </h4>
        <span className="ml-auto text-xs text-muted-foreground">
          {rollback.summary.successCount} 恢复 / {rollback.summary.failureCount} 失败
        </span>
      </div>

      <div className="space-y-1">
        {rollback.steps.map((step, i) => (
          <div key={i} className="flex items-center gap-2 rounded px-2 py-1">
            {step.success ? (
              <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
            ) : (
              <XCircle className="h-3.5 w-3.5 text-red-600" />
            )}
            <span className="font-mono text-xs">
              [{step.assetType}:{step.assetId}]
            </span>
            <span className="text-xs text-muted-foreground">{step.targetPath}</span>
            {step.error && (
              <span className="ml-auto text-xs text-red-500">{step.error}</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
