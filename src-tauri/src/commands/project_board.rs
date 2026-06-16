//! 项目看板 Tauri 命令
//! 从 AIControls v0.2.1 移植

use crate::project_metrics;

/// 使用 tokei 统计项目目录的代码行数。
#[tauri::command]
pub async fn count_project_code_lines(root: String) -> Result<project_metrics::CodeLineResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project_metrics::count_code_lines(std::path::Path::new(root.trim()))
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 读取项目目录下 package.json 中的 version 字段。
#[tauri::command]
pub async fn read_package_version(root: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project_metrics::read_package_version(std::path::Path::new(root.trim()))
    })
    .await
    .map_err(|e| format!("读取任务失败: {e}"))?
}

/// 检测 Git 仓库信息（分支、远程、最近提交等）。
#[tauri::command]
pub async fn detect_project_git_info(root: String) -> Result<project_metrics::ProjectGitInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project_metrics::detect_git_info(std::path::Path::new(root.trim()))
    })
    .await
    .map_err(|e| format!("检测任务失败: {e}"))?
}

/// 统计近 N 天内的 Git 提交数量（用于计算项目活跃度）。
#[tauri::command]
pub async fn git_commit_count_last_n_days(root: String, days: u32) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        if !dir.is_dir() {
            return Err("路径不是文件夹".into());
        }
        Ok(project_metrics::git_commit_count_last_n_days(dir, days))
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 返回最近 12 周每周的提交数量（从最旧到最新）。
#[tauri::command]
pub async fn git_weekly_commit_counts(root: String) -> Result<Vec<u32>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        Ok(project_metrics::git_weekly_commit_counts(dir))
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 返回 Git 仓库的贡献者列表（按提交数降序）。
#[tauri::command]
pub async fn git_contributors(root: String) -> Result<Vec<project_metrics::Contributor>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = std::path::Path::new(root.trim());
        Ok(project_metrics::git_contributors(dir))
    })
    .await
    .map_err(|e| format!("统计任务失败: {e}"))?
}

/// 在系统文件管理器中打开并选中指定路径。
#[tauri::command]
pub fn reveal_path_in_folder(path: String) -> Result<(), String> {
    project_metrics::reveal_path_in_folder(path.trim())
}
