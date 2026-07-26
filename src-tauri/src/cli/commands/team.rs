//! `os team` — 团队配置管理（Local Alpha L6）
//!
//! 只读三件套（评审问题 6）：
//! - `os team status --json`  — 连接状态概览
//! - `os team validate --json` — 校验团队包（Schema + 安全扫描）
//! - `os team explain --json` — 有效配置解释（编译器输出）
//!
//! 写入命令（deploy/drift/rollback）在 Git MVP 阶段追加。
//! 命名空间说明：`os team profile` 指团队岗位档案，与 `os profile`（项目蓝图）无关。

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct TeamArgs {
    #[command(subcommand)]
    pub action: TeamAction,
}

#[derive(Subcommand)]
pub enum TeamAction {
    /// 团队配置源连接状态概览
    Status {
        /// 团队配置包路径（默认当前目录）
        #[arg(long, default_value = ".")]
        path: String,
    },
    /// 校验团队配置包（Schema + 资产引用 + 安全扫描）
    Validate {
        /// 团队配置包路径
        #[arg(long, default_value = ".")]
        path: String,
        /// 跳过安全扫描（加速）
        #[arg(long)]
        skip_security: bool,
    },
    /// 有效配置解释（编译器输出：每个资产的决策 + 来源链）
    Explain {
        /// 团队配置包路径
        #[arg(long, default_value = ".")]
        path: String,
        /// 目标工具（claude_code / codex）
        #[arg(long, default_value = "claude_code")]
        target: String,
        /// 项目 ID（可选）
        #[arg(long)]
        project: Option<String>,
    },
    /// 列出团队 Profile（岗位档案）
    Profiles {
        /// 团队配置包路径
        #[arg(long, default_value = ".")]
        path: String,
    },
    /// Release Diff：比对 lock.json 基线与当前目录变更
    Diff {
        /// 团队配置包路径
        #[arg(long, default_value = ".")]
        path: String,
    },
    /// 分配 Profile 到项目（单 Profile 约束）
    Assign {
        /// 项目 ID
        #[arg(long)]
        project_id: String,
        /// 工作区 ID
        #[arg(long)]
        workspace_id: String,
        /// Profile ID
        #[arg(long)]
        profile_id: String,
    },
    /// 取消项目分配
    Unassign {
        /// 项目 ID
        #[arg(long)]
        project_id: String,
    },
    /// 生成部署计划（文件级 diff + 风险标注 + plan_sha256）
    Plan {
        /// 团队配置包路径
        #[arg(long, default_value = ".")]
        path: String,
        /// 目标工具（claude_code / codex）
        #[arg(long, default_value = "claude_code")]
        target: String,
        /// 项目根目录（部署目标）
        #[arg(long)]
        project_root: String,
        /// 项目 ID（可选）
        #[arg(long)]
        project: Option<String>,
    },
    /// 执行部署（写入 + 回执验证）
    Deploy {
        /// 团队配置包路径
        #[arg(long, default_value = ".")]
        path: String,
        /// 目标工具（claude_code / codex）
        #[arg(long, default_value = "claude_code")]
        target: String,
        /// 项目根目录（部署目标）
        #[arg(long)]
        project_root: String,
        /// 项目 ID（可选）
        #[arg(long)]
        project: Option<String>,
        /// 预演模式（不实际写入）
        #[arg(long)]
        dry_run: bool,
    },
    /// 偏差检测（部署后文件是否被外部修改）
    Drift {
        /// 部署回执 JSON 文件路径
        #[arg(long)]
        receipt: String,
        /// 项目根目录
        #[arg(long)]
        project_root: String,
    },
    /// 安全回滚（利用备份恢复偏差文件）
    Rollback {
        /// 部署回执 JSON 文件路径
        #[arg(long)]
        receipt: String,
        /// 偏差报告 JSON 文件路径（可选，不提供则自动检测）
        #[arg(long)]
        drift: Option<String>,
        /// 项目根目录
        #[arg(long)]
        project_root: String,
    },
}

