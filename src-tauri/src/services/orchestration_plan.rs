//! Engineering-grade orchestration execution helpers.
//!
//! This module keeps project orchestration writes observable and reversible:
//! plan -> snapshot -> apply -> verify -> receipt. It is intentionally small so
//! higher-level services can adopt it incrementally without a large refactor.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::write_text_file;
use crate::error::AppError;

const SNAPSHOT_DIR_REL: &str = ".opensunstar/backups/orchestration";
pub const RECEIPT_REL: &str = ".opensunstar/orchestration-receipt.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OrchestrationStepStatus {
    Planned,
    Applied,
    Skipped,
    Verified,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationStepReceipt {
    pub id: String,
    pub label: String,
    pub target_path: String,
    pub action: String,
    pub status: OrchestrationStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationVerification {
    pub id: String,
    pub label: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationReceipt {
    pub schema: String,
    pub operation: String,
    pub project_path: String,
    pub dry_run: bool,
    pub created_at: String,
    pub steps: Vec<OrchestrationStepReceipt>,
    pub verifications: Vec<OrchestrationVerification>,
}

#[derive(Debug, Clone)]
pub struct PlannedTextWrite {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub content: String,
    pub overwrite: bool,
    pub conflict_policy: String,
}

impl PlannedTextWrite {
    pub fn replace(id: &str, label: &str, path: PathBuf, content: String) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            path,
            content,
            overwrite: true,
            conflict_policy: "replace-managed-target".to_string(),
        }
    }

    pub fn create_if_missing(id: &str, label: &str, path: PathBuf, content: String) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            path,
            content,
            overwrite: false,
            conflict_policy: "create-if-missing".to_string(),
        }
    }
}

pub fn execute_text_write_plan(
    project_path: &str,
    operation: &str,
    steps: Vec<PlannedTextWrite>,
    dry_run: bool,
    verifications: Vec<OrchestrationVerification>,
) -> Result<OrchestrationReceipt, AppError> {
    execute_text_write_plan_with_receipt(project_path, operation, steps, dry_run, verifications, true)
}

pub fn execute_text_write_plan_without_receipt(
    project_path: &str,
    operation: &str,
    steps: Vec<PlannedTextWrite>,
    dry_run: bool,
    verifications: Vec<OrchestrationVerification>,
) -> Result<OrchestrationReceipt, AppError> {
    execute_text_write_plan_with_receipt(project_path, operation, steps, dry_run, verifications, false)
}

pub fn restore_latest_orchestration_receipt(project_path: &str) -> Result<OrchestrationReceipt, AppError> {
    let root = PathBuf::from(project_path);
    let receipt_path = root.join(RECEIPT_REL);
    let receipt_text = fs::read_to_string(&receipt_path).map_err(|e| AppError::io(&receipt_path, e))?;
    let receipt: OrchestrationReceipt = serde_json::from_str(&receipt_text)
        .map_err(|e| AppError::Message(format!("解析编排 receipt 失败: {e}")))?;

    let mut rollback_steps = Vec::new();
    for step in receipt.steps.iter().rev() {
        if step.status != OrchestrationStepStatus::Applied {
            continue;
        }
        let target = root.join(&step.target_path);
        if let Some(snapshot_rel) = step.snapshot_path.as_deref() {
            let snapshot_path = root.join(snapshot_rel);
            let snapshot = fs::read_to_string(&snapshot_path).map_err(|e| AppError::io(&snapshot_path, e))?;
            write_text_file(&target, &snapshot)?;
            rollback_steps.push(OrchestrationStepReceipt {
                id: format!("rollback-{}", step.id),
                label: format!("恢复 {}", step.label),
                target_path: step.target_path.clone(),
                action: "restore-snapshot".to_string(),
                status: OrchestrationStepStatus::Applied,
                before_checksum: step.after_checksum.clone(),
                after_checksum: Some(checksum_text(&snapshot)),
                snapshot_path: step.snapshot_path.clone(),
                reason: None,
            });
        } else if step.before_checksum.is_none() && target.exists() {
            fs::remove_file(&target).map_err(|e| AppError::io(&target, e))?;
            rollback_steps.push(OrchestrationStepReceipt {
                id: format!("rollback-{}", step.id),
                label: format!("移除 {}", step.label),
                target_path: step.target_path.clone(),
                action: "remove-created-file".to_string(),
                status: OrchestrationStepStatus::Applied,
                before_checksum: step.after_checksum.clone(),
                after_checksum: None,
                snapshot_path: None,
                reason: Some("原始文件不存在，回滚时删除本次创建的文件".to_string()),
            });
        }
    }

    let rollback_receipt = OrchestrationReceipt {
        schema: "opensunstar.orchestration-receipt/v1".to_string(),
        operation: format!("rollback:{}", receipt.operation),
        project_path: project_path.to_string(),
        dry_run: false,
        created_at: Utc::now().to_rfc3339(),
        steps: rollback_steps,
        verifications: vec![verification(
            "rollback-complete",
            "已按最近一次编排 receipt 恢复",
            true,
            Some(RECEIPT_REL.to_string()),
        )],
    };
    let rollback_json = serde_json::to_string_pretty(&rollback_receipt)
        .map_err(|e| AppError::Message(format!("序列化回滚 receipt 失败: {e}")))?;
    write_text_file(&receipt_path, &(rollback_json + "\n"))?;
    Ok(rollback_receipt)
}

