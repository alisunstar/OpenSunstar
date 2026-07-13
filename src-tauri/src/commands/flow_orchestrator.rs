//! SDD workflow orchestrator commands (flow-kit compatible indexing + gates).

use tauri::State;

use crate::database::Database;
use crate::services::flow_orchestrator::{
    export_flow_config, export_flow_config_strict, export_project_workflow_profile,
    export_project_workflow_profile_strict, get_workflow_preset, list_workflow_modules,
    list_workflow_presets, preview_flow_config_export, preview_project_workflow_profile_export,
    scan_project_specs_workflow, validate_workflow_stage_gate, FlowConfig, FlowWritePlan,
    SpecsWorkflowIndex, StageGateResult, WorkflowModule, WorkflowPreset, WorkflowPresetSummary,
    WorkflowProfile,
};
use crate::store::AppState;

fn project_path_for_id(db: &Database, project_id: &str) -> Result<String, String> {
    db.get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
        .map(|p| p.path)
}

#[tauri::command]
pub async fn list_workflow_modules_cmd(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> Result<Vec<WorkflowModule>, String> {
    let project_path = match project_id {
        Some(id) => Some(project_path_for_id(&state.db, &id)?),
        None => None,
    };
    list_workflow_modules(project_path.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workflow_presets_cmd(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> Result<Vec<WorkflowPresetSummary>, String> {
    let project_path = match project_id {
        Some(id) => Some(project_path_for_id(&state.db, &id)?),
        None => None,
    };
    list_workflow_presets(project_path.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_workflow_preset_cmd(
    state: State<'_, AppState>,
    id: String,
    project_id: Option<String>,
) -> Result<WorkflowPreset, String> {
    let project_path = match project_id {
        Some(pid) => Some(project_path_for_id(&state.db, &pid)?),
        None => None,
    };
    get_workflow_preset(&id, project_path.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_project_specs_workflow_cmd(
    state: State<'_, AppState>,
    project_id: String,
    preset_id: Option<String>,
    project_type: Option<String>,
) -> Result<SpecsWorkflowIndex, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    scan_project_specs_workflow(&path, preset_id.as_deref(), project_type.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_workflow_stage_gate_cmd(
    state: State<'_, AppState>,
    project_id: String,
    preset_id: String,
    project_type: String,
    change_id: String,
    target_stage: String,
) -> Result<StageGateResult, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let result =
        validate_workflow_stage_gate(&path, &preset_id, &project_type, &change_id, &target_stage)
            .map_err(|e| e.to_string())?;

    if let Err(e) = crate::services::flow_orchestrator::append_orchestration_log(
        &path,
        serde_json::json!({
            "event": "stage_gate",
            "presetId": preset_id,
            "projectType": project_type,
            "changeId": change_id,
            "targetStage": target_stage,
            "allowed": result.allowed,
            "missing": result.missing_artifacts,
        }),
    ) {
        log::warn!("写入 orchestration log 失败: {e}");
    }

    Ok(result)
}

#[tauri::command]
pub async fn preview_project_workflow_profile_export_cmd(
    state: State<'_, AppState>,
    project_id: String,
    preset_id: String,
    project_type: String,
    active_change_id: Option<String>,
    selected_modules: Option<Vec<String>>,
    disabled_stages: Option<Vec<String>>,
) -> Result<FlowWritePlan, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    preview_project_workflow_profile_export(
        &path,
        &preset_id,
        &project_type,
        active_change_id.as_deref(),
        selected_modules.as_deref(),
        disabled_stages.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_project_workflow_profile_cmd(
    state: State<'_, AppState>,
    project_id: String,
    preset_id: String,
    project_type: String,
    active_change_id: Option<String>,
    selected_modules: Option<Vec<String>>,
    disabled_stages: Option<Vec<String>>,
    strict_semantics: Option<bool>,
) -> Result<WorkflowProfile, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let result = if strict_semantics.unwrap_or(false) {
        export_project_workflow_profile_strict(
            &path,
            &preset_id,
            &project_type,
            active_change_id.as_deref(),
            selected_modules.as_deref(),
            disabled_stages.as_deref(),
        )
    } else {
        export_project_workflow_profile(
            &path,
            &preset_id,
            &project_type,
            active_change_id.as_deref(),
            selected_modules.as_deref(),
            disabled_stages.as_deref(),
        )
    };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_flow_config_export_cmd(
    state: State<'_, AppState>,
    project_id: String,
    preset_id: String,
    project_type: String,
    selected_modules: Option<Vec<String>>,
    disabled_stages: Option<Vec<String>>,
) -> Result<FlowWritePlan, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    preview_flow_config_export(
        &path,
        &preset_id,
        &project_type,
        selected_modules.as_deref(),
        disabled_stages.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_flow_config_cmd(
    state: State<'_, AppState>,
    project_id: String,
    preset_id: String,
    project_type: String,
    selected_modules: Option<Vec<String>>,
    disabled_stages: Option<Vec<String>>,
    strict_semantics: Option<bool>,
) -> Result<FlowConfig, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let result = if strict_semantics.unwrap_or(false) {
        export_flow_config_strict(
            &path,
            &preset_id,
            &project_type,
            selected_modules.as_deref(),
            disabled_stages.as_deref(),
        )
    } else {
        export_flow_config(
            &path,
            &preset_id,
            &project_type,
            selected_modules.as_deref(),
            disabled_stages.as_deref(),
        )
    };
    result.map_err(|e| e.to_string())
}
