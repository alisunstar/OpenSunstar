import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";

import type { Project } from "@/types/project";
import type { AgentReadinessBatchEntry } from "@/lib/readinessBatch";
import type { ProjectAiConfigNavigationIntent } from "@/app/navigation";
import { classifyReadinessLevel } from "@/lib/portfolioHealth";

/**
 * 「今日」告警流（工作区重构 2026-07-30）。
 *
 * 三类告警一个出口：
 * - 命（life）  ：代理故障转移事件（Rust `failover-event`，P1）——会丢上下文、
 *   烧进度的事，永远排最前；
 * - 钱（money） ：预算超限（现有 `budget-alert` 引擎）——以前只在窗口开着时
 *   弹 toast，窗口关了归零；这里落进首屏卡片，关了再开也看得见；
 * - 事（task）  ：配置缺口/漂移（readiness 派生态）——按「最近使用加权」排序，
 *   三个月没动的项目不给今天的你添堵。
 *
 * 事件类（life/money）来自 Rust 事件流，用 localStorage 落盘最近 20 条 ——
 * 否则窗口没开时发生过的故障转移，用户永远不知道它发生过。
 */

export type WorkspaceAlertKind = "life" | "money" | "task";
export type WorkspaceAlertSeverity = "critical" | "warning" | "info";

export interface WorkspaceAlertAction {
  label: string;
  onClick: () => void;
  /** 深链展示用（可选）：渲染成次级说明，例如「→ 项目资产配置 · 就绪与生效」。 */
  hint?: string;
}

export interface WorkspaceAlert {
  id: string;
  kind: WorkspaceAlertKind;
  severity: WorkspaceAlertSeverity;
  title: string;
  description: string;
  occurredAt: number;
  action?: WorkspaceAlertAction;
}

interface BudgetAlertPayload {
  providerId: string;
  appType: string;
  providerName: string;
  alertLevel: "warning" | "critical" | "emergency";
  period: string;
  usageUsd: number;
  limitUsd: number;
  percentage: number;
}

interface FailoverEventPayload {
  appType: string;
  fromProviderId: string | null;
  fromProviderName: string | null;
  toProviderId: string;
  toProviderName: string;
  at: number;
}

const STORAGE_KEY = "OpenSunstar-workspace-alerts";
const MAX_PERSISTED = 20;
/** 同一来源（kind+id 前缀）在这个窗口内只保留最新一条，避免刷屏。 */
const DEDUPE_WINDOW_MS = 10 * 60 * 1000;

interface PersistedAlert {
  id: string;
  kind: WorkspaceAlertKind;
  severity: WorkspaceAlertSeverity;
  title: string;
  description: string;
  occurredAt: number;
  /** 事件类告警的动作是固定的（跳 AI Tokens / 留在今日），只存类型。 */
  actionType?: "openTokenStats" | null;
}

function loadPersisted(): PersistedAlert[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as PersistedAlert[]) : [];
  } catch {
    return [];
  }
}

function savePersisted(alerts: PersistedAlert[]): void {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify(alerts.slice(0, MAX_PERSISTED)),
    );
  } catch {
    /* ignore */
  }
}

export interface UseWorkspaceAlertsOptions {
  projects: Project[];
  agentReadinessMap: Map<string, AgentReadinessBatchEntry>;
  /** 近 N 天提交数（今日 Tab 的窗口口径），用作「最近使用」权重。 */
  commitsInWindowMap: Map<string, number>;
  /** 修复漂移入口（KanbanPage 的预览-确认链路）。 */
  onRepairProject: (project: Project) => void;
  /** 深链「项目资产配置」。 */
  onOpenProjectAiConfig: (
    projectId: string,
    intent?: Pick<ProjectAiConfigNavigationIntent, "tab" | "section"> | null,
  ) => void;
  /** 跳 AI Tokens 页（预算设置在那）。 */
  onOpenTokenStats: () => void;
  /** 正在修复中的项目 id（防重复提交）。 */
  repairingProjectId?: string | null;
}

