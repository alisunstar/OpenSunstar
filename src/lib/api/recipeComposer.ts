import { invoke } from "@tauri-apps/api/core";
import type { InstallFileEntry, InstallAuditSummary } from "./designContract";

// ────────────────────────── Stage Graph Types ──────────────────────────

export interface StageGraphNode {
  id: string;
  name: string;
  artifacts: string[];
  dependsOn: string[];
  standalone: string; // "true" | "semi" | "false"
  requires: string[];
  condition?: string | null;
  lateral: boolean;
  depth: number;
}

export interface StageGraphEdge {
  source: string;
  target: string;
}

export interface StageGraph {
  presetId: string;
  presetName: string;
  sourceFramework: string;
  nodes: StageGraphNode[];
  edges: StageGraphEdge[];
  lateralNodes: StageGraphNode[];
}

// ────────────────────────── Recipe Types ──────────────────────────

export interface RecipeStage {
  id: string;
  name: string;
  artifact?: string | null;
  dependsOn: string[];
  enabled: boolean;
  condition?: string | null;
  doc: string;
}

export interface RecipeArtifact {
  path: string;
  purpose: string;
  freshnessDays?: number | null;
}

export interface RecipeRule {
  name: string;
  value: string;
  description: string;
}

export interface CompositionRecipe {
  schemaVersion: number;
  name: string;
  description: string;
  presetId: string;
  projectType: string;
  modules: string[];
  stages: RecipeStage[];
  excludedStages: string[];
  artifacts: RecipeArtifact[];
  rules: RecipeRule[];
  notes: string;
  generatedAt: string;
  opensunstarVersion: string;
}

export interface RecipeComposeParams {
  presetId: string;
  projectType: string;
  name: string;
  description?: string | null;
  selectedModules?: string[] | null;
  disabledStages?: string[] | null;
  notes?: string | null;
  stageDocs?: Record<string, string> | null;
}

export interface InstallResult {
  changeId: string;
  filesCreated: string[];
  filesSkipped: string[];
  specsDirCreated: boolean;
  stateFileCreated: boolean;
}

export interface RecipeInstallPlan {
  files: InstallFileEntry[];
  audit: InstallAuditSummary;
}

// ────────────────────────── API Methods ──────────────────────────

export const recipeComposerApi = {
  /** Build a stage graph DAG from a workflow preset. */
  async buildStageGraph(
    presetId: string,
    projectId?: string,
  ): Promise<StageGraph> {
    return await invoke<StageGraph>("build_stage_graph_cmd", {
      presetId,
      projectId: projectId ?? null,
    });
  },

  /** Compose a recipe from preset + user selections (no disk write). */
  async composeRecipe(
    projectId: string,
    params: RecipeComposeParams,
  ): Promise<CompositionRecipe> {
    return await invoke<CompositionRecipe>("compose_recipe_cmd", {
      projectId,
      params,
    });
  },

  /** Preview the YAML+Markdown hybrid output (no disk write). */
  async previewRecipe(
    projectId: string,
    params: RecipeComposeParams,
  ): Promise<string> {
    return await invoke<string>("preview_recipe_cmd", {
      projectId,
      params,
    });
  },

  /** Export recipe: compose + generate hybrid + write to .opensunstar/recipe/. */
  async exportRecipe(
    projectId: string,
    params: RecipeComposeParams,
  ): Promise<string> {
    return await invoke<string>("export_recipe_cmd", {
      projectId,
      params,
    });
  },

  /** List all saved recipe names from .opensunstar/recipe/. */
  async listSavedRecipes(projectId: string): Promise<string[]> {
    return await invoke<string[]>("list_saved_recipes_cmd", {
      projectId,
    });
  },

  /** Read a saved recipe file content. */
  async readSavedRecipe(
    projectId: string,
    name: string,
  ): Promise<string> {
    return await invoke<string>("read_saved_recipe_cmd", {
      projectId,
      name,
    });
  },

  /** Load a saved plan as structured composition data for editing or re-installing. */
  async loadSavedRecipe(
    projectId: string,
    name: string,
  ): Promise<CompositionRecipe> {
    return await invoke<CompositionRecipe>("load_saved_recipe_cmd", {
      projectId,
      name,
    });
  },

  /** Delete a saved recipe file. */
  async deleteSavedRecipe(
    projectId: string,
    name: string,
  ): Promise<void> {
    return await invoke<void>("delete_saved_recipe_cmd", {
      projectId,
      name,
    });
  },

  /** Preview recipe install plan: pre-flight dry run with audit scan (no disk write). */
  async previewInstallPlan(
    projectId: string,
    params: RecipeComposeParams,
    changeId: string,
  ): Promise<RecipeInstallPlan> {
    return await invoke<RecipeInstallPlan>("preview_recipe_install_plan_cmd", {
      projectId,
      params,
      changeId,
    });
  },

  /** Install a recipe: scaffold .specs/ dir, create template files, write STATE.md.
   *  This is the "template installer" — materializes the recipe into actual project files.
   *  Never overwrites existing files (safe install). */
  async installRecipe(
    projectId: string,
    params: RecipeComposeParams,
    changeId: string,
  ): Promise<InstallResult> {
    return await invoke<InstallResult>("install_recipe_cmd", {
      projectId,
      params,
      changeId,
    });
  },
};
