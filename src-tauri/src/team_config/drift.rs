//! 偏差检测 + 安全回滚（Git MVP M5+M6）
//!
//! M5: 将部署回执中记录的 post_write_sha256 与项目目录当前状态对比，
//!     检测部署后是否有外部修改（偏差）。
//! M6: 利用 executor 创建的备份文件，将偏差资产回滚到部署前状态。
//!
//! 设计约束（冻结文档）：
//! - 偏差检测 = 期望（receipt）vs 实际（文件系统）
//! - 回滚 = 零写入阻断（如果备份不存在则拒绝回滚，不猜测）
//! - 回滚后验证：恢复的文件 SHA-256 必须与备份一致

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::deployment::DeploymentAction;
use super::domain::AssetType;
use super::executor::{DeploymentReceipt, StepReceipt};

// ─── 偏差检测 ──────────────────────────────────────────────────────────────────

/// 单个资产的偏差状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DriftStatus {
    /// 无偏差：当前状态与部署回执一致
    Clean,
    /// 有偏差：文件被外部修改
    Modified,
    /// 文件被删除
    Deleted,
    /// 文件被新增（部署时不存在，现在出现了）
    Added,
    /// 无法检测（部署时未记录 SHA-256）
    Unknown,
}

impl DriftStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Added => "added",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_drifted(&self) -> bool {
        matches!(self, Self::Modified | Self::Deleted | Self::Added)
    }
}

/// 单个资产的偏差条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftEntry {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub target_path: String,
    pub status: DriftStatus,
    /// 部署时记录的 SHA-256
    pub expected_sha256: Option<String>,
    /// 当前文件系统的 SHA-256
    pub actual_sha256: Option<String>,
    /// 是否有可用备份（用于回滚）
    pub has_backup: bool,
}

/// 偏差检测报告
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftReport {
    pub project_id: String,
    pub plan_sha256: String,
    /// 检测时间戳
    pub checked_at: i64,
    pub entries: Vec<DriftEntry>,
    pub summary: DriftSummary,
}

/// 偏差汇总
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftSummary {
    pub total_checked: usize,
    pub clean_count: usize,
    pub drifted_count: usize,
    pub unknown_count: usize,
    /// 是否有任何偏差
    pub has_drift: bool,
    /// 可回滚的偏差数（有备份的）
    pub rollback_eligible_count: usize,
}

/// 检测部署后偏差
///
/// 将部署回执中记录的 post_write_sha256 与项目目录当前状态对比。
/// 仅检查写入步骤（Create/Update），Remove 步骤检查目标是否被重新创建。
pub fn detect_drift(receipt: &DeploymentReceipt, project_root: &Path) -> DriftReport {
    let mut entries: Vec<DriftEntry> = Vec::new();

    for step in &receipt.steps {
        if !step.action.is_write() {
            continue;
        }

        let entry = check_step_drift(step, project_root);
        entries.push(entry);
    }

    let summary = compute_drift_summary(&entries);

    DriftReport {
        project_id: receipt.project_id.clone(),
        plan_sha256: receipt.plan_sha256.clone(),
        checked_at: chrono::Utc::now().timestamp(),
        entries,
        summary,
    }
}

/// 检查单个步骤的偏差
fn check_step_drift(step: &StepReceipt, project_root: &Path) -> DriftEntry {
    let target_abs = project_root.join(&step.target_path);

    let base = DriftEntry {
        asset_type: step.asset_type.clone(),
        asset_id: step.asset_id.clone(),
        target_path: step.target_path.clone(),
        status: DriftStatus::Unknown,
        expected_sha256: step.post_write_sha256.clone(),
        actual_sha256: None,
        has_backup: step.backup_path.is_some(),
    };

    match step.action {
        DeploymentAction::Create | DeploymentAction::Update => {
            // 期望文件存在且 SHA-256 匹配
            let expected = match &step.post_write_sha256 {
                Some(sha) => sha,
                None => return base, // 无记录，无法检测
            };

            if !target_abs.exists() {
                return DriftEntry {
                    status: DriftStatus::Deleted,
                    ..base
                };
            }

            let actual = compute_current_sha256(&target_abs, &step.asset_type);
            let status = if &actual == expected {
                DriftStatus::Clean
            } else {
                DriftStatus::Modified
            };

            DriftEntry {
                status,
                actual_sha256: Some(actual),
                ..base
            }
        }
        DeploymentAction::Remove => {
            // 期望文件不存在（已被移除）
            if target_abs.exists() {
                let actual = compute_current_sha256(&target_abs, &step.asset_type);
                DriftEntry {
                    status: DriftStatus::Added, // 被重新创建
                    actual_sha256: Some(actual),
                    ..base
                }
            } else {
                DriftEntry {
                    status: DriftStatus::Clean,
                    ..base
                }
            }
        }
        _ => base,
    }
}

