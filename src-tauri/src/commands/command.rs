use indexmap::IndexMap;
use std::str::FromStr;

use tauri::State;

use crate::app_config::AppType;
use crate::command::Command;
use crate::services::CommandService;
use crate::store::AppState;

#[tauri::command]
pub async fn get_all_commands(
    state: State<'_, AppState>,
) -> Result<IndexMap<String, Command>, String> {
    CommandService::get_all_commands(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_command(
    command: Command,
    state: State<'_, AppState>,
) -> Result<(), String> {
    CommandService::upsert_command(&state, command).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_command(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    CommandService::delete_command(&state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_command_app(
    command_id: String,
    app: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    CommandService::toggle_app(&state, &command_id, app_type, enabled)
        .map_err(|e| e.to_string())
}
