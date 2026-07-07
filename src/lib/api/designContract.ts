import { invoke } from "@tauri-apps/api/core";

// ────────────────────────── Design Contract Types ──────────────────────────

export interface FontSize {
  name: string;
  size: string;
  lineHeight: string;
}

export interface DesignColors {
  primary: string;
  primaryHover: string;
  background: string;
  surface: string;
  textPrimary: string;
  textMuted: string;
  accent: string;
  success: string;
  warning: string;
  error: string;
  border: string;
  custom: Record<string, string>;
}

export interface DesignTypography {
  fontFamilyBase: string;
  fontFamilyHeading: string;
  fontFamilyMono: string;
  fontWeights: number[];
  sizeScale: FontSize[];
}

export interface DesignSpacing {
  baseUnit: number;
  scale: number[];
}

export interface ShadowLevel {
  name: string;
  value: string;
}

export interface DesignElevation {
  levels: ShadowLevel[];
}

export interface DesignShapes {
  borderRadius: Record<string, string>;
}

export interface ComponentSpec {
  name: string;
  description: string;
  variants: string[];
  sizes: string[];
  rules: string[];
}

export interface DesignGuardrail {
  rule: string;
  category: string;
  severity: string; // must | should | must_not | should_not
}

export interface DesignContract {
  schemaVersion: number;
  name: string;
  description?: string | null;
  colors: DesignColors;
  typography: DesignTypography;
  spacing: DesignSpacing;
  elevation: DesignElevation;
  shapes: DesignShapes;
  components: ComponentSpec[];
  guardrails: DesignGuardrail[];
  sourceTemplate?: string | null;
  generatedAt: string;
  opensunstarVersion: string;
}

export interface DesignContractParams {
  templateId?: string | null;
  name: string;
  description?: string | null;
  colors?: DesignColors | null;
  typography?: DesignTypography | null;
  spacing?: DesignSpacing | null;
  elevation?: DesignElevation | null;
  shapes?: DesignShapes | null;
  components?: ComponentSpec[] | null;
  guardrails?: DesignGuardrail[] | null;
}

export interface ImportResult {
  contract: DesignContract;
  source: string;
  warnings: string[];
}

export interface DesignInstallResult {
  filesCreated: string[];
  filesSkipped: string[];
  designMdCreated: boolean;
  dtchgJsonCreated: boolean;
}

// ────────────────────────── Install Plan Types (shared) ──────────────────────────

export interface InstallFileEntry {
  path: string;
  status: "create" | "skip" | "overwrite";
  newContent?: string | null;
  existingContent?: string | null;
}

export interface InstallAuditFinding {
  severity: string;
  ruleId: string;
  message: string;
  file: string;
}

export interface InstallAuditSummary {
  filesScanned: number;
  totalFindings: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  blocked: boolean;
  findings: InstallAuditFinding[];
}

export interface DesignInstallPlan {
  files: InstallFileEntry[];
  audit: InstallAuditSummary;
}

// ────────────────────────── API Methods ──────────────────────────

export const designContractApi = {
  /** List all built-in design templates (returns [id, name][]). */
  async listTemplates(): Promise<[string, string][]> {
    return await invoke<[string, string][]>("list_design_templates_cmd");
  },

  /** Get a specific built-in template by ID. */
  async getTemplate(templateId: string): Promise<DesignContract> {
    return await invoke<DesignContract>("get_design_template_cmd", {
      templateId,
    });
  },

  /** Compose a design contract from parameters (no disk write). */
  async compose(params: DesignContractParams): Promise<DesignContract> {
    return await invoke<DesignContract>("compose_design_contract_cmd", {
      params,
    });
  },

  /** Preview DESIGN.md output (no disk write). */
  async previewDesignMd(params: DesignContractParams): Promise<string> {
    return await invoke<string>("preview_design_md_cmd", { params });
  },

  /** Preview DTCG JSON output (no disk write). */
  async previewDtcgJson(params: DesignContractParams): Promise<string> {
    return await invoke<string>("preview_dtchg_json_cmd", { params });
  },

  /** Export: compose + write DESIGN.md to project root + archive. */
  async exportContract(
    projectId: string,
    params: DesignContractParams,
  ): Promise<string> {
    return await invoke<string>("export_design_contract_cmd", {
      projectId,
      params,
    });
  },

  /** Preview install plan: pre-flight dry run with audit scan (no disk write). */
  async previewInstallPlan(
    projectId: string,
    params: DesignContractParams,
  ): Promise<DesignInstallPlan> {
    return await invoke<DesignInstallPlan>("preview_design_install_plan_cmd", {
      projectId,
      params,
    });
  },

  /** Install: write DESIGN.md + design-tokens.json (safe, never overwrites). */
  async installContract(
    projectId: string,
    params: DesignContractParams,
  ): Promise<DesignInstallResult> {
    return await invoke<DesignInstallResult>("install_design_contract_cmd", {
      projectId,
      params,
    });
  },

  /** Import a DESIGN.md from a local file path. */
  async importFromFile(filePath: string): Promise<ImportResult> {
    return await invoke<ImportResult>("import_design_from_file_cmd", {
      filePath,
    });
  },

  /** Import a DESIGN.md from URL content (content fetched by frontend). */
  async importFromUrl(
    content: string,
    sourceUrl: string,
  ): Promise<ImportResult> {
    return await invoke<ImportResult>("import_design_from_url_cmd", {
      content,
      sourceUrl,
    });
  },
};
