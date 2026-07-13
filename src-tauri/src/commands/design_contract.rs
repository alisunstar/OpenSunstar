//! Design Contract commands: template management, compose, export, import, install.

use tauri::State;

use crate::database::Database;
use crate::services::design_contract::{
    compose_design_contract, export_design_contract, generate_design_md, generate_dtchg_json,
    get_design_template, import_design_from_content, import_design_from_file,
    install_design_contract, list_design_templates, preview_export_plan, preview_install_plan,
    verify_design_system_manifest, DesignContract, DesignContractParams, DesignInstallPlan,
    DesignInstallResult, DesignSystemVerification, ImportResult,
};
use crate::services::design_system_registry::{
    discover_design_systems, load_design_system_contract, load_design_system_package_detail,
    DesignSystemDiscovery, DesignSystemPackageDetail,
};
use crate::store::AppState;

fn project_path_for_id(db: &Database, project_id: &str) -> Result<String, String> {
    db.get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
        .map(|p| p.path)
}

/// List all built-in design templates (returns vec of (id, name)).
#[tauri::command]
pub async fn list_design_templates_cmd() -> Result<Vec<(String, String)>, String> {
    Ok(list_design_templates())
}

/// Discover packaged and user-installed offline design-system packages.
#[tauri::command]
pub async fn list_design_system_packages_cmd() -> Result<DesignSystemDiscovery, String> {
    discover_design_systems().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_design_system_package_contract_cmd(
    package_id: String,
) -> Result<DesignContract, String> {
    load_design_system_contract(&package_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_design_system_package_detail_cmd(
    package_id: String,
) -> Result<DesignSystemPackageDetail, String> {
    load_design_system_package_detail(&package_id).map_err(|e| e.to_string())
}

/// Get a specific built-in template by ID.
#[tauri::command]
pub async fn get_design_template_cmd(template_id: String) -> Result<DesignContract, String> {
    get_design_template(&template_id).ok_or_else(|| format!("模板不存在: {template_id}"))
}

/// Compose a design contract from parameters (no disk write).
#[tauri::command]
pub async fn compose_design_contract_cmd(
    params: DesignContractParams,
) -> Result<DesignContract, String> {
    compose_design_contract(&params).map_err(|e| e.to_string())
}

/// Preview the DESIGN.md output (no disk write).
#[tauri::command]
pub async fn preview_design_md_cmd(params: DesignContractParams) -> Result<String, String> {
    let contract = compose_design_contract(&params).map_err(|e| e.to_string())?;
    generate_design_md(&contract).map_err(|e| e.to_string())
}

/// Preview the DTCG JSON output (no disk write).
#[tauri::command]
pub async fn preview_dtchg_json_cmd(params: DesignContractParams) -> Result<String, String> {
    let contract = compose_design_contract(&params).map_err(|e| e.to_string())?;
    generate_dtchg_json(&contract).map_err(|e| e.to_string())
}

/// Preview overwrite-style export plan (no disk write to project).
#[tauri::command]
pub async fn preview_design_export_plan_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: DesignContractParams,
) -> Result<DesignInstallPlan, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let contract = compose_design_contract(&params).map_err(|e| e.to_string())?;
    preview_export_plan(&path, &contract).map_err(|e| e.to_string())
}

/// Export: compose + write DESIGN.md to project root + archive in .opensunstar/contract/.
#[tauri::command]
pub async fn export_design_contract_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: DesignContractParams,
) -> Result<String, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let contract = compose_design_contract(&params).map_err(|e| e.to_string())?;
    export_design_contract(&path, &contract).map_err(|e| e.to_string())
}

/// Preview install plan: pre-flight dry run with audit scan (no disk write to project).
#[tauri::command]
pub async fn preview_design_install_plan_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: DesignContractParams,
) -> Result<DesignInstallPlan, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let contract = compose_design_contract(&params).map_err(|e| e.to_string())?;
    preview_install_plan(&path, &contract).map_err(|e| e.to_string())
}

/// Install: write DESIGN.md + design-tokens.json to project (safe, never overwrites).
#[tauri::command]
pub async fn install_design_contract_cmd(
    state: State<'_, AppState>,
    project_id: String,
    params: DesignContractParams,
) -> Result<DesignInstallResult, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    let contract = compose_design_contract(&params).map_err(|e| e.to_string())?;
    install_design_contract(&path, &contract).map_err(|e| e.to_string())
}

/// Verify the project files against the selected design-system manifest (read-only).
#[tauri::command]
pub async fn verify_design_system_manifest_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<DesignSystemVerification, String> {
    let path = project_path_for_id(&state.db, &project_id)?;
    verify_design_system_manifest(&path).map_err(|e| e.to_string())
}

/// Import a DESIGN.md from a local file path.
#[tauri::command]
pub async fn import_design_from_file_cmd(file_path: String) -> Result<ImportResult, String> {
    import_design_from_file(&file_path).map_err(|e| e.to_string())
}

/// Import a DESIGN.md from URL content (content is fetched by frontend, passed here).
#[tauri::command]
pub async fn import_design_from_url_cmd(
    content: String,
    source_url: String,
    source_kind: String,
) -> Result<ImportResult, String> {
    import_design_from_content(&content, &source_url, &source_kind).map_err(|e| e.to_string())
}
