//! 项目扩展资产关联 Tauri 命令

use tauri::State;

use crate::ai::readiness_cache::invalidate_agent_readiness_for_project;
use crate::database::{ProjectAllAssetCounts, ProjectAssetLink, EXTENDED_ASSET_TYPES};
use crate::store::AppState;

fn touch_readiness(state: &AppState, project_id: &str) {
    invalidate_agent_readiness_for_project(&state.db, project_id, None);
}

#[tauri::command]
pub async fn get_project_asset_links(
    state: State<'_, AppState>,
    project_id: String,
    asset_type: Option<String>,
) -> Result<Vec<ProjectAssetLink>, String> {
    state
        .db
        .get_project_asset_links(&project_id, asset_type.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn link_project_asset(
    state: State<'_, AppState>,
    project_id: String,
    asset_type: String,
    asset_id: String,
    asset_app_type: Option<String>,
    enabled: bool,
) -> Result<(), String> {
    let app_type = asset_app_type.unwrap_or_default();
    state
        .db
        .link_project_asset(&project_id, &asset_type, &asset_id, &app_type, enabled)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn unlink_project_asset(
    state: State<'_, AppState>,
    project_id: String,
    asset_type: String,
    asset_id: String,
    asset_app_type: Option<String>,
) -> Result<bool, String> {
    let app_type = asset_app_type.unwrap_or_default();
    let removed = state
        .db
        .unlink_project_asset(&project_id, &asset_type, &asset_id, &app_type)
        .map_err(|e| e.to_string())?;
    if removed {
        touch_readiness(&state, &project_id);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn set_project_assets(
    state: State<'_, AppState>,
    project_id: String,
    asset_type: String,
    asset_ids: Vec<String>,
) -> Result<(), String> {
    state
        .db
        .set_project_assets(&project_id, &asset_type, &asset_ids)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn get_project_all_asset_counts(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectAllAssetCounts, String> {
    state
        .db
        .get_project_all_asset_counts(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_extended_project_asset_types() -> Result<Vec<String>, String> {
    Ok(EXTENDED_ASSET_TYPES.iter().map(|s| s.to_string()).collect())
}
