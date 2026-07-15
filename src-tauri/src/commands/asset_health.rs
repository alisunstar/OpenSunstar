//! Tauri read API for the project asset health evidence chain.

use tauri::State;

use crate::ai::asset_health::{
    apply_project_asset_health_plan, get_project_asset_health, plan_project_asset_health,
    rollback_project_asset_health_receipt, AssetHealthPlan, AssetHealthRecord,
};
use crate::database::{AssetDeploymentReceipt, AssetRevision};
use crate::store::AppState;

#[tauri::command]
pub async fn get_project_asset_health_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<AssetHealthRecord>, String> {
    get_project_asset_health(&state.db, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn plan_project_asset_health_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<AssetHealthPlan, String> {
    plan_project_asset_health(&state.db, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn apply_project_asset_health_plan_cmd(
    state: State<'_, AppState>,
    project_id: String,
    plan_sha256: String,
    confirmed: bool,
) -> Result<Vec<AssetDeploymentReceipt>, String> {
    apply_project_asset_health_plan(&state, &project_id, &plan_sha256, confirmed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rollback_project_asset_health_receipt_cmd(
    state: State<'_, AppState>,
    receipt_id: String,
    confirmed: bool,
) -> Result<AssetDeploymentReceipt, String> {
    rollback_project_asset_health_receipt(&state, &receipt_id, confirmed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn register_asset_revision_cmd(
    state: State<'_, AppState>,
    asset_type: String,
    asset_id: String,
    content: String,
    source_kind: String,
    source_ref: Option<String>,
    source_revision: Option<String>,
    version_label: Option<String>,
) -> Result<AssetRevision, String> {
    state
        .db
        .register_asset_revision(
            &asset_type,
            &asset_id,
            content.as_bytes(),
            &source_kind,
            source_ref.as_deref(),
            source_revision.as_deref(),
            version_label.as_deref(),
        )
        .map_err(|error| error.to_string())
}
