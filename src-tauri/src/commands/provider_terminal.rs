#![allow(non_snake_case)]

use tauri::State;

#[tauri::command]
pub async fn open_provider_terminal(
    state: State<'_, crate::store::AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    cwd: Option<String>,
) -> Result<bool, String> {
    crate::services::provider_terminal::open_provider_terminal(state.inner(), app, providerId, cwd)
        .await
}
