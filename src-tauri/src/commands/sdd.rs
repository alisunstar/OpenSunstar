//! SDD framework detection commands (read-only probes).

use std::collections::HashMap;
use tauri::State;

use crate::services::sdd::{self, SddDescriptorSummary, SddDetectionResult};
use crate::store::AppState;

fn project_path_for_id(db: &crate::database::Database, project_id: &str) -> Result<String, String> {
    db.get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
        .map(|p| p.path)
}

/// List all 7 framework descriptors.
#[tauri::command]
pub async fn sdd_list_descriptors_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<SddDescriptorSummary>, String> {
    sdd::list_descriptors(&state.db).map_err(|e| e.to_string())
}

/// Detect frameworks for a single project. Read-only.
#[tauri::command]
pub async fn sdd_detect_project_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<SddDetectionResult>, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let results = sdd::detect_project(&path);
    sdd::save_detection_results(&state.db, &project_id, &results).map_err(|e| e.to_string())?;
    Ok(results)
}

/// Detect frameworks for all projects. Read-only batch.
#[tauri::command]
pub async fn sdd_detect_all_projects_cmd(
    state: State<'_, AppState>,
) -> Result<HashMap<String, Vec<SddDetectionResult>>, String> {
    sdd::detect_all_projects(&state.db).map_err(|e| e.to_string())
}

/// Get saved detection results for a project.
#[tauri::command]
pub async fn sdd_get_detection_results_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<SddDetectionResult>, String> {
    sdd::get_detection_results(&state.db, &project_id).map_err(|e| e.to_string())
}

/// Get saved detection results for all previously scanned projects.
#[tauri::command]
pub async fn sdd_get_all_saved_detections_cmd(
    state: State<'_, AppState>,
) -> Result<HashMap<String, Vec<SddDetectionResult>>, String> {
    sdd::get_all_saved_detections(&state.db).map_err(|e| e.to_string())
}

/// Recommend a workflow preset tier from saved detection results for a project.
#[tauri::command]
pub async fn sdd_recommend_preset_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Option<String>, String> {
    let results = sdd::get_detection_results(&state.db, &project_id).map_err(|e| e.to_string())?;
    Ok(sdd::recommend_preset_from_detections(&results))
}