pub fn run(args: TeamArgs, state: &open_sunstar_lib::AppState, json: bool) -> Result<(), String> {
    match args.action {
        TeamAction::Status { path } => run_status(&path, json),
        TeamAction::Validate {
            path,
            skip_security,
        } => run_validate(&path, skip_security, json),
        TeamAction::Explain {
            path,
            target,
            project,
        } => run_explain(&path, &target, project, json),
        TeamAction::Profiles { path } => run_profiles(&path, json),
        TeamAction::Diff { path } => run_diff(&path, json),
        TeamAction::Assign {
            project_id,
            workspace_id,
            profile_id,
        } => run_assign(state, &project_id, &workspace_id, &profile_id, json),
        TeamAction::Unassign { project_id } => run_unassign(state, &project_id, json),
        TeamAction::Plan {
            path,
            target,
            project_root,
            project,
        } => run_plan(&path, &target, &project_root, project, json),
        TeamAction::Deploy {
            path,
            target,
            project_root,
            project,
            dry_run,
        } => run_deploy(&path, &target, &project_root, project, dry_run, json),
        TeamAction::Drift {
            receipt,
            project_root,
        } => run_drift(&receipt, &project_root, json),
        TeamAction::Rollback {
            receipt,
            drift,
            project_root,
        } => run_rollback(&receipt, drift, &project_root, json),
    }
}

fn run_status(path: &str, json: bool) -> Result<(), String> {
    use open_sunstar_lib::team_config::connect_team_source;

    let dir = std::path::Path::new(path);
    match connect_team_source(dir) {
        Ok(result) => {
            let status = serde_json::json!({
                "connected": true,
                "workspaceId": result.workspace.workspace_id,
                "name": result.workspace.name,
                "sourceKind": result.workspace.source_kind.as_str(),
                "sourcePath": result.workspace.source_path,
                "branch": result.workspace.branch,
                "headCommit": result.workspace.last_synced_commit,
                "isClean": result.git_safety.as_ref().map(|s| s.is_clean),
                "canPull": result.git_safety.as_ref().map(|s| s.can_pull()),
                "profilesCount": result.team_toml.profiles.len(),
                "policiesCount": result.team_toml.policies.len(),
                "warnings": result.warnings.iter().map(|w| w.to_string()).collect::<Vec<_>>(),
            });
            output::print_result(&status, json);
            Ok(())
        }
        Err(e) => {
            if json {
                let err = serde_json::json!({
                    "connected": false,
                    "error": e.to_string(),
                });
                output::print_result(&err, true);
            } else {
                eprintln!("✗ {e}");
            }
            // 未连接时返回非零退出码（D12 决策）
            Err(e.to_string())
        }
    }
}

fn run_validate(path: &str, skip_security: bool, json: bool) -> Result<(), String> {
    use open_sunstar_lib::team_config::{validate_team_package_dir, ValidationOptions};

    let dir = std::path::Path::new(path);
    let options = ValidationOptions {
        run_security_scan: !skip_security,
        check_asset_files: true,
    };

    let report = validate_team_package_dir(dir, &options).map_err(|e| e.to_string())?;

    if json {
        let result = serde_json::json!({
            "passed": report.passed,
            "errors": report.errors.iter().map(|e| serde_json::json!({
                "code": e.code.as_str(),
                "message": e.message,
                "location": e.location,
            })).collect::<Vec<_>>(),
            "warnings": report.warnings.iter().map(|w| serde_json::json!({
                "code": w.code.as_str(),
                "message": w.message,
                "location": w.location,
            })).collect::<Vec<_>>(),
            "securityBlocked": report.security.as_ref().map(|s| s.blocked).unwrap_or(false),
            "filesScanned": report.security.as_ref().map(|s| s.files_scanned),
        });
        output::print_result(&result, true);
    } else {
        if report.passed {
            println!("✓ team package validation passed");
        } else {
            println!("✗ team package validation FAILED");
        }
        for e in &report.errors {
            println!("  ERROR [{}] {}", e.code.as_str(), e.message);
        }
        for w in &report.warnings {
            println!("  WARN  [{}] {}", w.code.as_str(), w.message);
        }
        if let Some(sec) = &report.security {
            println!("  security: {} files scanned, blocked={}", sec.files_scanned, sec.blocked);
        }
    }

    if report.passed {
        Ok(())
    } else {
        // 校验失败返回非零退出码（D12 决策：CI 强制执行力）
        Err(format!("{} error(s) found", report.errors.len()))
    }
}

