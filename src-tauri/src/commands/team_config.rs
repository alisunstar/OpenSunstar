//! 团队配置 Tauri 命令（Local Alpha L6）
//!
//! 只读命令集：connect / validate / list_profiles / get_effective_state / status
//! 写入命令（deploy/rollback）在 Git MVP 阶段追加。

use serde::Serialize;
use tauri::State;

use crate::database::TeamWorkspace;
use crate::store::AppState;
use crate::team_config::{
    compile_effective_config, connect_team_source, diff_lock_vs_directory, generate_assignment_id,
    parse_team_package, validate_assignment, CompilerInput, ConnectResult, EffectiveConfig,
    ReleaseDiff, TargetApp, TeamProfile, ValidationOptions, ValidationReport,
};

// ─── 响应类型 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamConnectResponse {
    pub workspace_id: String,
    pub name: String,
    pub source_kind: String,
    pub source_path: String,
    pub branch: Option<String>,
    pub head_commit: Option<String>,
    pub profiles_count: usize,
    pub policies_count: usize,
    pub credential_slots_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamValidateResponse {
    pub passed: bool,
    pub errors: Vec<TeamValidationIssue>,
    pub warnings: Vec<TeamValidationIssue>,
    pub security_blocked: bool,
    pub files_scanned: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamValidationIssue {
    pub code: String,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamProfileSummary {
    pub profile_id: String,
    pub name: String,
    pub description: Option<String>,
    pub assets_count: usize,
    pub credential_slots_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatusResponse {
    pub connected: bool,
    pub workspace_id: Option<String>,
    pub name: Option<String>,
    pub source_kind: Option<String>,
    pub source_path: Option<String>,
    pub branch: Option<String>,
    pub head_commit: Option<String>,
    pub is_clean: Option<bool>,
    pub can_pull: Option<bool>,
    pub profiles_count: usize,
    pub validation_passed: Option<bool>,
}

// ─── Tauri 命令 ─────────────────────────────────────────────────────────────

/// 连接团队配置源（只读连接 + 持久化工作区元数据）
#[tauri::command]
pub fn connect_team_workspace(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<TeamConnectResponse, String> {
    let path = std::path::Path::new(&path);
    let result: ConnectResult = connect_team_source(path).map_err(|e| e.to_string())?;

    // 读取原始 team.toml 用于快照（仅一次 I/O）
    let toml_content =
        std::fs::read_to_string(path.join("team.toml")).map_err(|e| e.to_string())?;

    // 持久化工作区元数据
    let now = chrono::Utc::now().timestamp();
    let ws = TeamWorkspace {
        workspace_id: result.workspace.workspace_id.clone(),
        name: result.workspace.name.clone(),
        source_kind: result.workspace.source_kind.as_str().to_string(),
        source_path: result.workspace.source_path.clone(),
        branch: result.workspace.branch.clone(),
        last_synced_commit: result.workspace.last_synced_commit.clone(),
        last_synced_at: Some(now),
        team_toml_snapshot: Some(toml_content),
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    };
    app_state
        .db
        .upsert_team_workspace(&ws)
        .map_err(|e| e.to_string())?;

    Ok(TeamConnectResponse {
        workspace_id: result.workspace.workspace_id,
        name: result.workspace.name,
        source_kind: result.workspace.source_kind.as_str().to_string(),
        source_path: result.workspace.source_path,
        branch: result.workspace.branch,
        head_commit: result.workspace.last_synced_commit,
        profiles_count: result.team_toml.profiles.len(),
        policies_count: result.team_toml.policies.len(),
        credential_slots_count: result.team_toml.credential_slots.len(),
        warnings: result.warnings.iter().map(|w| w.to_string()).collect(),
    })
}

/// 校验团队配置包
#[tauri::command]
pub fn validate_team_workspace(
    path: String,
    run_security_scan: Option<bool>,
    _app_state: State<'_, AppState>,
) -> Result<TeamValidateResponse, String> {
    let path = std::path::Path::new(&path);
    let options = ValidationOptions {
        run_security_scan: run_security_scan.unwrap_or(true),
        check_asset_files: true,
    };

    let report: ValidationReport =
        crate::team_config::validate_team_package_dir(path, &options).map_err(|e| e.to_string())?;

    Ok(TeamValidateResponse {
        passed: report.passed,
        errors: report
            .errors
            .iter()
            .map(|i| TeamValidationIssue {
                code: i.code.as_str().to_string(),
                message: i.message.clone(),
                location: i.location.clone(),
            })
            .collect(),
        warnings: report
            .warnings
            .iter()
            .map(|i| TeamValidationIssue {
                code: i.code.as_str().to_string(),
                message: i.message.clone(),
                location: i.location.clone(),
            })
            .collect(),
        security_blocked: report.security.as_ref().map(|s| s.blocked).unwrap_or(false),
        files_scanned: report.security.as_ref().map(|s| s.files_scanned),
    })
}

/// 列出团队 Profile
#[tauri::command]
pub fn list_team_profiles(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<Vec<TeamProfileSummary>, String> {
    let path = std::path::Path::new(&path);
    let content = std::fs::read_to_string(path.join("team.toml")).map_err(|e| e.to_string())?;
    let (profiles, _, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    Ok(profiles
        .iter()
        .map(|p: &TeamProfile| TeamProfileSummary {
            profile_id: p.profile_id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            assets_count: p.assets.len(),
            credential_slots_count: p.credential_slots.len(),
        })
        .collect())
}

/// 获取有效配置（只读编译，不写入）
#[tauri::command]
pub fn get_team_effective_state(
    path: String,
    target_app: String,
    project_id: Option<String>,
    _app_state: State<'_, AppState>,
) -> Result<EffectiveConfig, String> {
    let path = std::path::Path::new(&path);
    let content = std::fs::read_to_string(path.join("team.toml")).map_err(|e| e.to_string())?;
    let (profiles, policies, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    let input = CompilerInput {
        team_profiles: profiles,
        team_policies: policies,
        project_assets: vec![],
        personal_overrides: vec![],
        target_app: TargetApp::from_str(&target_app),
        project_id: project_id.unwrap_or_else(|| "default".to_string()),
    };

    Ok(compile_effective_config(&input))
}

/// 团队配置状态概览（CLI `os team status` 的后端）
#[tauri::command]
pub fn get_team_status(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<TeamStatusResponse, String> {
    let path = std::path::Path::new(&path);

    match connect_team_source(path) {
        Ok(result) => {
            let options = ValidationOptions {
                run_security_scan: false,
                check_asset_files: true,
            };
            let validation = crate::team_config::validate_team_package_dir(path, &options).ok();

            Ok(TeamStatusResponse {
                connected: true,
                workspace_id: Some(result.workspace.workspace_id),
                name: Some(result.workspace.name),
                source_kind: Some(result.workspace.source_kind.as_str().to_string()),
                source_path: Some(result.workspace.source_path),
                branch: result.workspace.branch,
                head_commit: result.workspace.last_synced_commit,
                is_clean: result.git_safety.as_ref().map(|s| s.is_clean),
                can_pull: result.git_safety.as_ref().map(|s| s.can_pull()),
                profiles_count: result.team_toml.profiles.len(),
                validation_passed: validation.map(|v| v.passed),
            })
        }
        Err(_) => Ok(TeamStatusResponse {
            connected: false,
            workspace_id: None,
            name: None,
            source_kind: None,
            source_path: Some(path.display().to_string()),
            branch: None,
            head_commit: None,
            is_clean: None,
            can_pull: None,
            profiles_count: 0,
            validation_passed: None,
        }),
    }
}

/// Release Diff：比对 lock.json 与当前目录，显示自上次发布以来的变更
#[tauri::command]
pub fn get_team_release_diff(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<ReleaseDiff, String> {
    let path = std::path::Path::new(&path);

    // 尝试读取已有 lock.json 作为基线
    let lock_path = path.join("lock.json");
    if !lock_path.exists() {
        return Err("未找到 lock.json 基线文件，请先执行 release 生成".to_string());
    }

    let lock_content =
        std::fs::read_to_string(&lock_path).map_err(|e| format!("读取 lock.json 失败: {e}"))?;
    let lock: crate::team_config::ReleaseLock =
        serde_json::from_str(&lock_content).map_err(|e| format!("解析 lock.json 失败: {e}"))?;

    diff_lock_vs_directory(&lock, path).map_err(|e| e.to_string())
}

/// 分配 Profile 到项目（M1：单 Profile 约束）
#[tauri::command]
pub fn assign_team_profile(
    project_id: String,
    workspace_id: String,
    profile_id: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // 校验工作区存在
    let ws = app_state
        .db
        .get_team_workspace(&workspace_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("workspace '{workspace_id}' not found"))?;

    // 校验 profile 存在于工作区的 team.toml 快照中
    let available_profiles: Vec<String> = if let Some(snapshot) = &ws.team_toml_snapshot {
        parse_team_package(snapshot)
            .map(|(profiles, _, _)| profiles.iter().map(|p| p.profile_id.clone()).collect())
            .unwrap_or_default()
    } else {
        vec![]
    };
    validate_assignment(&profile_id, &available_profiles).map_err(|e| e.to_string())?;

    // 单 Profile 约束：检查是否已有活跃分配
    if let Some(existing) = app_state
        .db
        .get_active_team_assignment(&project_id)
        .map_err(|e| e.to_string())?
    {
        if existing.workspace_id != workspace_id || existing.profile_id != profile_id {
            return Err(format!(
                "project already assigned to profile '{}' (single-profile constraint)",
                existing.profile_id
            ));
        }
        // 相同分配，幂等返回
        return Ok(serde_json::json!({
            "assignmentId": existing.assignment_id,
            "changed": false,
        }));
    }

    let now = chrono::Utc::now().timestamp();
    let assignment_id = generate_assignment_id(&project_id, &workspace_id);
    app_state
        .db
        .upsert_team_assignment(
            &assignment_id,
            &project_id,
            &workspace_id,
            &profile_id,
            "active",
            now,
            now,
        )
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "assignmentId": assignment_id,
        "changed": true,
    }))
}

/// 查询项目的活跃分配
#[tauri::command]
pub fn get_team_assignment(
    project_id: String,
    app_state: State<'_, AppState>,
) -> Result<Option<crate::database::TeamAssignmentRow>, String> {
    app_state
        .db
        .get_active_team_assignment(&project_id)
        .map_err(|e| e.to_string())
}

/// 取消分配（软删除：status → removed）
#[tauri::command]
pub fn unassign_team_profile(
    project_id: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let existing = app_state
        .db
        .get_active_team_assignment(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no active assignment for this project".to_string())?;

    app_state
        .db
        .update_team_assignment_status(&existing.assignment_id, "removed")
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "removed": true }))
}

/// 生成部署计划（M2：文件级 diff + 风险标注 + plan_sha256）
#[tauri::command]
pub fn generate_team_deployment_plan(
    path: String,
    target_app: String,
    project_root: String,
    project_id: Option<String>,
    _app_state: State<'_, AppState>,
) -> Result<crate::team_config::DeploymentPlan, String> {
    let team_path = std::path::Path::new(&path);
    let project_root = std::path::Path::new(&project_root);

    if !project_root.is_dir() {
        return Err(format!(
            "项目路径不存在或不是目录: {}",
            project_root.display()
        ));
    }

    // 编译有效配置
    let content = std::fs::read_to_string(team_path.join("team.toml"))
        .map_err(|e| format!("读取 team.toml 失败: {e}"))?;
    let (profiles, policies, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    let input = CompilerInput {
        team_profiles: profiles,
        team_policies: policies,
        project_assets: vec![],
        personal_overrides: vec![],
        target_app: TargetApp::from_str(&target_app),
        project_id: project_id.unwrap_or_else(|| "default".to_string()),
    };

    let config = compile_effective_config(&input);

    // 生成部署计划
    Ok(crate::team_config::generate_deployment_plan(
        &config,
        project_root,
    ))
}

/// 执行部署计划（M3+M4：写入 + 回执验证）
#[tauri::command]
pub fn execute_team_deployment(
    path: String,
    target_app: String,
    project_root: String,
    project_id: Option<String>,
    dry_run: Option<bool>,
    _app_state: State<'_, AppState>,
) -> Result<crate::team_config::DeploymentReceipt, String> {
    let team_path = std::path::Path::new(&path);
    let project_root_path = std::path::Path::new(&project_root);

    if !project_root_path.is_dir() {
        return Err(format!(
            "项目路径不存在或不是目录: {}",
            project_root_path.display()
        ));
    }

    // 编译有效配置
    let content = std::fs::read_to_string(team_path.join("team.toml"))
        .map_err(|e| format!("读取 team.toml 失败: {e}"))?;
    let (profiles, policies, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    let input = CompilerInput {
        team_profiles: profiles,
        team_policies: policies,
        project_assets: vec![],
        personal_overrides: vec![],
        target_app: TargetApp::from_str(&target_app),
        project_id: project_id.unwrap_or_else(|| "default".to_string()),
    };

    let config = compile_effective_config(&input);

    // 生成部署计划
    let plan = crate::team_config::generate_deployment_plan(&config, project_root_path);

    // 执行
    let options = crate::team_config::ExecuteOptions {
        team_package_root: team_path.to_path_buf(),
        dry_run: dry_run.unwrap_or(false),
        create_backup: true,
    };

    Ok(crate::team_config::execute_deployment_plan(
        &plan,
        project_root_path,
        &options,
    ))
}

/// 偏差检测（M5：期望 vs 实际）
#[tauri::command]
pub fn check_team_drift(
    receipt_json: String,
    project_root: String,
    _app_state: State<'_, AppState>,
) -> Result<crate::team_config::DriftReport, String> {
    let project_root = std::path::Path::new(&project_root);
    if !project_root.is_dir() {
        return Err(format!(
            "项目路径不存在或不是目录: {}",
            project_root.display()
        ));
    }

    let receipt: crate::team_config::DeploymentReceipt =
        serde_json::from_str(&receipt_json).map_err(|e| format!("解析部署回执失败: {e}"))?;

    Ok(crate::team_config::detect_drift(&receipt, project_root))
}

/// 安全回滚（M6：零写入阻断）
#[tauri::command]
pub fn rollback_team_deployment(
    receipt_json: String,
    drift_json: String,
    project_root: String,
    _app_state: State<'_, AppState>,
) -> Result<crate::team_config::RollbackReport, String> {
    let project_root = std::path::Path::new(&project_root);
    if !project_root.is_dir() {
        return Err(format!(
            "项目路径不存在或不是目录: {}",
            project_root.display()
        ));
    }

    let receipt: crate::team_config::DeploymentReceipt =
        serde_json::from_str(&receipt_json).map_err(|e| format!("解析部署回执失败: {e}"))?;
    let drift: crate::team_config::DriftReport =
        serde_json::from_str(&drift_json).map_err(|e| format!("解析偏差报告失败: {e}"))?;

    Ok(crate::team_config::execute_rollback(
        &receipt,
        &drift,
        project_root,
    ))
}
