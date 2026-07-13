//! Orchestration Layer commands: Stage Graph + Recipe composition.

use tauri::State;

use crate::database::Database;
use crate::services::flow_orchestrator::{get_workflow_preset, list_workflow_modules};
use crate::services::recipe_composer::{
    build_stage_graph, compose_recipe, delete_saved_recipe, export_recipe, generate_recipe_hybrid,
    install_recipe, list_saved_recipes, parse_recipe_frontmatter, preview_recipe_install_plan,
    read_saved_recipe, CompositionRecipe, InstallResult, RecipeComposeParams, RecipeInstallPlan,
    StageGraph,
};
use crate::store::AppState;

fn project_path_for_id(db: &Database, project_id: &str) -> Result<String, String> {
    db.get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
        .map(|p| p.path)
}

/// Build a stage graph DAG from a workflow preset.
#[tauri::command]
pub async fn build_stage_graph_cmd(
    state: State<'_, AppState>,
    preset_id: String,
    project_id: Option<String>,
) -> Result<StageGraph, String> {
    let project_path = match project_id {
        Some(id) => Some(project_path_for_id(&state.db, &id)?),
        None => None,
    };
    let preset =
        get_workflow_preset(&preset_id, project_path.as_deref()).map_err(|e| e.to_string())?;
    Ok(build_stage_graph(&preset))
}

/// Compose a recipe from a preset + user selections (returns the recipe object, does NOT write to disk).
#[tauri::command]
pub async fn compose_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: RecipeComposeParams,
) -> Result<CompositionRecipe, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let preset = get_workflow_preset(&params.preset_id, Some(&path)).map_err(|e| e.to_string())?;
    let modules = list_workflow_modules(Some(&path)).map_err(|e| e.to_string())?;
    compose_recipe(&preset, &params, &modules).map_err(|e| e.to_string())
}

/// Preview the YAML+Markdown hybrid output for a recipe (without writing to disk).
#[tauri::command]
pub async fn preview_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: RecipeComposeParams,
) -> Result<String, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let preset = get_workflow_preset(&params.preset_id, Some(&path)).map_err(|e| e.to_string())?;
    let modules = list_workflow_modules(Some(&path)).map_err(|e| e.to_string())?;
    let recipe = compose_recipe(&preset, &params, &modules).map_err(|e| e.to_string())?;
    generate_recipe_hybrid(&recipe).map_err(|e| e.to_string())
}

/// Export a recipe: compose + generate hybrid + write to `.opensunstar/recipe/`.
#[tauri::command]
pub async fn export_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: RecipeComposeParams,
) -> Result<String, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let preset = get_workflow_preset(&params.preset_id, Some(&path)).map_err(|e| e.to_string())?;
    let modules = list_workflow_modules(Some(&path)).map_err(|e| e.to_string())?;
    let recipe = compose_recipe(&preset, &params, &modules).map_err(|e| e.to_string())?;
    export_recipe(&path, &recipe).map_err(|e| e.to_string())
}

/// List all saved recipe names from `.opensunstar/recipe/`.
#[tauri::command]
pub async fn list_saved_recipes_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<String>, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    list_saved_recipes(&path).map_err(|e| e.to_string())
}

/// Read a saved recipe file content.
#[tauri::command]
pub async fn read_saved_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<String, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    read_saved_recipe(&path, &name).map_err(|e| e.to_string())
}

/// Load a saved recipe's structured YAML frontmatter for editing or re-installing.
#[tauri::command]
pub async fn load_saved_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<CompositionRecipe, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let content = read_saved_recipe(&path, &name).map_err(|e| e.to_string())?;
    parse_recipe_frontmatter(&content).map_err(|e| e.to_string())
}

/// Delete a saved recipe file.
#[tauri::command]
pub async fn delete_saved_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<(), String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    delete_saved_recipe(&path, &name).map_err(|e| e.to_string())
}

/// Preview recipe install plan: pre-flight dry run with audit scan (no disk write to project).
#[tauri::command]
pub async fn preview_recipe_install_plan_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: RecipeComposeParams,
    change_id: String,
) -> Result<RecipeInstallPlan, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let preset = get_workflow_preset(&params.preset_id, Some(&path)).map_err(|e| e.to_string())?;
    let modules = list_workflow_modules(Some(&path)).map_err(|e| e.to_string())?;
    let recipe = compose_recipe(&preset, &params, &modules).map_err(|e| e.to_string())?;
    preview_recipe_install_plan(&path, &recipe, &change_id).map_err(|e| e.to_string())
}

/// Install a recipe: scaffold .specs/ directory, create template files, and write STATE.md.
/// This is the "template installer" — materializes the recipe into actual project files.
#[tauri::command]
pub async fn install_recipe_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: RecipeComposeParams,
    change_id: String,
) -> Result<InstallResult, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let preset = get_workflow_preset(&params.preset_id, Some(&path)).map_err(|e| e.to_string())?;
    let modules = list_workflow_modules(Some(&path)).map_err(|e| e.to_string())?;
    let recipe = compose_recipe(&preset, &params, &modules).map_err(|e| e.to_string())?;
    install_recipe(&path, &recipe, &change_id).map_err(|e| e.to_string())
}
