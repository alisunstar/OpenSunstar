use indexmap::IndexMap;
use std::str::FromStr;

use tauri::State;

use crate::agent::Agent;
use crate::app_config::AppType;
use crate::services::AgentService;
use crate::store::AppState;

#[tauri::command]
pub async fn get_all_agents(
    state: State<'_, AppState>,
) -> Result<IndexMap<String, Agent>, String> {
    AgentService::get_all_agents(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_agent(agent: Agent, state: State<'_, AppState>) -> Result<(), String> {
    AgentService::upsert_agent(&state, agent).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_agent(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    AgentService::delete_agent(&state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_agent_codex_toml(agent: Agent) -> Result<String, String> {
    AgentService::preview_codex_toml(&agent).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_agent_app(
    agent_id: String,
    app: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    AgentService::toggle_app(&state, &agent_id, app_type, enabled)
        .map_err(|e| e.to_string())
}
