/**
 * Team Config API — Git MVP Local Alpha 只读命令
 *
 * 对应 Rust 端 commands/team_config.rs 的 6 个 Tauri 命令。
 */
import { invoke } from "@tauri-apps/api/core";

// ─── Types ──────────────────────────────────────────────────────────────────

export interface TeamConnectResponse {
  workspaceId: string;
  name: string;
  sourceKind: string;
  sourcePath: string;
  branch: string | null;
  headCommit: string | null;
  profilesCount: number;
  policiesCount: number;
  credentialSlotsCount: number;
  warnings: string[];
}

export interface TeamValidationIssue {
  code: string;
  message: string;
  location: string | null;
}

export interface TeamValidateResponse {
  passed: boolean;
  errors: TeamValidationIssue[];
  warnings: TeamValidationIssue[];
  securityBlocked: boolean;
  filesScanned: number | null;
}

export interface TeamProfileSummary {
  profileId: string;
  name: string;
  description: string | null;
  assetsCount: number;
  credentialSlotsCount: number;
}

export interface EffectiveItem {
  assetType: string;
  assetId: string;
  decision: "enabled" | "denied" | "skipped" | "conflicted";
  provenance: ProvenanceEntry[];
}

export interface ProvenanceEntry {
  tier: string;
  sourceId: string;
  action: string;
  explanation: string;
}

export interface EffectiveConflict {
  assetType: string;
  assetId: string;
  code: string;
  sourceIds: string[];
  message: string;
}

export interface RequiredCredential {
  slotId: string;
  kind: string;
}

export interface EffectiveConfig {
  targetApp: string;
  projectId: string;
  configSha256: string;
  items: EffectiveItem[];
  conflicts: EffectiveConflict[];
  requiredCredentials: RequiredCredential[];
}

export interface TeamStatusResponse {
  connected: boolean;
  workspaceId: string | null;
  name: string | null;
  sourceKind: string | null;
  sourcePath: string | null;
  branch: string | null;
  headCommit: string | null;
  isClean: boolean | null;
  canPull: boolean | null;
  profilesCount: number;
  validationPassed: boolean | null;
}

export interface DiffEntry {
  path: string;
  action: "added" | "removed" | "modified" | "unchanged";
  assetType: string | null;
  oldSha256: string | null;
  newSha256: string | null;
  oldSize: number | null;
  newSize: number | null;
}

export interface DiffSummary {
  totalFilesBase: number;
  totalFilesTarget: number;
  addedCount: number;
  removedCount: number;
  modifiedCount: number;
  unchangedCount: number;
  hasChanges: boolean;
}

export interface ReleaseDiff {
  baseRef: string;
  targetRef: string;
  added: DiffEntry[];
  removed: DiffEntry[];
  modified: DiffEntry[];
  unchangedCount: number;
  summary: DiffSummary;
}

// ─── Git MVP Write Loop Types ──────────────────────────────────────────────

export interface DeploymentStep {
  assetType: string;
  assetId: string;
  action: "create" | "update" | "remove" | "skip" | "displayOnly";
  riskLevel: "safe" | "low" | "medium" | "high" | "requiresTrust";
  targetPath: string;
  desiredSha256: string | null;
  currentSha256: string | null;
  explanation: string;
}

export interface PlanSummary {
  totalAssets: number;
  createCount: number;
  updateCount: number;
  removeCount: number;
  skipCount: number;
  displayOnlyCount: number;
  writeCount: number;
  hasHighRisk: boolean;
}

export interface PlanWarning {
  assetType: string;
  assetId: string;
  code: string;
  message: string;
}

export interface DeploymentPlan {
  projectId: string;
  targetApp: string;
  steps: DeploymentStep[];
  summary: PlanSummary;
  warnings: PlanWarning[];
  planSha256: string;
}

export interface StepReceipt {
  assetType: string;
  assetId: string;
  action: string;
  targetPath: string;
  success: boolean;
  postWriteSha256: string | null;
  backupPath: string | null;
  error: string | null;
}

export interface ReceiptSummary {
  totalSteps: number;
  successCount: number;
  failureCount: number;
  skippedCount: number;
  allSuccess: boolean;
}

export interface DeploymentReceipt {
  projectId: string;
  targetApp: string;
  planSha256: string;
  steps: StepReceipt[];
  summary: ReceiptSummary;
  executedAt: number;
}

export interface DriftEntry {
  assetType: string;
  assetId: string;
  targetPath: string;
  status: "clean" | "modified" | "deleted" | "added" | "unknown";
  expectedSha256: string | null;
  actualSha256: string | null;
  hasBackup: boolean;
}

export interface DriftSummary {
  totalChecked: number;
  cleanCount: number;
  driftedCount: number;
  unknownCount: number;
  hasDrift: boolean;
  rollbackEligibleCount: number;
}

export interface DriftReport {
  projectId: string;
  planSha256: string;
  checkedAt: number;
  entries: DriftEntry[];
  summary: DriftSummary;
}

export interface RollbackStepResult {
  assetType: string;
  assetId: string;
  targetPath: string;
  success: boolean;
  restoredSha256: string | null;
  error: string | null;
}

export interface RollbackSummary {
  totalAttempted: number;
  successCount: number;
  failureCount: number;
  skippedNoBackup: number;
  allSuccess: boolean;
}

export interface RollbackReport {
  projectId: string;
  planSha256: string;
  rolledBackAt: number;
  steps: RollbackStepResult[];
  summary: RollbackSummary;
}

// ─── API ────────────────────────────────────────────────────────────────────

export const teamConfigApi = {
  connect(path: string): Promise<TeamConnectResponse> {
    return invoke("connect_team_workspace", { path });
  },

  validate(
    path: string,
    runSecurityScan?: boolean,
  ): Promise<TeamValidateResponse> {
    return invoke("validate_team_workspace", { path, runSecurityScan });
  },

  listProfiles(path: string): Promise<TeamProfileSummary[]> {
    return invoke("list_team_profiles", { path });
  },

  getEffectiveState(
    path: string,
    targetApp: string,
    projectId?: string,
  ): Promise<EffectiveConfig> {
    return invoke("get_team_effective_state", { path, targetApp, projectId });
  },

  getStatus(path: string): Promise<TeamStatusResponse> {
    return invoke("get_team_status", { path });
  },

  getReleaseDiff(path: string): Promise<ReleaseDiff> {
    return invoke("get_team_release_diff", { path });
  },

  // ─── Git MVP Write Loop ───────────────────────────────────────────────────

  generatePlan(
    path: string,
    targetApp: string,
    projectRoot: string,
    projectId?: string,
  ): Promise<DeploymentPlan> {
    return invoke("generate_team_deployment_plan", {
      path,
      targetApp,
      projectRoot,
      projectId,
    });
  },

  executeDeployment(
    path: string,
    targetApp: string,
    projectRoot: string,
    projectId?: string,
    dryRun?: boolean,
  ): Promise<DeploymentReceipt> {
    return invoke("execute_team_deployment", {
      path,
      targetApp,
      projectRoot,
      projectId,
      dryRun,
    });
  },

  checkDrift(receiptJson: string, projectRoot: string): Promise<DriftReport> {
    return invoke("check_team_drift", { receiptJson, projectRoot });
  },

  rollback(
    receiptJson: string,
    driftJson: string,
    projectRoot: string,
  ): Promise<RollbackReport> {
    return invoke("rollback_team_deployment", {
      receiptJson,
      driftJson,
      projectRoot,
    });
  },
};
