//! Blueprint 基线模板（S2-10 / S2-11）

use tauri::State;

use crate::services::blueprint::{
    apply_blueprint_to_project, get_blueprint, list_blueprints, preview_apply_blueprint, Blueprint,
    BlueprintApplyPreview,
};
use crate::store::AppState;

#[tauri::command]
pub async fn list_project_blueprints() -> Result<Vec<Blueprint>, String> {
    list_blueprints().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_blueprint(id: String) -> Result<Blueprint, String> {
    get_blueprint(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_apply_project_blueprint(
    state: State<'_, AppState>,
    project_id: String,
    blueprint_id: String,
) -> Result<BlueprintApplyPreview, String> {
    preview_apply_blueprint(&state, &project_id, &blueprint_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_project_blueprint(
    state: State<'_, AppState>,
    project_id: String,
    blueprint_id: String,
) -> Result<BlueprintApplyPreview, String> {
    apply_blueprint_to_project(&state, &project_id, &blueprint_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_project_baseline_snapshot(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<String, String> {
    let path =
        crate::services::project_artifacts::export_baseline_snapshot(&state.db, &project_id, None)
            .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}