export function useWorkspaceAlerts({
  projects,
  agentReadinessMap,
  commitsInWindowMap,
  onRepairProject,
  onOpenProjectAiConfig,
  onOpenTokenStats,
}: UseWorkspaceAlertsOptions) {
  const { t } = useTranslation();
  const [eventAlerts, setEventAlerts] = useState<PersistedAlert[]>(() =>
    loadPersisted(),
  );

  const pushEventAlert = useCallback((next: PersistedAlert) => {
    setEventAlerts((prev) => {
      // 同源去重：kind+去时间后缀的 id 相同，且在时间窗内，只留最新
      const baseId = next.id.replace(/@\d+$/, "");
      const filtered = prev.filter((a) => {
        const aBase = a.id.replace(/@\d+$/, "");
        if (aBase !== baseId) return true;
        return Math.abs(a.occurredAt - next.occurredAt) > DEDUPE_WINDOW_MS;
      });
      const merged = [next, ...filtered].slice(0, MAX_PERSISTED);
      savePersisted(merged);
      return merged;
    });
  }, []);

  useEffect(() => {
    const unlistenBudget = listen<BudgetAlertPayload>(
      "budget-alert",
      (event) => {
        const a = event.payload;
        const periodLabel = a.period === "daily" ? "日" : "月";
        const levelText =
          a.alertLevel === "emergency"
            ? t("alerts.money.emergency", { defaultValue: "严重超限" })
            : a.alertLevel === "critical"
              ? t("alerts.money.critical", { defaultValue: "预算超限" })
              : t("alerts.money.warning", { defaultValue: "预算预警" });
        pushEventAlert({
          id: `money:${a.providerId}:${a.period}@${Date.now()}`,
          kind: "money",
          severity: a.alertLevel === "warning" ? "warning" : "critical",
          title: t("alerts.money.title", {
            level: levelText,
            defaultValue: `钱 · ${a.providerName} ${periodLabel}用量${levelText}`,
          }),
          description: t("alerts.money.desc", {
            pct: a.percentage.toFixed(0),
            used: a.usageUsd.toFixed(2),
            limit: a.limitUsd.toFixed(2),
            period: periodLabel,
            defaultValue: `${periodLabel}用量已达 ${a.percentage.toFixed(0)}%（$${a.usageUsd.toFixed(2)} / $${a.limitUsd.toFixed(2)}）`,
          }),
          occurredAt: Date.now(),
          actionType: "openTokenStats",
        });
      },
    );
    const unlistenFailover = listen<FailoverEventPayload>(
      "failover-event",
      (event) => {
        const f = event.payload;
        const from = f.fromProviderName ?? "?";
        pushEventAlert({
          id: `life:${f.appType}:${f.toProviderId}@${f.at}`,
          kind: "life",
          severity: "critical",
          title: t("alerts.life.title", {
            to: f.toProviderName,
            defaultValue: `命 · ${from} 已熔断，自动切换到 ${f.toProviderName}`,
          }),
          description: t("alerts.life.desc", {
            app: f.appType,
            defaultValue: `${f.appType} 的代理故障转移已接管，进行中的任务未中断`,
          }),
          occurredAt: f.at * 1000,
          actionType: null,
        });
      },
    );
    return () => {
      unlistenBudget.then((fn) => fn());
      unlistenFailover.then((fn) => fn());
    };
  }, [pushEventAlert, t]);

  /** 事（task）：readiness 派生态。口径与 PortfolioHealthSummary 六档分级一致。 */
  const taskAlerts = useMemo<WorkspaceAlert[]>(() => {
    const alerts: WorkspaceAlert[] = [];
    for (const project of projects) {
      const readiness = agentReadinessMap.get(project.id);
      const level = classifyReadinessLevel(readiness);
      const commits = commitsInWindowMap.get(project.id) ?? 0;

      if (level === "alert") {
        const driftCount = readiness?.driftCount ?? 0;
        alerts.push({
          id: `task:drift:${project.id}`,
          kind: "task",
          severity: "critical",
          title: t("alerts.task.driftTitle", {
            name: project.name,
            defaultValue: `事 · ${project.name} 有 ${driftCount} 处配置漂移`,
          }),
          description: t("alerts.task.driftDesc", {
            count: driftCount,
            defaultValue: `配置与预期不一致，可预览后一键修复`,
          }),
          occurredAt: 0,
          action: {
            label: t("alerts.action.repair", { defaultValue: "去修复" }),
            onClick: () => onRepairProject(project),
          },
        });
      } else if (level === "warn") {
        const gaps = (readiness?.details ?? []).filter(
          (d) => d.score < d.weight,
        ).length;
        alerts.push({
          id: `task:gap:${project.id}`,
          kind: "task",
          severity: "warning",
          title: t("alerts.task.gapTitle", {
            name: project.name,
            defaultValue: `事 · ${project.name} 有 ${gaps} 项资产缺口`,
          }),
          description:
            commits > 0
              ? t("alerts.task.gapDescActive", {
                  count: commits,
                  defaultValue: `近窗口内有 ${commits} 次提交——在用的项目，优先补齐`,
                })
              : t("alerts.task.gapDesc", {
                  defaultValue: "就绪检查未满分，去「项目资产配置」补齐",
                }),
          occurredAt: 0,
          action: {
            label: t("alerts.action.configure", { defaultValue: "去配置" }),
            onClick: () =>
              onOpenProjectAiConfig(project.id, { tab: "readiness" }),
            hint: t("alerts.action.hintReadiness", {
              defaultValue: "→ 项目资产配置 · 就绪与生效",
            }),
          },
        });
      } else if (level === "unconfigured" && commits > 0) {
        // 尚未配置但最近在用：给一次温和提醒；不在用的不上卡（狼来了防线）
        alerts.push({
          id: `task:unconfigured:${project.id}`,
          kind: "task",
          severity: "info",
          title: t("alerts.task.unconfiguredTitle", {
            name: project.name,
            defaultValue: `事 · ${project.name} 尚未关联任何 AI 资产`,
          }),
          description: t("alerts.task.unconfiguredDesc", {
            count: commits,
            defaultValue: `近窗口内有 ${commits} 次提交，但还没通过 OpenSunstar 关联资产`,
          }),
          occurredAt: 0,
          action: {
            label: t("alerts.action.setup", { defaultValue: "去配置" }),
            onClick: () => onOpenProjectAiConfig(project.id, { tab: "assets" }),
            hint: t("alerts.action.hintAssets", {
              defaultValue: "→ 项目资产配置 · 资产关联",
            }),
          },
        });
      }
    }

    const severityRank = { critical: 0, warning: 1, info: 2 } as const;
    return alerts
      .sort((a, b) => {
        const bySeverity = severityRank[a.severity] - severityRank[b.severity];
        if (bySeverity !== 0) return bySeverity;
        // 最近使用加权：提交多的在前（从 id 反查项目）
        const commitsOf = (alert: WorkspaceAlert) => {
          const projectId = alert.id.split(":").pop() ?? "";
          return commitsInWindowMap.get(projectId) ?? 0;
        };
        return commitsOf(b) - commitsOf(a);
      })
      .slice(0, 5);
  }, [
    projects,
    agentReadinessMap,
    commitsInWindowMap,
    onOpenProjectAiConfig,
    onRepairProject,
    t,
  ]);

  const eventAlertList = useMemo<WorkspaceAlert[]>(
    () =>
      eventAlerts.map((a) => ({
        ...a,
        action:
          a.actionType === "openTokenStats"
            ? {
                label: t("alerts.action.budget", { defaultValue: "调整预算" }),
                onClick: onOpenTokenStats,
              }
            : undefined,
      })),
    [eventAlerts, onOpenTokenStats, t],
  );

  const alerts = useMemo<WorkspaceAlert[]>(() => {
    const kindRank = { life: 0, money: 1, task: 2 } as const;
    return [...eventAlertList, ...taskAlerts].sort(
      (a, b) => kindRank[a.kind] - kindRank[b.kind],
    );
  }, [eventAlertList, taskAlerts]);

  const dismissEventAlert = useCallback((id: string) => {
    setEventAlerts((prev) => {
      const next = prev.filter((a) => a.id !== id);
      savePersisted(next);
      return next;
    });
  }, []);

  return {
    alerts,
    hasAlerts: alerts.length > 0,
    lifeAlerts: alerts.filter((a) => a.kind === "life"),
    moneyAlerts: alerts.filter((a) => a.kind === "money"),
    taskAlerts,
    dismissEventAlert,
  };
}