fn execute_text_write_plan_with_receipt(
    project_path: &str,
    operation: &str,
    steps: Vec<PlannedTextWrite>,
    dry_run: bool,
    verifications: Vec<OrchestrationVerification>,
    write_receipt: bool,
) -> Result<OrchestrationReceipt, AppError> {
    let root = PathBuf::from(project_path);
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string();
    let snapshot_root = root.join(SNAPSHOT_DIR_REL).join(safe_segment(operation)).join(&timestamp);
    let mut receipts = Vec::with_capacity(steps.len());

    for step in steps {
        let before = read_optional(&step.path)?;
        let before_checksum = before.as_deref().map(checksum_text);
        let after_checksum = Some(checksum_text(&step.content));
        let target_rel = display_path(&root, &step.path);

        if before.is_some() && !step.overwrite {
            receipts.push(OrchestrationStepReceipt {
                id: step.id,
                label: step.label,
                target_path: target_rel,
                action: step.conflict_policy,
                status: OrchestrationStepStatus::Skipped,
                before_checksum,
                after_checksum: None,
                snapshot_path: None,
                reason: Some("目标文件已存在，按冲突策略不覆盖".to_string()),
            });
            continue;
        }

        if before.as_deref() == Some(step.content.as_str()) {
            receipts.push(OrchestrationStepReceipt {
                id: step.id,
                label: step.label,
                target_path: target_rel,
                action: "no-op".to_string(),
                status: OrchestrationStepStatus::Skipped,
                before_checksum,
                after_checksum,
                snapshot_path: None,
                reason: Some("内容未变化".to_string()),
            });
            continue;
        }

        let snapshot_path = if let Some(existing) = before.as_deref() {
            let snapshot_path = snapshot_root.join(target_rel.replace('/', "__").replace('\\', "__"));
            if !dry_run {
                write_text_file(&snapshot_path, existing)?;
            }
            Some(display_path(&root, &snapshot_path))
        } else {
            None
        };

        if !dry_run {
            write_text_file(&step.path, &step.content)?;
        }

        receipts.push(OrchestrationStepReceipt {
            id: step.id,
            label: step.label,
            target_path: target_rel,
            action: step.conflict_policy,
            status: if dry_run {
                OrchestrationStepStatus::Planned
            } else {
                OrchestrationStepStatus::Applied
            },
            before_checksum,
            after_checksum,
            snapshot_path,
            reason: None,
        });
    }

    let mut receipt = OrchestrationReceipt {
        schema: "opensunstar.orchestration-receipt/v1".to_string(),
        operation: operation.to_string(),
        project_path: project_path.to_string(),
        dry_run,
        created_at: Utc::now().to_rfc3339(),
        steps: receipts,
        verifications,
    };

    if !dry_run {
        for step in &receipt.steps {
            let passed = match step.status {
                OrchestrationStepStatus::Applied => verify_step_after_apply(&root, step)?,
                OrchestrationStepStatus::Skipped | OrchestrationStepStatus::Planned => true,
                OrchestrationStepStatus::Verified | OrchestrationStepStatus::Failed => true,
            };
            receipt.verifications.push(verification(
                &format!("step-{}", step.id),
                &format!("{} 写后校验", step.label),
                passed,
                Some(step.target_path.clone()),
            ));
        }
    }

    if !dry_run && write_receipt {
        let receipt_json = serde_json::to_string_pretty(&receipt)
            .map_err(|e| AppError::Message(format!("序列化编排 receipt 失败: {e}")))?;
        write_text_file(&root.join(RECEIPT_REL), &(receipt_json + "\n"))?;
    }

    Ok(receipt)
}

fn verify_step_after_apply(root: &Path, step: &OrchestrationStepReceipt) -> Result<bool, AppError> {
    let target = root.join(&step.target_path);
    if let Some(expected) = step.after_checksum.as_deref() {
        let text = fs::read_to_string(&target).map_err(|e| AppError::io(&target, e))?;
        Ok(checksum_text(&text) == expected)
    } else {
        Ok(!target.exists())
    }
}

pub fn verification(id: &str, label: &str, passed: bool, detail: impl Into<Option<String>>) -> OrchestrationVerification {
    OrchestrationVerification {
        id: id.to_string(),
        label: label.to_string(),
        passed,
        detail: detail.into(),
    }
}

pub fn checksum_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn read_optional(path: &Path) -> Result<Option<String>, AppError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::io(path, e)),
    }
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn safe_segment(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}