fn run_explain(
    path: &str,
    target: &str,
    project: Option<String>,
    json: bool,
) -> Result<(), String> {
    use open_sunstar_lib::team_config::{
        compile_effective_config, parse_team_package, CompilerInput, TargetApp,
    };

    let dir = std::path::Path::new(path);
    let content = std::fs::read_to_string(dir.join("team.toml"))
        .map_err(|e| format!("cannot read team.toml: {e}"))?;
    let (profiles, policies, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    let input = CompilerInput {
        team_profiles: profiles,
        team_policies: policies,
        project_assets: vec![],
        personal_overrides: vec![],
        target_app: TargetApp::from_str(target),
        project_id: project.unwrap_or_else(|| "default".to_string()),
    };

    let config = compile_effective_config(&input);

    if json {
        output::print_result(&config, true);
    } else {
        println!(
            "Effective config for {} (project: {})",
            config.target_app.as_str(),
            config.project_id
        );
        println!("  digest: {}", config.config_sha256);
        println!("  items: {}", config.items.len());
        for item in &config.items {
            let decision = match item.decision {
                open_sunstar_lib::team_config::EffectiveDecision::Enabled => "✓ enabled",
                open_sunstar_lib::team_config::EffectiveDecision::Denied => "✗ denied",
                open_sunstar_lib::team_config::EffectiveDecision::Skipped => "○ skipped",
                open_sunstar_lib::team_config::EffectiveDecision::Conflicted => "⚠ conflicted",
            };
            println!(
                "  [{}:{}] {}",
                item.asset_type.as_str(),
                item.asset_id,
                decision
            );
            for p in &item.provenance {
                println!("      ← {}", p.explanation);
            }
        }
        if !config.conflicts.is_empty() {
            println!("  conflicts:");
            for c in &config.conflicts {
                println!("    ⚠ {}:{}", c.asset_id, c.message);
            }
        }
        if !config.required_credentials.is_empty() {
            println!("  required credentials:");
            for slot in &config.required_credentials {
                println!("    🔑 {} ({})", slot.slot_id, slot.kind);
            }
        }
    }

    Ok(())
}

fn run_profiles(path: &str, json: bool) -> Result<(), String> {
    use open_sunstar_lib::team_config::parse_team_package;

    let dir = std::path::Path::new(path);
    let content = std::fs::read_to_string(dir.join("team.toml"))
        .map_err(|e| format!("cannot read team.toml: {e}"))?;
    let (profiles, _, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    if json {
        let summaries: Vec<serde_json::Value> = profiles
            .iter()
            .map(|p| {
                serde_json::json!({
                    "profileId": p.profile_id,
                    "name": p.name,
                    "description": p.description,
                    "assetsCount": p.assets.len(),
                    "credentialSlotsCount": p.credential_slots.len(),
                })
            })
            .collect();
        output::print_result(&summaries, true);
    } else {
        println!("Team profiles ({}):", profiles.len());
        for p in &profiles {
            println!(
                "  {} — {} ({} assets, {} credential slots)",
                p.profile_id,
                p.name,
                p.assets.len(),
                p.credential_slots.len()
            );
        }
    }

    Ok(())
}

fn run_diff(path: &str, json: bool) -> Result<(), String> {
    use open_sunstar_lib::team_config::{diff_lock_vs_directory, ReleaseLock};

    let dir = std::path::Path::new(path);
    let lock_path = dir.join("lock.json");
    if !lock_path.exists() {
        return Err("未找到 lock.json 基线文件，请先执行 release 生成".to_string());
    }

    let lock_content =
        std::fs::read_to_string(&lock_path).map_err(|e| format!("读取 lock.json 失败: {e}"))?;
    let lock: ReleaseLock =
        serde_json::from_str(&lock_content).map_err(|e| format!("解析 lock.json 失败: {e}"))?;

    let diff = diff_lock_vs_directory(&lock, dir)?;

    if json {
        output::print_result(&diff, true);
    } else {
        if !diff.summary.has_changes {
            println!("✓ no changes since release {}", diff.base_ref);
            return Ok(());
        }
        println!(
            "Changes since release {} ({} → working directory):",
            diff.base_ref, diff.summary.total_files_base
        );
        for entry in &diff.added {
            println!("  + {} (new, {} bytes)", entry.path, entry.new_size.unwrap_or(0));
        }
        for entry in &diff.removed {
            println!("  - {} (removed)", entry.path);
        }
        for entry in &diff.modified {
            println!(
                "  ~ {} ({} → {} bytes)",
                entry.path,
                entry.old_size.unwrap_or(0),
                entry.new_size.unwrap_or(0)
            );
        }
        println!(
            "  summary: +{} -{} ~{} ={}",
            diff.summary.added_count,
            diff.summary.removed_count,
            diff.summary.modified_count,
            diff.summary.unchanged_count
        );
    }

    Ok(())
}

fn run_assign(
    state: &open_sunstar_lib::AppState,
    project_id: &str,
    workspace_id: &str,
    profile_id: &str,
    json: bool,
) -> Result<(), String> {
    use open_sunstar_lib::team_config::{
        generate_assignment_id, parse_team_package, validate_assignment,
    };

    // 校验工作区存在
    let ws = state
        .db
        .get_team_workspace(workspace_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("workspace '{workspace_id}' not found"))?;

    // 校验 profile 存在
    let available_profiles: Vec<String> = if let Some(snapshot) = &ws.team_toml_snapshot {
        parse_team_package(snapshot)
            .map(|(profiles, _, _)| profiles.iter().map(|p| p.profile_id.clone()).collect())
            .unwrap_or_default()
    } else {
        vec![]
    };
    validate_assignment(profile_id, &available_profiles).map_err(|e| e.to_string())?;

    // 单 Profile 约束
    if let Some(existing) = state
        .db
        .get_active_team_assignment(project_id)
        .map_err(|e| e.to_string())?
    {
        if existing.workspace_id != workspace_id || existing.profile_id != profile_id {
            return Err(format!(
                "project already assigned to profile '{}'",
                existing.profile_id
            ));
        }
        let result = serde_json::json!({ "assignmentId": existing.assignment_id, "changed": false });
        output::print_result(&result, json);
        return Ok(());
    }

    let now = chrono::Utc::now().timestamp();
    let assignment_id = generate_assignment_id(project_id, workspace_id);
    state
        .db
        .upsert_team_assignment(&assignment_id, project_id, workspace_id, profile_id, "active", now, now)
        .map_err(|e| e.to_string())?;

    let result = serde_json::json!({ "assignmentId": assignment_id, "changed": true });
    if json {
        output::print_result(&result, true);
    } else {
        println!("✓ assigned profile '{profile_id}' to project '{project_id}'");
    }
    Ok(())
}

fn run_unassign(
    state: &open_sunstar_lib::AppState,
    project_id: &str,
    json: bool,
) -> Result<(), String> {
    let existing = state
        .db
        .get_active_team_assignment(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no active assignment for this project".to_string())?;

    state
        .db
        .update_team_assignment_status(&existing.assignment_id, "removed")
        .map_err(|e| e.to_string())?;

    if json {
        output::print_result(&serde_json::json!({ "removed": true }), true);
    } else {
        println!("✓ unassigned project '{project_id}'");
    }
    Ok(())
}

fn run_plan(
    path: &str,
    target: &str,
    project_root: &str,
    project: Option<String>,
    json: bool,
) -> Result<(), String> {
    use open_sunstar_lib::team_config::{
        compile_effective_config, generate_deployment_plan, parse_team_package, CompilerInput,
        DeploymentAction, TargetApp,
    };

    let team_dir = std::path::Path::new(path);
    let proj_root = std::path::Path::new(project_root);

    if !proj_root.is_dir() {
        return Err(format!(
            "project root does not exist or is not a directory: {}",
            proj_root.display()
        ));
    }

    let content = std::fs::read_to_string(team_dir.join("team.toml"))
        .map_err(|e| format!("cannot read team.toml: {e}"))?;
    let (profiles, policies, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    let input = CompilerInput {
        team_profiles: profiles,
        team_policies: policies,
        project_assets: vec![],
        personal_overrides: vec![],
        target_app: TargetApp::from_str(target),
        project_id: project.unwrap_or_else(|| "default".to_string()),
    };

    let config = compile_effective_config(&input);
    let plan = generate_deployment_plan(&config, proj_root);

    if json {
        output::print_result(&plan, true);
    } else {
        println!(
            "Deployment plan for {} → {}",
            plan.target_app.as_str(),
            proj_root.display()
        );
        println!("  plan_sha256: {}", plan.plan_sha256);
        println!(
            "  summary: {} create, {} update, {} remove, {} skip, {} display-only",
            plan.summary.create_count,
            plan.summary.update_count,
            plan.summary.remove_count,
            plan.summary.skip_count,
            plan.summary.display_only_count
        );
        println!();
        for step in &plan.steps {
            let icon = match step.action {
                DeploymentAction::Create => "+",
                DeploymentAction::Update => "~",
                DeploymentAction::Remove => "-",
                DeploymentAction::Skip => "=",
                DeploymentAction::DisplayOnly => "○",
            };
            let risk = match step.risk_level {
                open_sunstar_lib::team_config::RiskLevel::Safe => "",
                open_sunstar_lib::team_config::RiskLevel::Low => " [low]",
                open_sunstar_lib::team_config::RiskLevel::Medium => " [medium]",
                open_sunstar_lib::team_config::RiskLevel::High => " [HIGH]",
                open_sunstar_lib::team_config::RiskLevel::RequiresTrust => " [TRUST]",
            };
            println!(
                "  {} [{}:{}]{} → {}",
                icon,
                step.asset_type.as_str(),
                step.asset_id,
                risk,
                step.target_path
            );
        }
        if !plan.warnings.is_empty() {
            println!();
            println!("  warnings:");
            for w in &plan.warnings {
                println!("    ⚠ [{}:{}] {}", w.asset_type.as_str(), w.asset_id, w.message);
            }
        }
    }

    Ok(())
}

fn run_deploy(
    path: &str,
    target: &str,
    project_root: &str,
    project: Option<String>,
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    use open_sunstar_lib::team_config::{
        compile_effective_config, execute_deployment_plan, generate_deployment_plan,
        parse_team_package, CompilerInput, ExecuteOptions, TargetApp,
    };

    let team_dir = std::path::Path::new(path);
    let proj_root = std::path::Path::new(project_root);

    if !proj_root.is_dir() {
        return Err(format!(
            "project root does not exist or is not a directory: {}",
            proj_root.display()
        ));
    }

    let content = std::fs::read_to_string(team_dir.join("team.toml"))
        .map_err(|e| format!("cannot read team.toml: {e}"))?;
    let (profiles, policies, _) = parse_team_package(&content).map_err(|e| e.to_string())?;

    let input = CompilerInput {
        team_profiles: profiles,
        team_policies: policies,
        project_assets: vec![],
        personal_overrides: vec![],
        target_app: TargetApp::from_str(target),
        project_id: project.unwrap_or_else(|| "default".to_string()),
    };

    let config = compile_effective_config(&input);
    let plan = generate_deployment_plan(&config, proj_root);

    if dry_run && !json {
        println!("[dry-run] no files will be modified");
        println!();
    }

    let options = ExecuteOptions {
        team_package_root: team_dir.to_path_buf(),
        dry_run,
        create_backup: true,
    };

    let receipt = execute_deployment_plan(&plan, proj_root, &options);

    if json {
        output::print_result(&receipt, true);
    } else {
        if receipt.summary.all_success {
            println!(
                "✓ deployment complete: {} written, {} skipped",
                receipt.summary.success_count, receipt.summary.skipped_count
            );
        } else {
            println!(
                "⚠ deployment finished with errors: {} ok, {} failed, {} skipped",
                receipt.summary.success_count,
                receipt.summary.failure_count,
                receipt.summary.skipped_count
            );
        }
        println!("  plan_sha256: {}", receipt.plan_sha256);
        for step in &receipt.steps {
            if step.action.is_write() {
                let status = if step.success { "✓" } else { "✗" };
                println!(
                    "  {} [{}:{}] → {}",
                    status,
                    step.asset_type.as_str(),
                    step.asset_id,
                    step.target_path
                );
                if let Some(err) = &step.error {
                    if !err.contains("dry-run") {
                        println!("      error: {err}");
                    }
                }
            }
        }
    }

    if receipt.summary.all_success {
        Ok(())
    } else {
        Err(format!(
            "{} step(s) failed",
            receipt.summary.failure_count
        ))
    }
}

fn run_drift(receipt_path: &str, project_root: &str, json: bool) -> Result<(), String> {
    use open_sunstar_lib::team_config::{detect_drift, DeploymentReceipt};

    let proj_root = std::path::Path::new(project_root);
    if !proj_root.is_dir() {
        return Err(format!(
            "project root does not exist or is not a directory: {}",
            proj_root.display()
        ));
    }

    let receipt_content = std::fs::read_to_string(receipt_path)
        .map_err(|e| format!("cannot read receipt file: {e}"))?;
    let receipt: DeploymentReceipt =
        serde_json::from_str(&receipt_content).map_err(|e| format!("invalid receipt JSON: {e}"))?;

    let report = detect_drift(&receipt, proj_root);

    if json {
        output::print_result(&report, true);
    } else {
        if !report.summary.has_drift {
            println!("✓ no drift detected ({} assets checked)", report.summary.total_checked);
        } else {
            println!(
                "⚠ drift detected: {} drifted, {} clean, {} unknown",
                report.summary.drifted_count,
                report.summary.clean_count,
                report.summary.unknown_count
            );
            for entry in &report.entries {
                if entry.status.is_drifted() {
                    let backup_note = if entry.has_backup { " (rollback available)" } else { "" };
                    println!(
                        "  {} [{}:{}] → {} [{}]{}",
                        entry.status.as_str(),
                        entry.asset_type.as_str(),
                        entry.asset_id,
                        entry.target_path,
                        entry.status.as_str(),
                        backup_note
                    );
                }
            }
        }
    }

    if report.summary.has_drift {
        Err(format!("{} asset(s) drifted", report.summary.drifted_count))
    } else {
        Ok(())
    }
}

fn run_rollback(
    receipt_path: &str,
    drift_path: Option<String>,
    project_root: &str,
    json: bool,
) -> Result<(), String> {
    use open_sunstar_lib::team_config::{
        detect_drift, execute_rollback, DeploymentReceipt, DriftReport,
    };

    let proj_root = std::path::Path::new(project_root);
    if !proj_root.is_dir() {
        return Err(format!(
            "project root does not exist or is not a directory: {}",
            proj_root.display()
        ));
    }

    let receipt_content = std::fs::read_to_string(receipt_path)
        .map_err(|e| format!("cannot read receipt file: {e}"))?;
    let receipt: DeploymentReceipt =
        serde_json::from_str(&receipt_content).map_err(|e| format!("invalid receipt JSON: {e}"))?;

    // 加载或自动检测偏差
    let drift: DriftReport = if let Some(dp) = drift_path {
        let dc = std::fs::read_to_string(&dp)
            .map_err(|e| format!("cannot read drift file: {e}"))?;
        serde_json::from_str(&dc).map_err(|e| format!("invalid drift JSON: {e}"))?
    } else {
        detect_drift(&receipt, proj_root)
    };

    if !drift.summary.has_drift {
        if json {
            output::print_result(&serde_json::json!({ "message": "no drift, nothing to rollback" }), true);
        } else {
            println!("✓ no drift detected, nothing to rollback");
        }
        return Ok(());
    }

    let report = execute_rollback(&receipt, &drift, proj_root);

    if json {
        output::print_result(&report, true);
    } else {
        if report.summary.all_success {
            println!(
                "✓ rollback complete: {} restored",
                report.summary.success_count
            );
        } else {
            println!(
                "⚠ rollback finished with errors: {} ok, {} failed",
                report.summary.success_count, report.summary.failure_count
            );
        }
        for step in &report.steps {
            let status = if step.success { "✓" } else { "✗" };
            println!(
                "  {} [{}:{}] → {}",
                status,
                step.asset_type.as_str(),
                step.asset_id,
                step.target_path
            );
            if let Some(err) = &step.error {
                println!("      error: {err}");
            }
        }
    }

    if report.summary.all_success {
        Ok(())
    } else {
        Err(format!(
            "{} rollback step(s) failed",
            report.summary.failure_count
        ))
    }
}
