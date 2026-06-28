//! Skills 命令层
//!
//! v3.10.0+ 统一管理架构：
//! - 支持三应用开关（Claude/Codex/Gemini）
//! - SSOT 存储在 ~/.OpenSunstar/skills/

use crate::app_config::{AppType, InstalledSkill, UnmanagedSkill};
use crate::error::format_skill_error;
use crate::services::skill::{
    ClawHubSearchResult, ClawHubSkillStats, DiscoverableSkill, ImportSkillSelection,
    MigrationResult, ModelScopeSearchResult, Skill, SkillBackupEntry, SkillRepo, SkillService,
    SkillStorageLocation, SkillUninstallResult, SkillUpdateInfo, SkillsShSearchResult,
};
use crate::services::skills_sh_leaderboard::{
    self, SkillsShLeaderboardPeriod, SkillsShLeaderboardResult,
};
use crate::store::AppState;
use std::str::FromStr;
use std::sync::Arc;
use tauri::State;

/// SkillService 状态包装
pub struct SkillServiceState(pub Arc<SkillService>);

/// 解析 app 参数为 AppType
fn parse_app_type(app: &str) -> Result<AppType, String> {
    AppType::from_str(app).map_err(|e| e.to_string())
}

// ========== 统一管理命令 ==========

/// 获取所有已安装的 Skills
#[tauri::command]
pub fn get_installed_skills(app_state: State<'_, AppState>) -> Result<Vec<InstalledSkill>, String> {
    SkillService::get_all_installed(&app_state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skill_backups() -> Result<Vec<SkillBackupEntry>, String> {
    SkillService::list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_skill_backup(backup_id: String) -> Result<bool, String> {
    SkillService::delete_backup(&backup_id).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 安装 Skill（新版统一安装）
///
/// 参数：
/// - skill: 从发现列表获取的技能信息
/// - current_app: 当前选中的应用，安装后默认启用该应用
#[tauri::command]
pub async fn install_skill_unified(
    skill: DiscoverableSkill,
    current_app: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<InstalledSkill, String> {
    let app_type = parse_app_type(&current_app)?;

    service
        .0
        .install(&app_state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())
}

/// 卸载 Skill（新版统一卸载）
#[tauri::command]
pub fn uninstall_skill_unified(
    id: String,
    app_state: State<'_, AppState>,
) -> Result<SkillUninstallResult, String> {
    SkillService::uninstall(&app_state.db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_skill_backup(
    backup_id: String,
    current_app: String,
    app_state: State<'_, AppState>,
) -> Result<InstalledSkill, String> {
    let app_type = parse_app_type(&current_app)?;
    SkillService::restore_from_backup(&app_state.db, &backup_id, &app_type)
        .map_err(|e| e.to_string())
}

/// 切换 Skill 的应用启用状态
#[tauri::command]
pub fn toggle_skill_app(
    id: String,
    app: String,
    enabled: bool,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let app_type = parse_app_type(&app)?;
    SkillService::toggle_app(&app_state.db, &id, &app_type, enabled).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 批量切换 Skills 的应用启用状态
#[tauri::command]
pub fn batch_toggle_skill_app(
    ids: Vec<String>,
    app: String,
    enabled: bool,
    app_state: State<'_, AppState>,
) -> Result<usize, String> {
    let app_type = parse_app_type(&app)?;
    SkillService::batch_toggle_app(&app_state.db, &ids, &app_type, enabled)
        .map_err(|e| e.to_string())
}

/// 扫描未管理的 Skills
#[tauri::command]
pub fn scan_unmanaged_skills(
    app_state: State<'_, AppState>,
) -> Result<Vec<UnmanagedSkill>, String> {
    SkillService::scan_unmanaged(&app_state.db).map_err(|e| e.to_string())
}

/// 从应用目录导入 Skills
#[tauri::command]
pub fn import_skills_from_apps(
    imports: Vec<ImportSkillSelection>,
    app_state: State<'_, AppState>,
) -> Result<Vec<InstalledSkill>, String> {
    SkillService::import_from_apps(&app_state.db, imports).map_err(|e| e.to_string())
}

// ========== 发现功能命令 ==========

/// 发现可安装的 Skills（从仓库获取）
#[tauri::command]
pub async fn discover_available_skills(
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<DiscoverableSkill>, String> {
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())
}

/// 检查 Skills 更新
#[tauri::command]
pub async fn check_skill_updates(
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<SkillUpdateInfo>, String> {
    service
        .0
        .check_updates(&app_state.db)
        .await
        .map_err(|e| e.to_string())
}

/// 更新单个 Skill
#[tauri::command]
pub async fn update_skill(
    id: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<InstalledSkill, String> {
    service
        .0
        .update_skill(&app_state.db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 迁移 Skill 存储位置
#[tauri::command]
pub async fn migrate_skill_storage(
    target: SkillStorageLocation,
    app_state: State<'_, AppState>,
) -> Result<MigrationResult, String> {
    SkillService::migrate_storage(&app_state.db, target).map_err(|e| e.to_string())
}

/// 手动重新同步所有技能的 app 目录（修复断裂的 symlink 等）
#[tauri::command]
pub fn resync_all_skills(
    app_state: State<'_, AppState>,
) -> Result<usize, String> {
    let skills_map = app_state
        .db
        .get_all_installed_skills()
        .map_err(|e| e.to_string())?;

    let mut count = 0usize;
    let app_types = [
        AppType::Claude,
        AppType::Codex,
        AppType::Gemini,
        AppType::OpenCode,
        AppType::Hermes,
    ];

    for skill in skills_map.values() {
        for app in &app_types {
            let app_key = match app {
                AppType::Claude => "claude",
                AppType::Codex => "codex",
                AppType::Gemini => "gemini",
                AppType::OpenCode => "opencode",
                AppType::Hermes => "hermes",
                _ => continue,
            };
            let enabled = match app_key {
                "claude" => skill.apps.claude,
                "codex" => skill.apps.codex,
                "gemini" => skill.apps.gemini,
                "opencode" => skill.apps.opencode,
                "hermes" => skill.apps.hermes,
                _ => false,
            };
            if enabled {
                if SkillService::sync_to_app_dir(&skill.directory, app).is_ok() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

/// 搜索 skills.sh 公共目录
#[tauri::command]
pub async fn search_skills_sh(
    query: String,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult, String> {
    SkillService::search_skills_sh(&query, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

/// 获取 skills.sh 官方排行榜（All Time / Trending 24h TOP50）
#[tauri::command]
pub async fn get_skills_sh_leaderboard(
    period: String,
    force_refresh: Option<bool>,
) -> Result<SkillsShLeaderboardResult, String> {
    let period = SkillsShLeaderboardPeriod::parse(&period).map_err(|e| e.to_string())?;
    skills_sh_leaderboard::get_skills_sh_leaderboard(period, force_refresh.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

/// 搜索 ClawHub 公共目录
#[tauri::command]
pub async fn search_clawhub(
    query: String,
    limit: usize,
) -> Result<ClawHubSearchResult, String> {
    SkillService::search_clawhub(&query, limit)
        .await
        .map_err(|e| e.to_string())
}

/// 批量获取 ClawHub 技能的星标/下载/安装量
#[tauri::command]
pub async fn batch_get_clawhub_stats(
    slugs: Vec<String>,
) -> Result<Vec<ClawHubSkillStats>, String> {
    SkillService::batch_get_clawhub_stats(&slugs)
        .await
        .map_err(|e| e.to_string())
}

/// 静默安装 ClawHub 技能（执行 npx clawhub@latest install <slug>）
#[tauri::command]
pub async fn install_clawhub_skill(slug: String) -> Result<(), String> {
    // 验证 slug 安全性：不允许路径分隔符或特殊字符
    if slug.is_empty()
        || slug.len() > 200
        || slug.contains(['/', '\\', ';', '&', '|', '`', '$', '\'', '"', '<', '>', '\n', '\r'])
    {
        return Err("无效的 ClawHub 技能名称".to_string());
    }

    let command = format!("npx clawhub@latest install {}", slug);

    tokio::task::spawn_blocking(move || run_cli_silently(&command, "clawhub_install"))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 搜索 ModelScope 技能中心
#[tauri::command]
pub async fn search_modelscope(
    query: String,
    page_number: usize,
    page_size: usize,
) -> Result<ModelScopeSearchResult, String> {
    SkillService::search_modelscope(&query, page_number, page_size)
        .await
        .map_err(|e| e.to_string())
}

/// 静默执行 CLI 命令（捕获输出，不弹窗口）
#[cfg(target_os = "windows")]
fn run_cli_silently(command_line: &str, label: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let bat_file =
        std::env::temp_dir().join(format!("OpenSunstar_{}_{}.bat", label, std::process::id()));
    let bat_content = format!("@echo off\r\n{}\r\n", command_line);
    std::fs::write(&bat_file, bat_content)
        .map_err(|e| format!("写入批处理文件失败: {e}"))?;

    let output = Command::new("cmd")
        .arg("/C")
        .arg(&bat_file)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let _ = std::fs::remove_file(&bat_file);

    let output = output.map_err(|e| format!("启动安装进程失败: {e}"))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    // 提取末尾最多 500 字符
    let tail = if detail.len() > 500 {
        &detail[detail.len() - 500..]
    } else {
        detail
    };
    Err(format!("安装失败:\n{}", tail))
}

#[cfg(not(target_os = "windows"))]
fn run_cli_silently(command_line: &str, _label: &str) -> Result<(), String> {
    use std::process::Command;

    let output = Command::new("bash")
        .arg("-c")
        .arg(command_line)
        .output()
        .map_err(|e| format!("启动安装进程失败: {e}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    let tail = if detail.len() > 500 {
        &detail[detail.len() - 500..]
    } else {
        detail
    };
    Err(format!("安装失败:\n{}", tail))
}

// ========== 兼容旧 API 的命令 ==========

/// 获取技能列表（兼容旧 API）
#[tauri::command]
pub async fn get_skills(
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<Skill>, String> {
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    service
        .0
        .list_skills(repos, &app_state.db)
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定应用的技能列表（兼容旧 API）
#[tauri::command]
pub async fn get_skills_for_app(
    app: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<Skill>, String> {
    // 新版本不再区分应用，统一返回所有技能
    let _ = parse_app_type(&app)?; // 验证 app 参数有效
    get_skills(service, app_state).await
}

/// 安装技能（兼容旧 API）
#[tauri::command]
pub async fn install_skill(
    directory: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    install_skill_for_app("claude".to_string(), directory, service, app_state).await
}

/// 安装指定应用的技能（兼容旧 API）
#[tauri::command]
pub async fn install_skill_for_app(
    app: String,
    directory: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let app_type = parse_app_type(&app)?;

    // 先获取技能信息
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    let skills = service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())?;

    let skill = skills
        .into_iter()
        .find(|s| {
            let install_name = std::path::Path::new(&s.directory)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| s.directory.clone());
            install_name.eq_ignore_ascii_case(&directory)
                || s.directory.eq_ignore_ascii_case(&directory)
        })
        .ok_or_else(|| {
            format_skill_error(
                "SKILL_NOT_FOUND",
                &[("directory", &directory)],
                Some("checkRepoUrl"),
            )
        })?;

    service
        .0
        .install(&app_state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// 卸载技能（兼容旧 API）
#[tauri::command]
pub fn uninstall_skill(
    directory: String,
    app_state: State<'_, AppState>,
) -> Result<SkillUninstallResult, String> {
    uninstall_skill_for_app("claude".to_string(), directory, app_state)
}

/// 卸载指定应用的技能（兼容旧 API）
#[tauri::command]
pub fn uninstall_skill_for_app(
    app: String,
    directory: String,
    app_state: State<'_, AppState>,
) -> Result<SkillUninstallResult, String> {
    let _ = parse_app_type(&app)?; // 验证参数

    // 通过 directory 找到对应的 skill id
    let skills = SkillService::get_all_installed(&app_state.db).map_err(|e| e.to_string())?;

    let skill = skills
        .into_iter()
        .find(|s| s.directory.eq_ignore_ascii_case(&directory))
        .ok_or_else(|| format!("未找到已安装的 Skill: {directory}"))?;

    SkillService::uninstall(&app_state.db, &skill.id).map_err(|e| e.to_string())
}

// ========== 仓库管理命令 ==========

/// 获取技能仓库列表
#[tauri::command]
pub fn get_skill_repos(app_state: State<'_, AppState>) -> Result<Vec<SkillRepo>, String> {
    app_state.db.get_skill_repos().map_err(|e| e.to_string())
}

/// 添加技能仓库
#[tauri::command]
pub fn add_skill_repo(repo: SkillRepo, app_state: State<'_, AppState>) -> Result<bool, String> {
    app_state
        .db
        .save_skill_repo(&repo)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 删除技能仓库
#[tauri::command]
pub fn remove_skill_repo(
    owner: String,
    name: String,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    app_state
        .db
        .delete_skill_repo(&owner, &name)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 启用/禁用技能仓库
#[tauri::command]
pub fn toggle_skill_repo(
    owner: String,
    name: String,
    enabled: bool,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    if let Some(repo) = repos
        .iter_mut()
        .find(|r| r.owner == owner && r.name == name)
    {
        repo.enabled = enabled;
        app_state
            .db
            .save_skill_repo(repo)
            .map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 从 ZIP 文件安装 Skills
#[tauri::command]
pub fn install_skills_from_zip(
    file_path: String,
    current_app: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<InstalledSkill>, String> {
    let app_type = parse_app_type(&current_app)?;
    let path = std::path::Path::new(&file_path);

    SkillService::install_from_zip(&app_state.db, path, &app_type).map_err(|e| e.to_string())
}
