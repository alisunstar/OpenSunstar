use tauri::State;

use crate::services::permission::{PermissionPreset, PermissionService};
use crate::store::AppState;
use crate::tool_permission::ToolPermission;

#[tauri::command]
pub async fn get_all_tool_permissions(
    state: State<'_, AppState>,
) -> Result<Vec<ToolPermission>, String> {
    PermissionService::get_all_permissions(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_tool_permission(
    permission: ToolPermission,
    state: State<'_, AppState>,
) -> Result<(), String> {
    PermissionService::upsert_permission(&state, permission).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tool_permission(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    PermissionService::delete_permission(&state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_tool_permissions(state: State<'_, AppState>) -> Result<(), String> {
    PermissionService::sync_permissions_to_claude(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_permission_presets() -> Result<Vec<PermissionPreset>, String> {
    Ok(PermissionService::get_presets())
}

#[tauri::command]
pub async fn apply_permission_preset(
    preset_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    PermissionService::apply_preset(&state, &preset_id).map_err(|e| e.to_string())
}
