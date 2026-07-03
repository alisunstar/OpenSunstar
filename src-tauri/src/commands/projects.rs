//! 项目级配置隔离 - Tauri 命令

use tauri::State;

use crate::database::{Project, ProjectConfigLink, ProjectPromptLink};
use crate::store::AppState;

fn touch_readiness(state: &AppState, project_id: &str) {
    crate::services::project_artifacts::touch_project_governance(state, project_id);
}

// ========== Projects CRUD ==========

#[tauri::command]
pub async fn get_all_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    state.db.get_all_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project(state: State<'_, AppState>, id: String) -> Result<Option<Project>, String> {
    state.db.get_project(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_by_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<Project>, String> {
    state.db.get_project_by_path(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_project(state: State<'_, AppState>, project: Project) -> Result<(), String> {
    state.db.upsert_project(&project).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_project(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    state.db.delete_project(&id).map_err(|e| e.to_string())
}

// ========== Project × MCP ==========

#[tauri::command]
pub async fn get_project_mcp_servers(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ProjectConfigLink>, String> {
    state
        .db
        .get_project_mcp_servers(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn link_project_mcp_server(
    state: State<'_, AppState>,
    project_id: String,
    mcp_server_id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .link_project_mcp_server(&project_id, &mcp_server_id, enabled)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn unlink_project_mcp_server(
    state: State<'_, AppState>,
    project_id: String,
    mcp_server_id: String,
) -> Result<bool, String> {
    let removed = state
        .db
        .unlink_project_mcp_server(&project_id, &mcp_server_id)
        .map_err(|e| e.to_string())?;
    if removed {
        touch_readiness(&state, &project_id);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn set_project_mcp_servers(
    state: State<'_, AppState>,
    project_id: String,
    mcp_server_ids: Vec<String>,
) -> Result<(), String> {
    state
        .db
        .set_project_mcp_servers(&project_id, &mcp_server_ids)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

// ========== Project × Skills ==========

#[tauri::command]
pub async fn get_project_skills(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ProjectConfigLink>, String> {
    state
        .db
        .get_project_skills(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn link_project_skill(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .link_project_skill(&project_id, &skill_id, enabled)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn unlink_project_skill(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
) -> Result<bool, String> {
    let removed = state
        .db
        .unlink_project_skill(&project_id, &skill_id)
        .map_err(|e| e.to_string())?;
    if removed {
        touch_readiness(&state, &project_id);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn set_project_skills(
    state: State<'_, AppState>,
    project_id: String,
    skill_ids: Vec<String>,
) -> Result<(), String> {
    state
        .db
        .set_project_skills(&project_id, &skill_ids)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

// ========== Project × Prompts ==========

#[tauri::command]
pub async fn get_project_prompts(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ProjectPromptLink>, String> {
    state
        .db
        .get_project_prompts(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn link_project_prompt(
    state: State<'_, AppState>,
    project_id: String,
    prompt_id: String,
    prompt_app_type: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .link_project_prompt(&project_id, &prompt_id, &prompt_app_type, enabled)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn unlink_project_prompt(
    state: State<'_, AppState>,
    project_id: String,
    prompt_id: String,
    prompt_app_type: String,
) -> Result<bool, String> {
    let removed = state
        .db
        .unlink_project_prompt(&project_id, &prompt_id, &prompt_app_type)
        .map_err(|e| e.to_string())?;
    if removed {
        touch_readiness(&state, &project_id);
    }
    Ok(removed)
}

#[tauri::command]
pub async fn set_project_target_app(
    state: State<'_, AppState>,
    project_id: String,
    target_app: Option<String>,
) -> Result<(), String> {
    state
        .db
        .set_project_target_app(&project_id, target_app.as_deref())
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}

#[tauri::command]
pub async fn set_project_prompts(
    state: State<'_, AppState>,
    project_id: String,
    prompts: Vec<(String, String)>,
) -> Result<(), String> {
    state
        .db
        .set_project_prompts(&project_id, &prompts)
        .map_err(|e| e.to_string())?;
    touch_readiness(&state, &project_id);
    Ok(())
}
