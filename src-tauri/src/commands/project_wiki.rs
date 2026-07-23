//! Project Wiki Tauri 命令层

use tauri::State;

use crate::error::AppError;
use crate::services::project_wiki;
use crate::store::AppState;

/// 扫描项目 Wiki 状态
#[tauri::command]
pub async fn scan_project_wiki_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<project_wiki::WikiScanResult, AppError> {
    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| AppError::Message(format!("查询项目失败: {e}")))?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;

    project_wiki::scan_project_wiki(&project.path, &project_id)
}

/// 构建 Wiki Inventory
#[tauri::command]
pub async fn inventory_project_wiki_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<project_wiki::WikiInventory, AppError> {
    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| AppError::Message(format!("查询项目失败: {e}")))?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;

    project_wiki::build_wiki_inventory(&project.path, &project_id)
}

/// 运行 Wiki Lint 校验
#[tauri::command]
pub async fn run_project_wiki_lint_cmd(
    state: State<'_, AppState>,
    project_id: String,
    quality_mode: Option<bool>,
) -> Result<project_wiki::WikiLintResult, AppError> {
    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| AppError::Message(format!("查询项目失败: {e}")))?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;

    project_wiki::run_wiki_lint(&project.path, &project_id, quality_mode.unwrap_or(false))
}

/// 预览 Wiki 初始化
#[tauri::command]
pub async fn preview_project_wiki_init_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<project_wiki::WikiInitPlan, AppError> {
    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| AppError::Message(format!("查询项目失败: {e}")))?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;

    project_wiki::preview_wiki_init(&project.path, &project_id)
}

/// 初始化 Wiki
#[tauri::command]
pub async fn init_project_wiki_cmd(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<project_wiki::WikiInitResult, AppError> {
    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| AppError::Message(format!("查询项目失败: {e}")))?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;

    project_wiki::init_project_wiki(&project.path, &project_id, &project.name)
}

/// 映射变更文件到 Wiki 页面
#[tauri::command]
pub async fn map_project_wiki_changed_files_cmd(
    state: State<'_, AppState>,
    project_id: String,
    changed_files: Option<Vec<String>>,
) -> Result<project_wiki::WikiChangedFilesResult, AppError> {
    let project = state
        .db
        .get_project(&project_id)
        .map_err(|e| AppError::Message(format!("查询项目失败: {e}")))?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;

    project_wiki::map_wiki_changed_files(&project.path, changed_files)
}
