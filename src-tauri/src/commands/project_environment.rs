//! Project environment snapshot commands.

use tauri::State;

use crate::services::project_environment::{
    ProjectEnvironmentApplyPreview, ProjectEnvironmentApplyReceipt, ProjectEnvironmentDimension,
    ProjectEnvironmentService, ProjectEnvironmentSnapshotDto,
};
use crate::store::AppState;

#[tauri::command]
pub async fn list_project_environment_snapshots(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ProjectEnvironmentSnapshotDto>, String> {
    ProjectEnvironmentService::list(&state, &project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_project_environment_snapshot(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    included_dimensions: Option<Vec<ProjectEnvironmentDimension>>,
) -> Result<ProjectEnvironmentSnapshotDto, String> {
    let included_dimensions =
        included_dimensions.unwrap_or_else(|| ProjectEnvironmentDimension::all());
    ProjectEnvironmentService::create(&state, &project_id, &name, &included_dimensions)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_project_environment_snapshot(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<bool, String> {
    ProjectEnvironmentService::delete(&state, &snapshot_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_project_environment_snapshot_apply(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<ProjectEnvironmentApplyPreview, String> {
    ProjectEnvironmentService::preview_apply(&state, &snapshot_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_project_environment_snapshot(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<ProjectEnvironmentApplyReceipt, String> {
    ProjectEnvironmentService::apply(&state, &snapshot_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rollback_project_environment_snapshot(
    state: State<'_, AppState>,
    snapshot_id: String,
) -> Result<ProjectEnvironmentApplyReceipt, String> {
    ProjectEnvironmentService::rollback(&state, &snapshot_id).map_err(|e| e.to_string())
}
