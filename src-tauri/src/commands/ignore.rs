use tauri::State;

use crate::ai::readiness_cache::invalidate_all_agent_readiness_caches;
use crate::app_config::AppType;
use crate::ignore_rule::IgnoreRule;
use crate::services::IgnoreService;
use crate::store::AppState;

#[tauri::command]
pub async fn get_all_ignore_rules(
    state: State<'_, AppState>,
) -> Result<Vec<IgnoreRule>, String> {
    IgnoreService::get_all_rules(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_ignore_rule(
    rule: IgnoreRule,
    state: State<'_, AppState>,
) -> Result<(), String> {
    IgnoreService::upsert_rule(&state, rule).map_err(|e| e.to_string())?;
    invalidate_all_agent_readiness_caches(&state.db);
    Ok(())
}

#[tauri::command]
pub async fn delete_ignore_rule(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let removed = IgnoreService::delete_rule(&state, &id).map_err(|e| e.to_string())?;
    if removed {
        invalidate_all_agent_readiness_caches(&state.db);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn toggle_ignore_app(
    rule_id: String,
    app: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type: AppType = app
        .parse::<AppType>()
        .map_err(|e| e.to_string())?;
    IgnoreService::toggle_app(&state, &rule_id, app_type, enabled).map_err(|e| e.to_string())?;
    invalidate_all_agent_readiness_caches(&state.db);
    Ok(())
}

#[tauri::command]
pub async fn import_ignore_from_gitignore(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let count = IgnoreService::import_from_gitignore(&state, &file_path).map_err(|e| e.to_string())?;
    if count > 0 {
        invalidate_all_agent_readiness_caches(&state.db);
    }
    Ok(count)
}

#[tauri::command]
pub async fn sync_ignore_rules(state: State<'_, AppState>) -> Result<(), String> {
    IgnoreService::sync_all_apps(&state).map_err(|e| e.to_string())
}
