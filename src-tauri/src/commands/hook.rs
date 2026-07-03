use tauri::State;

use crate::ai::readiness_cache::invalidate_all_agent_readiness_caches;
use crate::app_config::AppType;
use crate::hook::Hook;
use crate::services::HookService;
use crate::store::AppState;

#[tauri::command]
pub async fn get_all_hooks(state: State<'_, AppState>) -> Result<Vec<Hook>, String> {
    HookService::get_all_hooks(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_hook(hook: Hook, state: State<'_, AppState>) -> Result<(), String> {
    HookService::upsert_hook(&state, hook).map_err(|e| e.to_string())?;
    invalidate_all_agent_readiness_caches(&state.db);
    Ok(())
}

#[tauri::command]
pub async fn delete_hook(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let removed = HookService::delete_hook(&state, &id).map_err(|e| e.to_string())?;
    if removed {
        invalidate_all_agent_readiness_caches(&state.db);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn toggle_hook_app(
    hook_id: String,
    app: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type: AppType = app.parse::<AppType>().map_err(|e| e.to_string())?;
    HookService::toggle_app(&state, &hook_id, app_type, enabled).map_err(|e| e.to_string())?;
    invalidate_all_agent_readiness_caches(&state.db);
    Ok(())
}

#[tauri::command]
pub async fn sync_hooks(state: State<'_, AppState>) -> Result<(), String> {
    HookService::sync_all_apps(&state).map_err(|e| e.to_string())
}