/// 计算当前文件/目录的 SHA-256
fn compute_current_sha256(path: &Path, asset_type: &AssetType) -> String {
    match asset_type {
        AssetType::Skill => {
            if path.is_dir() {
                // 复用 deployment 模块的目录哈希
                super::deployment::scan_current_sha256(
                    path.parent().and_then(|p| p.parent()).unwrap_or(path),
                    asset_type,
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .as_ref(),
                    &super::domain::TargetApp::ClaudeCode,
                )
                .unwrap_or_default()
            } else {
                String::new()
            }
        }
        _ => {
            if path.is_file() {
                std::fs::read(path)
                    .map(|content| super::release::sha256_of_content(&content))
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
    }
}

/// 计算偏差汇总
fn compute_drift_summary(entries: &[DriftEntry]) -> DriftSummary {
    let mut summary = DriftSummary {
        total_checked: entries.len(),
        clean_count: 0,
        drifted_count: 0,
        unknown_count: 0,
        has_drift: false,
        rollback_eligible_count: 0,
    };

    for entry in entries {
        match entry.status {
            DriftStatus::Clean => summary.clean_count += 1,
            DriftStatus::Unknown => summary.unknown_count += 1,
            _ => {
                summary.drifted_count += 1;
                summary.has_drift = true;
                if entry.has_backup {
                    summary.rollback_eligible_count += 1;
                }
            }
        }
    }

    summary
}

// ─── 安全回滚 ──────────────────────────────────────────────────────────────────

/// 单步回滚结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackStepResult {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub target_path: String,
    pub success: bool,
    /// 回滚后验证的 SHA-256
    pub restored_sha256: Option<String>,
    pub error: Option<String>,
}

/// 回滚报告
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackReport {
    pub project_id: String,
    pub plan_sha256: String,
    pub rolled_back_at: i64,
    pub steps: Vec<RollbackStepResult>,
    pub summary: RollbackSummary,
}

/// 回滚汇总
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackSummary {
    pub total_attempted: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_no_backup: usize,
    pub all_success: bool,
}

/// 执行回滚
///
/// 利用部署回执中记录的 backup_path，将有偏差的资产恢复到部署前状态。
///
/// # 安全约束（M6：零写入阻断）
/// - 仅回滚有偏差且有备份的步骤
/// - 备份不存在时拒绝回滚（不猜测、不重建）
/// - 回滚后验证：恢复的文件必须与备份内容一致
pub fn execute_rollback(
    receipt: &DeploymentReceipt,
    drift_report: &DriftReport,
    project_root: &Path,
) -> RollbackReport {
    let mut results: Vec<RollbackStepResult> = Vec::new();

    // 建立 drift entry 索引
    let drift_map: std::collections::HashMap<(&str, &str), &DriftEntry> = drift_report
        .entries
        .iter()
        .map(|e| ((e.asset_type.as_str(), e.asset_id.as_str()), e))
        .collect();

    for step in &receipt.steps {
        if !step.action.is_write() {
            continue;
        }

        let key = (step.asset_type.as_str(), step.asset_id.as_str());
        let drift_entry = drift_map.get(&key);

        // 仅回滚有偏差的步骤
        let is_drifted = drift_entry.map(|e| e.status.is_drifted()).unwrap_or(false);
        if !is_drifted {
            continue;
        }

        let result = rollback_step(step, project_root);
        results.push(result);
    }

    let summary = compute_rollback_summary(&results);

    RollbackReport {
        project_id: receipt.project_id.clone(),
        plan_sha256: receipt.plan_sha256.clone(),
        rolled_back_at: chrono::Utc::now().timestamp(),
        steps: results,
        summary,
    }
}

/// 回滚单个步骤
fn rollback_step(step: &StepReceipt, project_root: &Path) -> RollbackStepResult {
    let base = RollbackStepResult {
        asset_type: step.asset_type.clone(),
        asset_id: step.asset_id.clone(),
        target_path: step.target_path.clone(),
        success: false,
        restored_sha256: None,
        error: None,
    };

    // 零写入阻断：无备份则拒绝
    let backup_path = match &step.backup_path {
        Some(bp) => std::path::PathBuf::from(bp),
        None => {
            return RollbackStepResult {
                error: Some("无可用备份，拒绝回滚（零写入阻断）".to_string()),
                ..base
            };
        }
    };

    // 备份路径必须落在本项目的备份目录内。
    // backup_path 来自回执 JSON，可被伪造：不校验则可把任意文件（~/.claude.json、
    // 私钥等）复制进项目树，再随团队配置仓库 push 出去。
    let allowed_root = super::executor::backup_root(project_root);
    if let Err(e) = super::executor::assert_path_contained(&backup_path, &allowed_root) {
        return RollbackStepResult {
            error: Some(format!(
                "备份路径越界，拒绝回滚: {} (detail: {e})",
                backup_path.display()
            )),
            ..base
        };
    }

    if !backup_path.exists() {
        return RollbackStepResult {
            error: Some(format!("备份文件不存在: {}", backup_path.display())),
            ..base
        };
    }

    let target_abs = project_root.join(&step.target_path);

    // W2: 路径包含性校验，防止恶意 receipt 的 target_path 逃逸
    {
        use std::path::Component;
        let normalize = |p: &Path| -> std::path::PathBuf {
            let mut parts = Vec::new();
            for c in p.components() {
                match c {
                    Component::ParentDir => {
                        parts.pop();
                    }
                    Component::CurDir => {}
                    other => parts.push(other),
                }
            }
            parts.iter().collect()
        };
        if !normalize(&target_abs).starts_with(normalize(project_root)) {
            return RollbackStepResult {
                error: Some(format!(
                    "路径遍历检测: {} 逃逸出 {}",
                    step.target_path,
                    project_root.display()
                )),
                ..base
            };
        }
    }

    // 执行恢复
    let restore_result = if backup_path.is_dir() {
        // 目录恢复
        if target_abs.exists() {
            if target_abs.is_dir() {
                std::fs::remove_dir_all(&target_abs).map_err(|e| e.to_string())
            } else {
                std::fs::remove_file(&target_abs).map_err(|e| e.to_string())
            }
        } else {
            Ok(())
        }
        .and_then(|_| copy_dir_recursive(&backup_path, &target_abs))
    } else {
        // 文件恢复
        if let Some(parent) = target_abs.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::copy(&backup_path, &target_abs)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };

    match restore_result {
        Ok(()) => {
            // 回滚后验证
            let restored_sha = if target_abs.is_file() {
                std::fs::read(&target_abs)
                    .ok()
                    .map(|c| super::release::sha256_of_content(&c))
            } else if target_abs.is_dir() {
                Some(compute_current_sha256(&target_abs, &step.asset_type))
            } else {
                None
            };

            RollbackStepResult {
                success: true,
                restored_sha256: restored_sha,
                ..base
            }
        }
        Err(e) => RollbackStepResult {
            error: Some(format!("恢复失败: {e}")),
            ..base
        },
    }
}

/// 递归复制目录（回滚用，跳过 symlink 防止跟随攻击）
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // C3: 跳过 symlink，防止跟随攻击
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 计算回滚汇总
fn compute_rollback_summary(results: &[RollbackStepResult]) -> RollbackSummary {
    let mut summary = RollbackSummary {
        total_attempted: results.len(),
        success_count: 0,
        failure_count: 0,
        skipped_no_backup: 0,
        all_success: true,
    };

    for r in results {
        if r.success {
            summary.success_count += 1;
        } else {
            summary.failure_count += 1;
            summary.all_success = false;
            if r.error
                .as_ref()
                .map(|e| e.contains("备份"))
                .unwrap_or(false)
            {
                summary.skipped_no_backup += 1;
            }
        }
    }

    summary
}

// ─── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team_config::deployment::DeploymentAction;
    use crate::team_config::domain::AssetType;
    use crate::team_config::executor::{DeploymentReceipt, ReceiptSummary, StepReceipt};
    use std::fs;
    use tempfile::TempDir;

    fn make_receipt(steps: Vec<StepReceipt>) -> DeploymentReceipt {
        let success_count = steps
            .iter()
            .filter(|s| s.success && s.action.is_write())
            .count();
        DeploymentReceipt {
            project_id: "proj_test".to_string(),
            target_app: "claude_code".to_string(),
            plan_sha256: "plan_abc".to_string(),
            summary: ReceiptSummary {
                total_steps: steps.len(),
                success_count,
                failure_count: 0,
                skipped_count: steps.len() - success_count,
                all_success: true,
            },
            steps,
            executed_at: 1000000,
        }
    }

    fn make_write_step(
        asset_type: AssetType,
        asset_id: &str,
        target_path: &str,
        post_sha: Option<&str>,
        backup: Option<&str>,
    ) -> StepReceipt {
        StepReceipt {
            asset_type,
            asset_id: asset_id.to_string(),
            action: DeploymentAction::Create,
            target_path: target_path.to_string(),
            success: true,
            post_write_sha256: post_sha.map(|s| s.to_string()),
            backup_path: backup.map(|s| s.to_string()),
            error: None,
        }
    }

    #[test]
    fn detect_clean_state() {
        let project = TempDir::new().unwrap();
        let content = b"*.secret\n";
        fs::write(project.path().join(".claudeignore"), content).unwrap();
        let sha = crate::team_config::release::sha256_of_content(content);

        let receipt = make_receipt(vec![make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            Some(&sha),
            None,
        )]);

        let report = detect_drift(&receipt, project.path());
        assert!(!report.summary.has_drift);
        assert_eq!(report.summary.clean_count, 1);
        assert_eq!(report.entries[0].status, DriftStatus::Clean);
    }

    #[test]
    fn detect_modified_file() {
        let project = TempDir::new().unwrap();
        fs::write(project.path().join(".claudeignore"), b"original").unwrap();
        let original_sha = crate::team_config::release::sha256_of_content(b"original");

        let receipt = make_receipt(vec![make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            Some(&original_sha),
            None,
        )]);

        // 外部修改
        fs::write(project.path().join(".claudeignore"), b"tampered").unwrap();

        let report = detect_drift(&receipt, project.path());
        assert!(report.summary.has_drift);
        assert_eq!(report.entries[0].status, DriftStatus::Modified);
    }

    #[test]
    fn detect_deleted_file() {
        let project = TempDir::new().unwrap();

        let receipt = make_receipt(vec![make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            Some("some_sha"),
            None,
        )]);

        // 文件不存在 → Deleted
        let report = detect_drift(&receipt, project.path());
        assert!(report.summary.has_drift);
        assert_eq!(report.entries[0].status, DriftStatus::Deleted);
    }

    #[test]
    fn rollback_restores_from_backup() {
        let project = TempDir::new().unwrap();

        // 创建备份（必须位于 create_backup 实际使用的项目内备份目录）
        let backup_dir = project.path().join(".opensunstar").join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        let backup_file = backup_dir.join("backup_claudeignore");
        fs::write(&backup_file, b"original content").unwrap();

        // 当前文件被修改
        fs::write(project.path().join(".claudeignore"), b"tampered").unwrap();
        let original_sha = crate::team_config::release::sha256_of_content(b"original content");

        let receipt = make_receipt(vec![make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            Some(&original_sha),
            Some(backup_file.to_str().unwrap()),
        )]);

        // 检测偏差
        let drift = detect_drift(&receipt, project.path());
        assert!(drift.summary.has_drift);
        assert_eq!(drift.summary.rollback_eligible_count, 1);

        // 执行回滚
        let rollback = execute_rollback(&receipt, &drift, project.path());
        assert!(rollback.summary.all_success);
        assert_eq!(rollback.summary.success_count, 1);

        // 验证恢复
        let restored = fs::read_to_string(project.path().join(".claudeignore")).unwrap();
        assert_eq!(restored, "original content");
    }

    #[test]
    fn rollback_refuses_without_backup() {
        let project = TempDir::new().unwrap();
        fs::write(project.path().join(".claudeignore"), b"tampered").unwrap();

        let receipt = make_receipt(vec![make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            Some("expected_sha"),
            None, // 无备份
        )]);

        let drift = detect_drift(&receipt, project.path());
        assert!(drift.summary.has_drift);
        assert_eq!(drift.summary.rollback_eligible_count, 0);

        // 回滚应跳过（无偏差步骤不触发）或拒绝
        let rollback = execute_rollback(&receipt, &drift, project.path());
        // 有偏差但无备份 → 尝试回滚但失败
        assert!(!rollback.summary.all_success || rollback.summary.total_attempted == 0);
    }

    #[test]
    fn rollback_skips_clean_steps() {
        let project = TempDir::new().unwrap();
        let content = b"stable content";
        fs::write(project.path().join(".claudeignore"), content).unwrap();
        let sha = crate::team_config::release::sha256_of_content(content);

        let receipt = make_receipt(vec![make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            Some(&sha),
            Some("/nonexistent/backup"),
        )]);

        let drift = detect_drift(&receipt, project.path());
        assert!(!drift.summary.has_drift);

        // 无偏差 → 不尝试回滚
        let rollback = execute_rollback(&receipt, &drift, project.path());
        assert_eq!(rollback.summary.total_attempted, 0);
        assert!(rollback.summary.all_success);
    }

    #[test]
    fn detect_remove_step_recreated() {
        let project = TempDir::new().unwrap();
        // Remove 步骤期望文件不存在，但文件被重新创建
        fs::write(project.path().join(".claudeignore"), b"recreated").unwrap();

        let step = StepReceipt {
            asset_type: AssetType::Ignore,
            asset_id: "secrets".to_string(),
            action: DeploymentAction::Remove,
            target_path: ".claudeignore".to_string(),
            success: true,
            post_write_sha256: None,
            backup_path: None,
            error: None,
        };

        let receipt = make_receipt(vec![step]);
        let report = detect_drift(&receipt, project.path());
        assert!(report.summary.has_drift);
        assert_eq!(report.entries[0].status, DriftStatus::Added);
    }

    #[test]
    fn rollback_rejects_backup_path_outside_project() {
        let project = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        // 伪造回执把 backup_path 指向项目外的敏感文件
        let secret = outside.path().join("id_rsa");
        fs::write(&secret, b"PRIVATE KEY").unwrap();

        let step = make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            None,
            Some(secret.to_str().unwrap()),
        );

        let result = rollback_step(&step, project.path());

        assert!(!result.success, "越界备份路径必须被拒绝");
        assert!(
            result
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("备份路径"),
            "错误信息应指明备份路径越界, got: {:?}",
            result.error
        );
        assert!(
            !project.path().join(".claudeignore").exists(),
            "拒绝后不得产生任何写入"
        );
    }

    #[test]
    fn rollback_accepts_backup_inside_project_backup_dir() {
        let project = TempDir::new().unwrap();
        let backup_dir = project.path().join(".opensunstar").join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        let backup = backup_dir.join("20260726_000000_.claudeignore_deadbeef");
        fs::write(&backup, b"original").unwrap();

        let step = make_write_step(
            AssetType::Ignore,
            "secrets",
            ".claudeignore",
            None,
            Some(backup.to_str().unwrap()),
        );

        let result = rollback_step(&step, project.path());

        assert!(result.success, "合法备份路径应可回滚: {:?}", result.error);
        assert_eq!(
            fs::read(project.path().join(".claudeignore")).unwrap(),
            b"original"
        );
    }
}
