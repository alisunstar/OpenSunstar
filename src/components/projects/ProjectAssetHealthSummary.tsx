import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  CircleHelp,
  ClipboardCheck,
  RefreshCw,
  RotateCcw,
  XCircle,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { projectsApi } from "@/lib/api/projects";
import type {
  AssetHealthPlan,
  AssetHealthRecord,
  AssetHealthStatus,
} from "@/types/assetHealth";

interface ProjectAssetHealthSummaryProps {
  projectId: string;
}

const STATUS_ORDER: AssetHealthStatus[] = [
  "unhealthy",
  "attention",
  "unknown",
  "unsupported",
  "healthy",
];

const STATUS_META: Record<
  AssetHealthStatus,
  { label: string; className: string; Icon: typeof CheckCircle2 }
> = {
  healthy: {
    label: "符合验证策略",
    className: "text-emerald-700 dark:text-emerald-300",
    Icon: CheckCircle2,
  },
  attention: {
    label: "待确认",
    className: "text-amber-700 dark:text-amber-300",
    Icon: AlertTriangle,
  },
  unhealthy: {
    label: "有问题",
    className: "text-red-700 dark:text-red-300",
    Icon: XCircle,
  },
  unknown: {
    label: "未检查",
    className: "text-muted-foreground",
    Icon: CircleHelp,
  },
  unsupported: {
    label: "不支持",
    className: "text-muted-foreground",
    Icon: CircleHelp,
  },
};

const REASON_LABELS: Record<string, string> = {
  unsupported_combination: "当前工具不支持",
  partial_support_unverified: "部分支持，尚待验证",
  deployment_failed: "写入失败",
  unmanaged_file_protected: "已保护用户文件，未覆盖",
  verification_failed: "配置验证失败",
  evidence_expired: "验证证据已过期",
  runtime_verified: "目标应用已读取并生效",
  verification_policy_satisfied: "配置解析通过",
  config_verified_runtime_pending: "配置正常，尚无运行时证据",
  deployment_unverified: "已写入，尚待验证",
  deployment_interrupted: "上次同步异常中断，请检查文件",
  deployment_rolled_back: "已回滚，需要重新生成同步计划",
  asset_revision_changed: "资产已有新修订，需要重新同步",
  adapter_execution_failed: "写回适配器执行失败",
  not_scanned: "尚未生成同步计划",
};

const EVIDENCE_LABELS: Record<string, string> = {
  none: "无证据",
  planned: "计划/回执",
  written: "已写入",
  verification: "验证结果",
  config_parsed: "配置已解析",
  runtime_verified: "运行时已验证",
  manual_confirmed: "人工已确认",
};

const ASSET_TYPE_LABELS: Record<string, string> = {
  mcp: "MCP 服务",
  skill: "技能",
  prompt: "提示词",
  command: "命令",
  hook: "钩子",
  ignore: "忽略规则",
  permission: "权限",
  subagent: "子代理",
};

const SOURCE_LABELS: Record<string, string> = {
  manual: "手动关联",
  observed: "历史关联",
  migration: "迁移导入",
};

