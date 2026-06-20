use tauri::State;

use crate::hook::Hook;
use crate::services::HookService;
use crate::store::AppState;

#[tauri::command]
pub async fn get_all_hooks(state: State<'_, AppState>) -> Result<Vec<Hook>, String> {
    HookService::get_all_hooks(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_hook(hook: Hook, state: State<'_, AppState>) -> Result<(), String> {
    HookService::upsert_hook(&state, hook).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_hook(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    HookService::delete_hook(&state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_hooks(state: State<'_, AppState>) -> Result<(), String> {
    HookService::sync_hooks_to_claude(&state).map_err(|e| e.to_string())
}
