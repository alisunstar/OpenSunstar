//! GitHub Gist sync IPC commands

use crate::commands::sync_support::{
    attach_warning, post_sync_warning_from_result, run_post_import_sync,
};
use crate::services::gist_sync;
use crate::store::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn gist_sync_test_connection() -> Result<Value, String> {
    gist_sync::test_connection()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gist_sync_upload(state: State<'_, AppState>) -> Result<Value, String> {
    gist_sync::upload(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gist_sync_download(state: State<'_, AppState>) -> Result<Value, String> {
    let db = state.db.clone();
    let db_for_sync = db.clone();

    let mut result = gist_sync::download(&db).await.map_err(|e| e.to_string())?;

    let warning = post_sync_warning_from_result(
        tauri::async_runtime::spawn_blocking(move || run_post_import_sync(db_for_sync))
            .await
            .map_err(|e| e.to_string()),
    );
    if let Some(msg) = warning.as_ref() {
        log::warn!("[Gist] post-download sync warning: {msg}");
    }
    result = attach_warning(result, warning);

    Ok(result)
}

#[tauri::command]
pub async fn gist_sync_save_pat(pat: String) -> Result<(), String> {
    gist_sync::save_pat(&pat).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gist_sync_clear() -> Result<(), String> {
    gist_sync::clear_config().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gist_sync_is_configured() -> Result<bool, String> {
    Ok(gist_sync::is_configured())
}
