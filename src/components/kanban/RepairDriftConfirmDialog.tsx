import { useTranslation } from "react-i18next";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { GOVERNANCE_CHECK_LABELS } from "@/lib/governanceStats";

export interface RepairDriftAssetConfirm {
  kind: "asset";
  checkName: string;
  label?: string;
  effectiveDetail?: string | null;
  livePath?: string | null;
  targetApp?: string | null;
}

export interface RepairDriftProjectConfirm {
  kind: "project";
  projectName: string;
  driftCount: number;
  targetApp?: string | null;
}

export type RepairDriftConfirmPayload =
  | RepairDriftAssetConfirm
  | RepairDriftProjectConfirm
  | null;

export interface RepairDriftConfirmDialogProps {
  pending: RepairDriftConfirmPayload;
  onConfirm: () => void;
  onCancel: () => void;
  zIndex?: "base" | "nested" | "alert" | "top";
}

function assetLabel(payload: RepairDriftAssetConfirm): string {
  return payload.label ?? GOVERNANCE_CHECK_LABELS[payload.checkName] ?? payload.checkName;
}

export function RepairDriftConfirmDialog({
  pending,
  onConfirm,
  onCancel,
  zIndex = "top",
}: RepairDriftConfirmDialogProps) {
  const { t } = useTranslation();

  if (!pending) return null;

  if (pending.kind === "asset") {
    const label = assetLabel(pending);
    const lines = [
      t("kanban.governance.repairConfirmAssetIntro", {
        label,
        app: pending.targetApp ?? "claude",
        defaultValue:
          "将把 OpenSunstar 库中的「{{label}}」写回目标 CLI（{{app}}）对应的项目级文件，可能覆盖你在 IDE/终端里手动改过的内容。",
      }),
      pending.effectiveDetail
        ? t("kanban.governance.repairConfirmDetail", {
            detail: pending.effectiveDetail,
            defaultValue: "不一致说明：{{detail}}",
          })
        : null,
      pending.livePath
        ? t("kanban.governance.repairConfirmPath", {
            path: pending.livePath,
            defaultValue: "目标路径：{{path}}",
          })
        : null,
      t("kanban.governance.repairConfirmProceed", {
        defaultValue: "确认后将立即写回并复扫验证；此操作不可自动撤销。",
      }),
    ]
      .filter(Boolean)
      .join("\n\n");

    return (
      <ConfirmDialog
        isOpen
        variant="destructive"
        zIndex={zIndex}
        title={t("kanban.governance.repairConfirmTitle", {
          defaultValue: "确认写回修复？",
        })}
        message={lines}
        confirmText={t("kanban.governance.repairConfirmAction", {
          defaultValue: "确认写回",
        })}
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );
  }

  const message = [
    t("kanban.governance.repairConfirmProjectIntro", {
      name: pending.projectName,
      count: pending.driftCount,
      app: pending.targetApp ?? "claude",
      defaultValue:
        "将修复项目「{{name}}」的全部 {{count}} 项配置不一致：逐项写回 OpenSunstar 库内容到目标 CLI（{{app}}），可能覆盖外部手动修改。",
    }),
    t("kanban.governance.repairConfirmProceed", {
      defaultValue: "确认后将立即写回并复扫验证；此操作不可自动撤销。",
    }),
  ].join("\n\n");

  return (
    <ConfirmDialog
      isOpen
      variant="destructive"
      zIndex={zIndex}
      title={t("kanban.governance.repairConfirmProjectTitle", {
        defaultValue: "确认修复全部不一致项？",
      })}
      message={message}
      confirmText={t("kanban.governance.repairConfirmProjectAction", {
        defaultValue: "确认修复全部",
      })}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}
