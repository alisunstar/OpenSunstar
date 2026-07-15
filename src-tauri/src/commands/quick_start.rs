use crate::database::{QuickStartOperation, QuickStartOperationEvent};
use crate::services::quick_start::{QuickStartApplyRequest, QuickStartService};
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub async fn quick_start_apply(
    state: State<'_, AppState>,
    request: QuickStartApplyRequest,
) -> Result<QuickStartOperation, String> {
    QuickStartService::apply(state.inner(), request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quick_start_get_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<QuickStartOperation>, String> {
    state
        .db
        .get_quick_start_operation(&operation_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quick_start_list_recoverable(
    state: State<'_, AppState>,
) -> Result<Vec<QuickStartOperation>, String> {
    state
        .db
        .list_recoverable_quick_start_operations()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quick_start_list_recent(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<QuickStartOperation>, String> {
    state
        .db
        .list_recent_quick_start_operations(limit.unwrap_or(20))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quick_start_get_events(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Vec<QuickStartOperationEvent>, String> {
    state
        .db
        .list_quick_start_operation_events(&operation_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn quick_start_rollback(
    state: State<'_, AppState>,
    operation_id: String,
    expected_revision: i64,
) -> Result<QuickStartOperation, String> {
    QuickStartService::rollback(state.inner(), &operation_id, expected_revision)
        .await
        .map_err(|error| error.to_string())
}
