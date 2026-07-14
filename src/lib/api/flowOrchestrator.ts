import { invoke } from "@tauri-apps/api/core";
import type { InstallAuditSummary, InstallFileEntry } from "./designContract";

export interface WorkflowModule {
  id: string;
  name: string;
  nameZh?: string;
  source: string;
  description: string;
  capabilities: string[];
}

export interface WorkflowPresetSummary {
  id: string;
  name: string;
  nameZh?: string;
  description: string;
  r3Tier?: string;
  moduleCount: number;
  stageCount: number;
}

export interface WorkflowStage {
  id: string;
  name: string;
  prompt?: string;
  dependsOn?: string[];
  artifacts?: Array<{
    file: string;
    scope: string;
    optional?: boolean;
  }>;
}

export interface WorkflowPreset {
  id: string;
  name: string;
  nameZh?: string;
  description: string;
  r3Tier?: string;
  modules: string[];
  stages: WorkflowStage[];
  paths: Record<string, string[]>;
}

export interface FlowConfigGate {
  type: string;
  artifacts: string[];
}

export interface FlowConfigStage {
  id: string;
  enabled: boolean;
  depends_on?: string[];
  gates?: FlowConfigGate[];
}

export interface FlowConfigRules {
  max_auto_retry: number;
  role_separation: boolean;
  require_diff_boundary: boolean;
}

export interface FlowConfigReviewTier {
  enabled: boolean;
  lenses: string[];
  max_lenses?: number | null;
  default_lens?: string | null;
  paths?: string[];
  changed_lines_threshold?: number | null;
}

export interface FlowConfigReviewPolicy {
  mode: string;
  trivial: FlowConfigReviewTier;
  standard: FlowConfigReviewTier;
  hot_path: FlowConfigReviewTier;
  large_diff: FlowConfigReviewTier;
}

export interface FlowConfig {
  schema_version: number;
  preset_id: string;
  project_type: string;
  modules: string[];
  stages: FlowConfigStage[];
  rules: FlowConfigRules;
  review_policy: FlowConfigReviewPolicy;
  semantic_warnings?: string[];
}

export interface WorkflowProfile {
  schemaVersion: number;
  presetId: string;
  projectType: string;
  modules: string[];
  resolvedStages: string[];
  activeChangeId?: string | null;
  exportedAt: string;
  opensunstarVersion?: string;
  semanticWarnings?: string[];
}

export interface ArtifactStatus {
  file: string;
  relativePath: string;
  exists: boolean;
  nonEmpty: boolean;
  optional: boolean;
}

export interface TaskSummary {
  total: number;
  pending: number;
  inProgress: number;
  done: number;
  blocked: number;
}

export interface SpecsChangeIndex {
  changeId: string;
  artifactCompleteness: number;
  artifacts: ArtifactStatus[];
  taskSummary?: TaskSummary | null;
}

export interface SpecsWorkflowIndex {
  projectPath: string;
  workspaceExists: boolean;
  hasFlowKit: boolean;
  hasFlowConfig: boolean;
  hasSpecsDir: boolean;
  activeChangeId?: string | null;
  savedProfile?: WorkflowProfile | null;
  changes: SpecsChangeIndex[];
}

export interface StageGateResult {
  allowed: boolean;
  targetStage: string;
  changeId: string;
  missingArtifacts: string[];
  satisfiedArtifacts: string[];
  warnings: string[];
}

export interface FlowWritePlan {
  files: InstallFileEntry[];
  audit: InstallAuditSummary;
  semanticWarnings: string[];
}

export interface OrchestrationLogEntry {
  ts?: string | null;
  event: string;
  summary: string;
  payload: Record<string, unknown>;
}

export type OrchestrationStepStatus = "planned" | "applied" | "skipped" | "verified" | "failed";

export interface OrchestrationStepReceipt {
  id: string;
  label: string;
  targetPath: string;
  action: string;
  status: OrchestrationStepStatus;
  beforeChecksum?: string | null;
  afterChecksum?: string | null;
  snapshotPath?: string | null;
  reason?: string | null;
}

export interface OrchestrationVerification {
  id: string;
  label: string;
  passed: boolean;
  detail?: string | null;
}