export function ProjectAssetHealthSummary({
  projectId,
}: ProjectAssetHealthSummaryProps) {
  const [records, setRecords] = useState<AssetHealthRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [plan, setPlan] = useState<AssetHealthPlan | null>(null);
  const [applying, setApplying] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setRecords(await projectsApi.getAssetHealth(projectId));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  const previewPlan = useCallback(async () => {
    setMessage(null);
    try {
      setPlan(await projectsApi.planAssetHealth(projectId));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }, [projectId]);

  const applyPlan = useCallback(async () => {
    if (!plan) return;
    if (
      !window.confirm(
        "确认按当前预览同步项目 AI 资产？受保护的用户文件不会被覆盖。",
      )
    ) {
      return;
    }
    setApplying(true);
    setMessage(null);
    try {
      const receipts = await projectsApi.applyAssetHealthPlan(
        projectId,
        plan.planSha256,
      );
      setMessage(`同步完成，已生成 ${receipts.length} 条可追溯回执。`);
      setPlan(null);
      await refresh();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setApplying(false);
    }
  }, [plan, projectId, refresh]);

  const rollback = useCallback(
    async (receiptId: string) => {
      if (
        !window.confirm(
          "确认回滚该回执写入的受管文件？若文件已被人工修改，系统会拒绝回滚。",
        )
      ) {
        return;
      }
      setApplying(true);
      setMessage(null);
      try {
        await projectsApi.rollbackAssetHealthReceipt(receiptId);
        setMessage("回滚完成，已保留新的回滚回执。");
        await refresh();
      } catch (error) {
        setMessage(error instanceof Error ? error.message : String(error));
      } finally {
        setApplying(false);
      }
    },
    [refresh],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const counts = useMemo(
    () =>
      Object.fromEntries(
        STATUS_ORDER.map((status) => [
          status,
          records.filter((record) => record.status === status).length,
        ]),
      ) as Record<AssetHealthStatus, number>,
    [records],
  );

  return (
    <section className="border-b border-border/60 pb-5" aria-label="资产健康">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold">资产健康</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            按项目验证策略区分“配置可解析”与“目标应用已读取并生效”。
          </p>
        </div>
        <div className="flex items-center gap-1.5">
          <Button
            type="button"
            size="sm"
            variant="outline"
            className="h-8"
            onClick={() => void previewPlan()}
            disabled={loading || applying || records.length === 0}
          >
            <ClipboardCheck className="mr-1.5 h-3.5 w-3.5" />
            预览同步计划
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="h-8 w-8"
            onClick={() => void refresh()}
            disabled={loading || applying}
            title="刷新资产健康"
            aria-label="刷新资产健康"
          >
            <RefreshCw
              className={loading ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"}
            />
          </Button>
        </div>
      </div>
      <div className="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-xs">
        {STATUS_ORDER.filter((status) => counts[status] > 0).map((status) => {
          const { Icon, label, className } = STATUS_META[status];
          return (
            <span
              key={status}
              className={`inline-flex items-center gap-1 ${className}`}
            >
              <Icon className="h-3.5 w-3.5" />
              {label} {counts[status]}
            </span>
          );
        })}
        {!loading && records.length === 0 && (
          <span className="text-muted-foreground">
            尚未关联已启用的项目资产
          </span>
        )}
      </div>
      {message && (
        <p
          className="mt-3 rounded-md border border-border/60 bg-muted/30 px-3 py-2 text-xs"
          role="status"
        >
          {message}
        </p>
      )}
      {plan && (
        <div className="mt-3 rounded-md border border-blue-500/40 bg-blue-500/5 p-3 text-xs">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="font-semibold">同步计划（预览阶段，尚未写入）</p>
              <p className="mt-1 text-muted-foreground">
                {
                  plan.steps.filter(
                    (step) => step.action === "legacy_project_sync",
                  ).length
                }{" "}
                项将调用适配器，
                {
                  plan.steps.filter(
                    (step) => step.action !== "legacy_project_sync",
                  ).length
                }{" "}
                项因不支持或文件保护而跳过。
              </p>
            </div>
            <Button
              type="button"
              size="sm"
              onClick={() => void applyPlan()}
              disabled={applying}
            >
              {applying ? "执行中…" : "确认并执行"}
            </Button>
          </div>
          <div className="mt-2 max-h-40 space-y-1 overflow-auto">
            {plan.steps.map((step) => (
              <div
                key={step.expectationId}
                className="flex items-center justify-between gap-3 rounded bg-background/60 px-2 py-1.5"
              >
                <span className="font-medium">
                  {ASSET_TYPE_LABELS[step.assetType] ?? step.assetType} →{" "}
                  {step.targetApp}
                </span>
                <span className="text-right text-muted-foreground">
                  {step.action === "skip_unsupported"
                    ? "当前工具不支持，跳过"
                    : step.action === "skip_protected"
                      ? `保护用户文件：${step.protectedPaths.join("、")}`
                      : `${step.managedPaths.join("、") || "受管配置路径"} · ${step.verifyModes.includes("config_parse") ? "写后解析" : "写后核验"}`}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
      {records.length > 0 && (
        <div className="mt-3 overflow-hidden rounded-md border border-border/60 text-xs">
          <div className="grid grid-cols-[minmax(130px,1fr)_100px_minmax(180px,1.4fr)_110px_76px] gap-2 bg-muted/40 px-3 py-2 text-muted-foreground">
            <span>来源与期望</span>
            <span>目标应用</span>
            <span>验证结论</span>
            <span>写入结果</span>
            <span>操作</span>
          </div>
          {records.slice(0, 12).map((record) => (
            <div
              key={record.expectation.expectationId}
              className="grid grid-cols-[minmax(130px,1fr)_100px_minmax(180px,1.4fr)_110px_76px] items-center gap-2 border-t border-border/50 px-3 py-2"
            >
              <span className="min-w-0">
                <span
                  className="block truncate font-medium"
                  title={record.expectation.assetId}
                >
                  {ASSET_TYPE_LABELS[record.expectation.assetType] ??
                    record.expectation.assetType}{" "}
                  · {record.expectation.assetId}
                </span>
                <span className="block text-muted-foreground">
                  {SOURCE_LABELS[record.expectation.source] ?? "项目关联"} ·
                  期望启用
                </span>
              </span>
              <span>{record.expectation.targetApp}</span>
              <span className="min-w-0">
                <span className="font-medium">
                  {EVIDENCE_LABELS[record.evidenceLevel] ??
                    record.evidenceLevel}
                </span>
                <span className="ml-2 text-muted-foreground">
                  {REASON_LABELS[record.reasonCode] ?? "需要查看详情"}
                </span>
              </span>
              <span
                title={record.lastReceiptFiles
                  .map((file) => `${file.action}: ${file.relativePath}`)
                  .join("\n")}
              >
                {record.lastReceiptFiles.length
                  ? `${record.lastReceiptFiles.length} 个文件`
                  : "无文件变更"}
              </span>
              <span>
                {record.lastReceiptId &&
                record.lastReceiptFiles.some((file) =>
                  ["create", "update", "delete"].includes(file.action),
                ) ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2"
                    onClick={() => void rollback(record.lastReceiptId!)}
                    disabled={applying}
                    title="按回执安全回滚"
                  >
                    <RotateCcw className="mr-1 h-3.5 w-3.5" />
                    回滚
                  </Button>
                ) : (
                  "—"
                )}
              </span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
