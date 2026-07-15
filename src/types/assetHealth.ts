import type { ProjectAssetType } from "./projectAsset";

export type AssetHealthStatus =
  | "healthy"
  | "attention"
  | "unhealthy"
  | "unknown"
  | "unsupported";

export interface AssetHealthExpectation {
  expectationId: string;
  projectId: string;
  assetType: ProjectAssetType;
  assetId: string;
  targetApp: string;
  desiredState: string;
  requiredRevisionId?: string | null;
  verificationPolicy?: string | null;
  scope: string;
  source: string;
  ownerMode: string;
}

export interface AssetReceiptFile {
  fileId: string;
  receiptId: string;
  relativePath: string;
  action:
    | "create"
    | "update"
    | "delete"
    | "unchanged"
    | "skipped_protected"
    | string;
  beforeSha256?: string | null;
  afterSha256?: string | null;
  snapshotRef?: string | null;
  reasonCode?: string | null;
  createdAt: number;
}

export interface AssetHealthRecord {
  expectation: AssetHealthExpectation;
  status: AssetHealthStatus;
  evidenceLevel: string;
  reasonCode: string;
  recommendedAction: string;
  lastReceiptId?: string | null;
  lastEvidenceId?: string | null;
  observedAt?: number | null;
  lastReceiptFiles: AssetReceiptFile[];
}

export interface AssetHealthPlanStep {
  expectationId: string;
  assetType: ProjectAssetType;
  assetId: string;
  requiredRevisionId?: string | null;
  targetApp: string;
  action:
    | "legacy_project_sync"
    | "skip_unsupported"
    | "skip_protected"
    | string;
  reasonCode: string;
  adapterId: string;
  writeMode: string;
  verifyModes: string[];
  limitations: string[];
  managedPaths: string[];
  protectedPaths: string[];
}

export interface AssetHealthPlan {
  operationId: string;
  projectId: string;
  planSha256: string;
  steps: AssetHealthPlanStep[];
}

export interface AssetDeploymentReceipt {
  receiptId: string;
  expectationId: string;
  operationId: string;
  adapterId: string;
  adapterVersion: string;
  planSha256: string;
  requiredRevisionId?: string | null;
  dryRun: boolean;
  outcome: string;
  reasonCode?: string | null;
  createdAt: number;
}
