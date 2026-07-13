//! Onboarding IPC commands

use crate::services::onboarding::{self, ScanResult};
use crate::store::AppState;

#[tauri::command]
pub async fn scan_environment() -> Result<ScanResult, String> {
    onboarding::scan_environment().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn complete_onboarding(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .set_setting("onboarding_completed", "true")
        .map_err(|e| format!("Failed to mark onboarding complete: {e}"))
}

#[tauri::command]
pub async fn is_onboarding_needed(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let completed = state
        .db
        .get_setting("onboarding_completed")
        .map(|v| v.as_deref() == Some("true"))
        .unwrap_or(false);
    Ok(!completed)
}
