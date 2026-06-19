//! Bridge IPC commands

use crate::services::bridge;
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub async fn bridge_prompt(
    state: State<'_, AppState>,
    source_app: String,
    target_app: String,
    id: String,
) -> Result<serde_json::Value, String> {
    bridge::bridge_prompt(&state.db, &source_app, &target_app, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_bridgeable_prompts(
    state: State<'_, AppState>,
    source_app: String,
) -> Result<Vec<bridge::BridgeCandidate>, String> {
    bridge::get_bridgeable_prompts(&state.db, &source_app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn push_bridge_changes(
    state: State<'_, AppState>,
    source_app: String,
    source_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    bridge::push_bridge_changes(&state.db, &source_app, &source_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unlink_bridge(
    state: State<'_, AppState>,
    app_type: String,
    id: String,
) -> Result<(), String> {
    bridge::unlink_bridge(&state.db, &app_type, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_bridge(
    source_app: String,
    target_app: String,
    content: String,
) -> Result<bridge::BridgePreview, String> {
    Ok(bridge::preview_bridge(&source_app, &target_app, &content))
}

#[tauri::command]
pub async fn get_bridge_auto_push(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.db
        .get_setting("bridge_auto_push")
        .map(|v| v.map(|s| s == "true").unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_bridge_auto_push(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('bridge_auto_push', ?1)",
        rusqlite::params![if enabled { "true" } else { "false" }],
    )
    .map_err(|e| format!("Failed to save bridge_auto_push: {e}"))?;
    Ok(())
}