export interface OrchestrationReceipt {
  schema: string;
  operation: string;
  projectPath: string;
  dryRun: boolean;
  createdAt: string;
  steps: OrchestrationStepReceipt[];
  verifications: OrchestrationVerification[];
}

export const flowOrchestratorApi = {
  async listModules(projectId?: string): Promise<WorkflowModule[]> {
    return await invoke<WorkflowModule[]>("list_workflow_modules_cmd", {
      projectId: projectId ?? null,
    });
  },

  async listPresets(projectId?: string): Promise<WorkflowPresetSummary[]> {
    return await invoke<WorkflowPresetSummary[]>("list_workflow_presets_cmd", {
      projectId: projectId ?? null,
    });
  },

  async getPreset(id: string, projectId?: string): Promise<WorkflowPreset> {
    return await invoke<WorkflowPreset>("get_workflow_preset_cmd", {
      id,
      projectId: projectId ?? null,
    });
  },

  async scanProject(
    projectId: string,
    presetId?: string,
    projectType?: string,
  ): Promise<SpecsWorkflowIndex> {
    return await invoke<SpecsWorkflowIndex>("scan_project_specs_workflow_cmd", {
      projectId,
      presetId: presetId ?? null,
      projectType: projectType ?? null,
    });
  },

  async validateStageGate(
    projectId: string,
    params: {
      presetId: string;
      projectType: string;
      changeId: string;
      targetStage: string;
    },
  ): Promise<StageGateResult> {
    return await invoke<StageGateResult>("validate_workflow_stage_gate_cmd", {
      projectId,
      ...params,
    });
  },

  async exportProfile(
    projectId: string,
    presetId: string,
    projectType: string,
    activeChangeId?: string,
    selectedModules?: string[],
    disabledStages?: string[],
    strictSemantics = true,
  ): Promise<WorkflowProfile> {
    return await invoke<WorkflowProfile>("export_project_workflow_profile_cmd", {
      projectId,
      presetId,
      projectType,
      activeChangeId: activeChangeId ?? null,
      selectedModules: selectedModules ?? null,
      disabledStages: disabledStages ?? null,
      strictSemantics,
    });
  },

  async previewProfileExport(
    projectId: string,
    presetId: string,
    projectType: string,
    activeChangeId?: string,
    selectedModules?: string[],
    disabledStages?: string[],
  ): Promise<FlowWritePlan> {
    return await invoke<FlowWritePlan>("preview_project_workflow_profile_export_cmd", {
      projectId,
      presetId,
      projectType,
      activeChangeId: activeChangeId ?? null,
      selectedModules: selectedModules ?? null,
      disabledStages: disabledStages ?? null,
    });
  },

  async exportFlowConfig(
    projectId: string,
    presetId: string,
    projectType: string,
    selectedModules?: string[],
    disabledStages?: string[],
    strictSemantics = true,
  ): Promise<FlowConfig> {
    return await invoke<FlowConfig>("export_flow_config_cmd", {
      projectId,
      presetId,
      projectType,
      selectedModules: selectedModules ?? null,
      disabledStages: disabledStages ?? null,
      strictSemantics,
    });
  },

  async previewFlowConfigExport(
    projectId: string,
    presetId: string,
    projectType: string,
    selectedModules?: string[],
    disabledStages?: string[],
  ): Promise<FlowWritePlan> {
    return await invoke<FlowWritePlan>("preview_flow_config_export_cmd", {
      projectId,
      presetId,
      projectType,
      selectedModules: selectedModules ?? null,
      disabledStages: disabledStages ?? null,
    });
  },

  async readOrchestrationLog(
    projectId: string,
    limit = 20,
  ): Promise<OrchestrationLogEntry[]> {
    return await invoke<OrchestrationLogEntry[]>("read_orchestration_log_cmd", {
      projectId,
      limit,
    });
  },

  async restoreLatestReceipt(projectId: string): Promise<OrchestrationReceipt> {
    return await invoke<OrchestrationReceipt>("restore_latest_orchestration_receipt_cmd", {
      projectId,
    });
  },
};
