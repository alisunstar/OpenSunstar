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
pub async fn get_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Project>, String> {
    state.db.get_project(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_by_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<Project>, String> {
    state
        .db
        .get_project_by_path(&path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_project(state: State<'_, AppState>, project: Project) -> Result<(), String> {
    state.db.upsert_project(&project).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_project(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    state.db.delete_project(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_project_board_metadata(
    state: State<'_, AppState>,
    project_id: String,
    stage: String,
    mvp_progress: Option<i32>,
) -> Result<(), String> {
    state
        .db
        .update_project_board_metadata(&project_id, &stage, mvp_progress)
        .map_err(|e| e.to_string())
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

// ========== Project Context File Detection ==========

/// Per-app context file info: filename, existence, and whether OpenSunstar manages it.
#[derive(serde::Serialize)]
pub struct ProjectContextFile {
    pub app: String,
    pub filename: String,
    pub exists: bool,
    pub managed: bool,
}

#[tauri::command]
pub async fn get_project_context_files(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ProjectContextFile>, String> {
    use crate::app_config::AppType;
    use crate::prompt_files::project_prompt_file_path;
    use crate::services::marker_merge::has_companion_marker;
    use std::path::Path;

    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    let root = Path::new(&project.path);
    let apps = [
        AppType::Claude,
        AppType::Codex,
        AppType::Gemini,
        AppType::OpenCode,
        AppType::Hermes,
    ];

    let mut results = Vec::new();
    for app in &apps {
        if let Ok(file_path) = project_prompt_file_path(root, app) {
            let filename = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let exists = file_path.is_file();
            let managed = exists && has_companion_marker(&file_path);
            results.push(ProjectContextFile {
                app: app.as_str().to_string(),
                filename,
                exists,
                managed,
            });
        }
    }

    Ok(results)
}
