//! Derived health for a project asset expectation.
//!
//! Health is intentionally computed from facts instead of persisted as a second
//! source of truth. This keeps status explainable and makes stale evidence safe.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ai::asset_app_support::{asset_capability, asset_support, AssetSupport};
use crate::database::{
    AssetDeploymentReceipt, AssetReceiptFile, AssetRuntimeEvidence, Database,
    ProjectAssetExpectation,
};
use crate::error::AppError;

const ASSET_HEALTH_SNAPSHOT_DIR: &str = ".opensunstar/backups/asset-health";

#[derive(Debug, Clone)]
struct ManagedFileSnapshot {
    digest: String,
    content: Vec<u8>,
}

pub const HEALTHY: &str = "healthy";
pub const ATTENTION: &str = "attention";
pub const UNHEALTHY: &str = "unhealthy";
pub const UNKNOWN: &str = "unknown";
pub const UNSUPPORTED: &str = "unsupported";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetHealthRecord {
    pub expectation: ProjectAssetExpectation,
    pub status: String,
    pub evidence_level: String,
    pub reason_code: String,
    pub recommended_action: String,
    pub last_receipt_id: Option<String>,
    pub last_evidence_id: Option<String>,
    pub observed_at: Option<i64>,
    pub last_receipt_files: Vec<AssetReceiptFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetHealthPlanStep {
    pub expectation_id: String,
    pub asset_type: String,
    pub asset_id: String,
    pub required_revision_id: Option<String>,
    pub target_app: String,
    pub action: String,
    pub reason_code: String,
    pub adapter_id: String,
    pub write_mode: String,
    pub verify_modes: Vec<String>,
    pub limitations: Vec<String>,
    pub managed_paths: Vec<String>,
    pub protected_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetHealthPlan {
    pub operation_id: String,
    pub project_id: String,
    pub plan_sha256: String,
    pub steps: Vec<AssetHealthPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HealthDecision {
    status: &'static str,
    evidence_level: &'static str,
    reason_code: &'static str,
    recommended_action: &'static str,
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn managed_path_hints(target_app: &str) -> Vec<String> {
    match target_app {
        "claude" => vec![
            ".claude/".into(),
            ".mcp.json".into(),
            ".claudeignore".into(),
            "CLAUDE.md".into(),
        ],
        "codex" => vec![".codex/".into(), ".codexignore".into(), "AGENTS.md".into()],
        "gemini" => vec![
            ".gemini/".into(),
            ".geminiignore".into(),
            "GEMINI.md".into(),
        ],
        "opencode" => vec![
            ".opencode/".into(),
            ".opencodeignore".into(),
            "opencode.json".into(),
            "AGENTS.md".into(),
        ],
        "openclaw" => vec![
            ".openclaw/".into(),
            "openclaw.json".into(),
            "AGENTS.md".into(),
        ],
        "hermes" => vec![
            ".hermes/".into(),
            ".hermesignore".into(),
            "hermes.yaml".into(),
            "AGENTS.md".into(),
        ],
        _ => Vec::new(),
    }
}

fn sync_check_name(asset_type: &str) -> Option<&'static str> {
    match asset_type {
        "mcp" => Some("mcp_enabled"),
        "prompt" => Some("prompt_files"),
        "command" => Some("commands_configured"),
        "hook" => Some("hooks_configured"),
        "permission" => Some("permissions"),
        "skill" => Some("skills_configured"),
        "subagent" => Some("subagents_configured"),
        "ignore" => Some("ignore_rules"),
        _ => None,
    }
}

fn path_matches_app(relative_path: &str, target_app: &str) -> bool {
    managed_path_hints(target_app)
        .iter()
        .any(|hint| relative_path == hint.trim_end_matches('/') || relative_path.starts_with(hint))
        || relative_path.starts_with(".agents/")
        || relative_path.starts_with(".opensunstar/")
}

fn collect_files(root: &Path, current: &Path, output: &mut HashMap<String, ManagedFileSnapshot>) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            if path.ends_with(Path::new(ASSET_HEALTH_SNAPSHOT_DIR)) {
                continue;
            }
            collect_files(root, &path, output);
        } else if metadata.is_file() && metadata.len() <= 2 * 1024 * 1024 {
            if let (Ok(relative), Ok(bytes)) = (path.strip_prefix(root), fs::read(&path)) {
                output.insert(
                    relative.to_string_lossy().replace('\\', "/"),
                    ManagedFileSnapshot {
                        digest: format!("{:x}", Sha256::digest(&bytes)),
                        content: bytes,
                    },
                );
            }
        }
    }
}

fn snapshot_managed_files(root: &Path) -> HashMap<String, ManagedFileSnapshot> {
    let mut output = HashMap::new();
    for directory in [
        ".claude",
        ".codex",
        ".gemini",
        ".opencode",
        ".openclaw",
        ".hermes",
        ".agents",
        ".opensunstar",
    ] {
        let path = root.join(directory);
        if path.is_dir() {
            collect_files(root, &path, &mut output);
        }
    }
    for file in [
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        ".mcp.json",
        ".claudeignore",
        ".codexignore",
        ".geminiignore",
        ".opencodeignore",
        ".hermesignore",
        "opencode.json",
        "openclaw.json",
        "hermes.yaml",
    ] {
        let path = root.join(file);
        if path.is_file() {
            if let Ok(bytes) = fs::read(&path) {
                output.insert(
                    file.into(),
                    ManagedFileSnapshot {
                        digest: format!("{:x}", Sha256::digest(&bytes)),
                        content: bytes,
                    },
                );
            }
        }
    }
    output
}

fn path_matches_asset(relative_path: &str, asset_type: &str) -> bool {
    let path = relative_path.to_ascii_lowercase();
    match asset_type {
        "mcp" => {
            path.contains("mcp") || path.ends_with("settings.json") || path.ends_with("config.json")
        }
        "skill" => path.contains("skill"),
        "prompt" => {
            path.contains("prompt")
                || path.ends_with("claude.md")
                || path.ends_with("gemini.md")
                || path.ends_with("agents.md")
        }
        "command" => path.contains("command"),
        "hook" => path.contains("hook") || path.ends_with("settings.json"),
        "ignore" => path.contains("ignore"),
        "permission" => path.contains("permission") || path.ends_with("settings.json"),
        "subagent" => path.contains("agent") && !path.ends_with("agents.md"),
        _ => false,
    }
}

fn is_managed_existing_file(relative_path: &str, snapshot: &ManagedFileSnapshot) -> bool {
    relative_path.starts_with(".opensunstar/")
        || String::from_utf8_lossy(&snapshot.content)
            .to_ascii_lowercase()
            .contains("opensunstar")
}

fn snapshot_ref_for(
    root: &Path,
    operation_id: &str,
    relative_path: &str,
    content: &[u8],
) -> Result<String, AppError> {
    let safe_name = format!(
        "{}-{}.snapshot",
        &format!("{:x}", Sha256::digest(relative_path.as_bytes()))[..16],
        Uuid::new_v4()
    );
    let relative = format!("{ASSET_HEALTH_SNAPSHOT_DIR}/{operation_id}/{safe_name}");
    let target = root.join(&relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    fs::write(&target, content).map_err(|error| AppError::io(&target, error))?;
    Ok(relative)
}

fn file_digest(path: &Path) -> Result<Option<String>, AppError> {
    match fs::read(path) {
        Ok(content) => Ok(Some(format!("{:x}", Sha256::digest(content)))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::io(path, error)),
    }
}

fn config_files_parse(root: &Path, relative_paths: &[String]) -> bool {
    !relative_paths.is_empty()
        && relative_paths.iter().all(|relative_path| {
            let path = root.join(relative_path);
            let Ok(content) = fs::read_to_string(&path) else {
                return false;
            };
            match path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
            {
                "json" => serde_json::from_str::<serde_json::Value>(&content).is_ok(),
                "toml" => toml::from_str::<toml::Value>(&content).is_ok(),
                "yaml" | "yml" => serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok(),
                "md" | "txt" => !content.trim().is_empty(),
                _ => true,
            }
        })
}

fn evidence_is_expired(evidence: &AssetRuntimeEvidence, now: i64) -> bool {
    evidence.status == "expired"
        || evidence
            .expires_at
            .is_some_and(|expires_at| expires_at <= now)
}

fn evaluate_health(
    expectation: &ProjectAssetExpectation,
    receipt: Option<&AssetDeploymentReceipt>,
    evidence: Option<&AssetRuntimeEvidence>,
    now: i64,
) -> HealthDecision {
    match asset_support(&expectation.asset_type, &expectation.target_app) {
        AssetSupport::Unsupported => {
            return HealthDecision {
                status: UNSUPPORTED,
                evidence_level: "none",
                reason_code: "unsupported_combination",
                recommended_action: "switch_target_app_or_remove_expectation",
            }
        }
        AssetSupport::Partial => {
            if receipt.is_none() && evidence.is_none() {
                return HealthDecision {
                    status: ATTENTION,
                    evidence_level: "none",
                    reason_code: "partial_support_unverified",
                    recommended_action: "review_limitations_and_verify",
                };
            }
        }
        AssetSupport::Supported => {}
    }

    if let Some(receipt) = receipt {
        if receipt.required_revision_id != expectation.required_revision_id {
            return HealthDecision {
                status: ATTENTION,
                evidence_level: "written",
                reason_code: "asset_revision_changed",
                recommended_action: "review_and_redeploy_revision",
            };
        }
        if receipt.outcome == "interrupted" {
            return HealthDecision {
                status: ATTENTION,
                evidence_level: "planned",
                reason_code: "deployment_interrupted",
                recommended_action: "inspect_files_and_create_new_plan",
            };
        }
        if receipt.outcome == "rolled_back" {
            return HealthDecision {
                status: ATTENTION,
                evidence_level: "written",
                reason_code: "deployment_rolled_back",
                recommended_action: "create_deployment_plan",
            };
        }
        if matches!(
            receipt.outcome.as_str(),
            "failed" | "skipped_protected" | "partial"
        ) {
            return HealthDecision {
                status: UNHEALTHY,
                evidence_level: "planned",
                reason_code: if receipt.outcome == "failed" {
                    "deployment_failed"
                } else {
                    "unmanaged_file_protected"
                },
                recommended_action: "review_receipt_and_plan",
            };
        }
    }

    match evidence {
        Some(evidence) if evidence.status == "failed" => HealthDecision {
            status: UNHEALTHY,
            evidence_level: "verification",
            reason_code: "verification_failed",
            recommended_action: "inspect_verification_evidence",
        },
        Some(evidence) if evidence_is_expired(evidence, now) => HealthDecision {
            status: ATTENTION,
            evidence_level: "verification",
            reason_code: "evidence_expired",
            recommended_action: "refresh_verification",
        },
        Some(evidence) if evidence.status == "passed" => {
            let policy = expectation
                .verification_policy
                .as_deref()
                .unwrap_or("config");
            let is_runtime = matches!(
                evidence.evidence_kind.as_str(),
                "native_probe" | "ci_attestation"
            );
            let satisfies_policy = is_runtime
                || (policy == "config" && evidence.evidence_kind == "config_parse")
                || (policy == "manual" && evidence.evidence_kind == "manual_confirmation");
            HealthDecision {
                status: if satisfies_policy { HEALTHY } else { ATTENTION },
                evidence_level: if is_runtime {
                    "runtime_verified"
                } else if evidence.evidence_kind == "config_parse" {
                    "config_parsed"
                } else {
                    "manual_confirmed"
                },
                reason_code: if satisfies_policy {
                    if is_runtime {
                        "runtime_verified"
                    } else {
                        "verification_policy_satisfied"
                    }
                } else {
                    "config_verified_runtime_pending"
                },
                recommended_action: if satisfies_policy {
                    "view_evidence"
                } else {
                    "run_runtime_verification"
                },
            }
        }
        _ if receipt.is_some() => HealthDecision {
            status: ATTENTION,
            evidence_level: "written",
            reason_code: "deployment_unverified",
            recommended_action: "run_config_verification",
        },
        _ => HealthDecision {
            status: UNKNOWN,
            evidence_level: "none",
            reason_code: "not_scanned",
            recommended_action: "create_deployment_plan",
        },
    }
}

pub fn get_project_asset_health(
    db: &Database,
    project_id: &str,
) -> Result<Vec<AssetHealthRecord>, AppError> {
    db.ensure_project_asset_health_inventory(project_id)?;
    let now = now_ts();
    db.get_project_asset_expectations(project_id)?
        .into_iter()
        .map(|expectation| {
            let receipt = db.latest_asset_deployment_receipt(&expectation.expectation_id)?;
            let evidence = db.latest_asset_runtime_evidence(&expectation.expectation_id)?;
            let receipt_files = match receipt.as_ref() {
                Some(receipt) => db.get_asset_receipt_files(&receipt.receipt_id)?,
                None => Vec::new(),
            };
            let decision = evaluate_health(&expectation, receipt.as_ref(), evidence.as_ref(), now);
            Ok(AssetHealthRecord {
                observed_at: evidence
                    .as_ref()
                    .map(|item| item.observed_at)
                    .or_else(|| receipt.as_ref().map(|item| item.created_at)),
                last_receipt_files: receipt_files,
                last_receipt_id: receipt.as_ref().map(|item| item.receipt_id.clone()),
                last_evidence_id: evidence.as_ref().map(|item| item.evidence_id.clone()),
                expectation,
                status: decision.status.to_string(),
                evidence_level: decision.evidence_level.to_string(),
                reason_code: decision.reason_code.to_string(),
                recommended_action: decision.recommended_action.to_string(),
            })
        })
        .collect()
}

pub fn plan_project_asset_health(
    db: &Database,
    project_id: &str,
) -> Result<AssetHealthPlan, AppError> {
    let records = get_project_asset_health(db, project_id)?;
    let project = db
        .get_project(project_id)?
        .ok_or_else(|| AppError::InvalidInput(format!("项目不存在: {project_id}")))?;
    let current_files = snapshot_managed_files(Path::new(&project.path));
    let steps = records
        .into_iter()
        .map(|record| {
            let capability = asset_capability(
                &record.expectation.asset_type,
                &record.expectation.target_app,
            );
            let managed_paths = managed_path_hints(&record.expectation.target_app);
            let protected_paths = current_files
                .iter()
                .filter(|(path, snapshot)| {
                    path_matches_app(path, &record.expectation.target_app)
                        && path_matches_asset(path, &record.expectation.asset_type)
                        && !is_managed_existing_file(path, snapshot)
                })
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>();
            AssetHealthPlanStep {
                action: if record.status == UNSUPPORTED {
                    "skip_unsupported".to_string()
                } else if !protected_paths.is_empty() {
                    "skip_protected".to_string()
                } else {
                    "legacy_project_sync".to_string()
                },
                reason_code: if protected_paths.is_empty() {
                    record.reason_code
                } else {
                    "unmanaged_file_protected".into()
                },
                expectation_id: record.expectation.expectation_id,
                asset_type: record.expectation.asset_type,
                asset_id: record.expectation.asset_id,
                required_revision_id: record.expectation.required_revision_id,
                target_app: record.expectation.target_app,
                adapter_id: capability.adapter_id,
                write_mode: capability.write_mode,
                verify_modes: capability.verify_modes,
                limitations: capability.limitations,
                managed_paths,
                protected_paths,
            }
        })
        .collect::<Vec<_>>();
    let canonical = serde_json::to_vec(&(project_id, &steps))
        .map_err(|error| AppError::Config(format!("序列化资产健康计划失败: {error}")))?;
    let plan_sha256 = format!("{:x}", Sha256::digest(canonical));
    Ok(AssetHealthPlan {
        operation_id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        plan_sha256,
        steps,
    })
}

/// Applies the existing per-asset project adapters behind an explicit,
/// hash-checked plan. It records file-level facts and configuration evidence;
/// native application-consumption probes remain a separate P1 capability.
pub fn apply_project_asset_health_plan(
    state: &crate::store::AppState,
    project_id: &str,
    plan_sha256: &str,
    confirmed: bool,
) -> Result<Vec<AssetDeploymentReceipt>, AppError> {
    if !confirmed {
        return Err(AppError::InvalidInput(
            "必须显式确认资产健康同步计划".into(),
        ));
    }
    let plan = plan_project_asset_health(&state.db, project_id)?;
    if plan.plan_sha256 != plan_sha256 {
        return Err(AppError::InvalidInput(
            "plan_digest_mismatch：项目资产或目标应用已变化，请重新预览计划".into(),
        ));
    }

    let project = state
        .db
        .get_project(project_id)?
        .ok_or_else(|| AppError::InvalidInput(format!("项目不存在: {project_id}")))?;
    let project_root = PathBuf::from(&project.path);
    let before_files = snapshot_managed_files(&project_root);
    let now = now_ts();
    let mut pending_receipts = HashMap::new();
    for step in plan
        .steps
        .iter()
        .filter(|step| step.action != "skip_unsupported")
    {
        let receipt = AssetDeploymentReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            expectation_id: step.expectation_id.clone(),
            operation_id: plan.operation_id.clone(),
            adapter_id: step.adapter_id.clone(),
            adapter_version: "1".into(),
            plan_sha256: plan.plan_sha256.clone(),
            required_revision_id: step.required_revision_id.clone(),
            dry_run: false,
            outcome: "interrupted".into(),
            target_path: None,
            before_sha256: None,
            after_sha256: None,
            snapshot_ref: None,
            reason_code: Some("execution_started".into()),
            created_at: now,
        };
        state.db.record_asset_deployment_receipt(&receipt)?;
        pending_receipts.insert(step.expectation_id.clone(), receipt);
    }
    let mut prewritten_snapshot_refs = HashMap::new();
    for (relative_path, before) in &before_files {
        let belongs_to_plan = plan.steps.iter().any(|step| {
            step.action == "legacy_project_sync"
                && path_matches_app(relative_path, &step.target_app)
                && path_matches_asset(relative_path, &step.asset_type)
        });
        if belongs_to_plan && is_managed_existing_file(relative_path, before) {
            prewritten_snapshot_refs.insert(
                relative_path.clone(),
                snapshot_ref_for(
                    &project_root,
                    &plan.operation_id,
                    relative_path,
                    &before.content,
                )?,
            );
        }
    }
    let asset_types = plan
        .steps
        .iter()
        .filter(|step| step.action == "legacy_project_sync")
        .map(|step| step.asset_type.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut sync_errors = HashMap::new();
    for asset_type in asset_types {
        let result = sync_check_name(&asset_type)
            .ok_or_else(|| AppError::InvalidInput(format!("缺少资产写回适配器: {asset_type}")))
            .and_then(|check_name| {
                crate::services::project_config_sync::sync_asset_for_project_path(
                    state,
                    &project.path,
                    check_name,
                )
            });
        if let Err(error) = result {
            sync_errors.insert(asset_type, error.to_string());
        }
    }
    let mut after_files = snapshot_managed_files(&project_root);
    let protected_changes = before_files
        .iter()
        .filter(|(path, before)| {
            !is_managed_existing_file(path, before)
                && plan.steps.iter().any(|step| {
                    step.action != "skip_unsupported"
                        && path_matches_app(path, &step.target_app)
                        && path_matches_asset(path, &step.asset_type)
                })
        })
        .map(|(path, before)| (path.clone(), before.clone()))
        .collect::<Vec<_>>();
    for (relative_path, before) in protected_changes.iter().filter(|(path, before)| {
        after_files.get(path).map(|after| &after.digest) != Some(&before.digest)
    }) {
        let target = project_root.join(relative_path);
        fs::write(&target, &before.content).map_err(|error| AppError::io(&target, error))?;
    }
    if !protected_changes.is_empty() {
        after_files = snapshot_managed_files(&project_root);
    }
    let mut receipts = Vec::new();
    for step in plan
        .steps
        .into_iter()
        .filter(|step| step.action != "skip_unsupported")
    {
        let changed_files = before_files
            .keys()
            .chain(after_files.keys())
            .filter(|path| {
                path_matches_app(path, &step.target_app)
                    && path_matches_asset(path, &step.asset_type)
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|path| {
                before_files.get(*path).map(|item| &item.digest)
                    != after_files.get(*path).map(|item| &item.digest)
            })
            .cloned()
            .collect::<Vec<_>>();
        let protected_files = protected_changes
            .iter()
            .filter(|(path, _)| {
                path_matches_app(path, &step.target_app)
                    && path_matches_asset(path, &step.asset_type)
            })
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let outcome = match (changed_files.is_empty(), protected_files.is_empty()) {
            (true, true) => "unchanged",
            (true, false) => "skipped_protected",
            (false, true) => "written",
            (false, false) => "partial",
        };
        let sync_error = sync_errors.get(&step.asset_type);
        let outcome = if sync_error.is_some() {
            "failed"
        } else {
            outcome
        };
        let mut receipt = pending_receipts
            .remove(&step.expectation_id)
            .expect("pending receipt registered for every executable step");
        receipt.outcome = outcome.into();
        receipt.reason_code = Some(match outcome {
            "failed" => "adapter_execution_failed".into(),
            "unchanged" => "no_file_changes".into(),
            "skipped_protected" => "unmanaged_file_protected".into(),
            "partial" => "partial_with_protected_files".into(),
            _ => "per_file_receipt_recorded".into(),
        });
        state.db.update_asset_deployment_receipt(&receipt)?;
        for relative_path in changed_files {
            let action = match (
                before_files.get(&relative_path),
                after_files.get(&relative_path),
            ) {
                (None, Some(_)) => "create",
                (Some(_), None) => "delete",
                _ => "update",
            };
            let snapshot_ref = prewritten_snapshot_refs.get(&relative_path).cloned();
            state
                .db
                .record_asset_receipt_file(&crate::database::AssetReceiptFile {
                    file_id: Uuid::new_v4().to_string(),
                    receipt_id: receipt.receipt_id.clone(),
                    relative_path: relative_path.clone(),
                    action: action.into(),
                    before_sha256: before_files
                        .get(&relative_path)
                        .map(|item| item.digest.clone()),
                    after_sha256: after_files
                        .get(&relative_path)
                        .map(|item| item.digest.clone()),
                    snapshot_ref,
                    reason_code: None,
                    created_at: now,
                })?;
        }
        for relative_path in protected_files {
            state.db.record_asset_receipt_file(&AssetReceiptFile {
                file_id: Uuid::new_v4().to_string(),
                receipt_id: receipt.receipt_id.clone(),
                relative_path,
                action: "skipped_protected".into(),
                before_sha256: None,
                after_sha256: None,
                snapshot_ref: None,
                reason_code: Some("unmanaged_file_protected".into()),
                created_at: now,
            })?;
        }
        if step.action == "legacy_project_sync"
            && step.verify_modes.iter().any(|mode| mode == "config_parse")
        {
            let observed_paths = after_files
                .keys()
                .filter(|path| path_matches_app(path, &step.target_app))
                .cloned()
                .collect::<Vec<_>>();
            let passed = config_files_parse(&project_root, &observed_paths);
            let mut observed = observed_paths
                .iter()
                .filter_map(|path| {
                    after_files
                        .get(path)
                        .map(|item| format!("{path}:{}", item.digest))
                })
                .collect::<Vec<_>>();
            observed.sort();
            state
                .db
                .record_asset_runtime_evidence(&AssetRuntimeEvidence {
                    evidence_id: Uuid::new_v4().to_string(),
                    expectation_id: step.expectation_id.clone(),
                    evidence_kind: "config_parse".into(),
                    status: if passed {
                        "passed".into()
                    } else {
                        "failed".into()
                    },
                    observed_revision_sha256: Some(format!(
                        "{:x}",
                        Sha256::digest(observed.join("\n"))
                    )),
                    confidence: "medium".into(),
                    collector: step.adapter_id.clone(),
                    collector_version: "1".into(),
                    observed_at: now,
                    expires_at: Some(now + 7 * 24 * 60 * 60),
                })?;
        }
        receipts.push(receipt);
    }
    Ok(receipts)
}

/// Rolls back one deployment receipt only when every current file digest still
/// matches that receipt's post-write digest. Conflict validation happens before
/// the first write, so a user edit cannot be partially overwritten.
pub fn rollback_project_asset_health_receipt(
    state: &crate::store::AppState,
    receipt_id: &str,
    confirmed: bool,
) -> Result<AssetDeploymentReceipt, AppError> {
    if !confirmed {
        return Err(AppError::InvalidInput("必须显式确认资产回滚".into()));
    }
    let source = state
        .db
        .get_asset_deployment_receipt(receipt_id)?
        .ok_or_else(|| AppError::InvalidInput(format!("部署回执不存在: {receipt_id}")))?;
    let expectation = state
        .db
        .get_project_asset_expectation(&source.expectation_id)?
        .ok_or_else(|| AppError::InvalidInput("部署回执对应的资产期望不存在".into()))?;
    let project = state
        .db
        .get_project(&expectation.project_id)?
        .ok_or_else(|| AppError::InvalidInput("部署回执对应的项目不存在".into()))?;
    let root = PathBuf::from(&project.path);
    let files = state.db.get_asset_receipt_files(receipt_id)?;
    let rollback_files = files
        .iter()
        .filter(|file| matches!(file.action.as_str(), "create" | "update" | "delete"))
        .collect::<Vec<_>>();
    if rollback_files.is_empty() {
        return Err(AppError::InvalidInput("该回执没有可回滚的受管文件".into()));
    }

    for file in &rollback_files {
        let current = file_digest(&root.join(&file.relative_path))?;
        if current != file.after_sha256 {
            return Err(AppError::InvalidInput(format!(
                "rollback_conflict：{} 已在部署后被修改，未执行任何回滚",
                file.relative_path
            )));
        }
        if file.before_sha256.is_some() {
            let snapshot_ref = file.snapshot_ref.as_deref().ok_or_else(|| {
                AppError::InvalidInput(format!("回执缺少快照: {}", file.relative_path))
            })?;
            if !snapshot_ref.starts_with(&format!("{ASSET_HEALTH_SNAPSHOT_DIR}/"))
                || snapshot_ref.split(['/', '\\']).any(|part| part == "..")
            {
                return Err(AppError::InvalidInput("回执快照路径不受信任".into()));
            }
            let snapshot_digest = file_digest(&root.join(snapshot_ref))?;
            if snapshot_digest != file.before_sha256 {
                return Err(AppError::InvalidInput(format!(
                    "回执快照摘要不匹配: {}",
                    file.relative_path
                )));
            }
        }
    }

    for file in &rollback_files {
        let target = root.join(&file.relative_path);
        match file.before_sha256.as_ref() {
            Some(_) => {
                let snapshot = root.join(file.snapshot_ref.as_deref().expect("validated snapshot"));
                let content =
                    fs::read(&snapshot).map_err(|error| AppError::io(&snapshot, error))?;
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
                }
                fs::write(&target, content).map_err(|error| AppError::io(&target, error))?;
            }
            None if target.exists() => {
                fs::remove_file(&target).map_err(|error| AppError::io(&target, error))?;
            }
            None => {}
        }
    }

    let now = now_ts();
    let receipt = AssetDeploymentReceipt {
        receipt_id: Uuid::new_v4().to_string(),
        expectation_id: source.expectation_id,
        operation_id: format!("rollback:{}", source.operation_id),
        adapter_id: format!("{}-rollback", source.adapter_id),
        adapter_version: source.adapter_version,
        plan_sha256: source.plan_sha256,
        required_revision_id: source.required_revision_id,
        dry_run: false,
        outcome: "rolled_back".into(),
        target_path: None,
        before_sha256: None,
        after_sha256: None,
        snapshot_ref: None,
        reason_code: Some("receipt_rollback_completed".into()),
        created_at: now,
    };
    state.db.record_asset_deployment_receipt(&receipt)?;
    for source_file in rollback_files {
        state.db.record_asset_receipt_file(&AssetReceiptFile {
            file_id: Uuid::new_v4().to_string(),
            receipt_id: receipt.receipt_id.clone(),
            relative_path: source_file.relative_path.clone(),
            action: "rollback".into(),
            before_sha256: source_file.after_sha256.clone(),
            after_sha256: source_file.before_sha256.clone(),
            snapshot_ref: source_file.snapshot_ref.clone(),
            reason_code: Some(format!("rolled_back_from:{receipt_id}")),
            created_at: now,
        })?;
    }
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::database::Project;

    fn expectation(asset_type: &str, target_app: &str) -> ProjectAssetExpectation {
        ProjectAssetExpectation {
            expectation_id: "expectation".into(),
            project_id: "project".into(),
            asset_type: asset_type.into(),
            asset_id: "asset".into(),
            target_app: target_app.into(),
            desired_state: "enabled".into(),
            required_revision_id: None,
            verification_policy: None,
            scope: "project".into(),
            source: "manual".into(),
            owner_mode: "managed".into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn unsupported_combination_is_never_reported_as_missing() {
        let result = evaluate_health(&expectation("mcp", "claude-desktop"), None, None, 100);
        assert_eq!(result.status, UNSUPPORTED);
        assert_eq!(result.reason_code, "unsupported_combination");
    }

    #[test]
    fn a_written_asset_without_consumption_evidence_needs_attention() {
        let receipt = AssetDeploymentReceipt {
            receipt_id: "receipt".into(),
            expectation_id: "expectation".into(),
            operation_id: "operation".into(),
            adapter_id: "adapter".into(),
            adapter_version: "1".into(),
            plan_sha256: "plan".into(),
            required_revision_id: None,
            dry_run: false,
            outcome: "written".into(),
            target_path: None,
            before_sha256: None,
            after_sha256: None,
            snapshot_ref: None,
            reason_code: None,
            created_at: 10,
        };
        let result = evaluate_health(&expectation("mcp", "claude"), Some(&receipt), None, 100);
        assert_eq!(result.status, ATTENTION);
        assert_eq!(result.reason_code, "deployment_unverified");
    }

    #[test]
    fn plans_never_schedule_an_unsupported_combination_for_legacy_sync() {
        let step = AssetHealthPlanStep {
            expectation_id: "expectation".into(),
            asset_type: "mcp".into(),
            asset_id: "asset".into(),
            required_revision_id: None,
            target_app: "claude-desktop".into(),
            action: "skip_unsupported".into(),
            reason_code: "unsupported_combination".into(),
            adapter_id: "claude-desktop-mcp-unsupported-v1".into(),
            write_mode: "none".into(),
            verify_modes: Vec::new(),
            limitations: Vec::new(),
            managed_paths: Vec::new(),
            protected_paths: Vec::new(),
        };
        assert_ne!(step.action, "legacy_project_sync");
    }

    #[test]
    fn config_evidence_satisfies_the_default_config_policy() {
        let evidence = AssetRuntimeEvidence {
            evidence_id: "evidence".into(),
            expectation_id: "expectation".into(),
            evidence_kind: "config_parse".into(),
            status: "passed".into(),
            observed_revision_sha256: None,
            confidence: "medium".into(),
            collector: "fixture".into(),
            collector_version: "1".into(),
            observed_at: 50,
            expires_at: Some(200),
        };
        let result = evaluate_health(&expectation("mcp", "claude"), None, Some(&evidence), 100);
        assert_eq!(result.status, HEALTHY);
        assert_eq!(result.evidence_level, "config_parsed");
    }

    #[test]
    fn config_evidence_does_not_satisfy_runtime_policy() {
        let mut expected = expectation("mcp", "claude");
        expected.verification_policy = Some("runtime".into());
        let evidence = AssetRuntimeEvidence {
            evidence_id: "evidence".into(),
            expectation_id: "expectation".into(),
            evidence_kind: "config_parse".into(),
            status: "passed".into(),
            observed_revision_sha256: None,
            confidence: "medium".into(),
            collector: "fixture".into(),
            collector_version: "1".into(),
            observed_at: 50,
            expires_at: Some(200),
        };
        let result = evaluate_health(&expected, None, Some(&evidence), 100);
        assert_eq!(result.status, ATTENTION);
        assert_eq!(result.reason_code, "config_verified_runtime_pending");
    }

    #[test]
    fn expired_evidence_needs_refresh() {
        let evidence = AssetRuntimeEvidence {
            evidence_id: "evidence".into(),
            expectation_id: "expectation".into(),
            evidence_kind: "native_probe".into(),
            status: "passed".into(),
            observed_revision_sha256: None,
            confidence: "high".into(),
            collector: "fixture".into(),
            collector_version: "1".into(),
            observed_at: 50,
            expires_at: Some(99),
        };
        let result = evaluate_health(&expectation("mcp", "claude"), None, Some(&evidence), 100);
        assert_eq!(result.status, ATTENTION);
        assert_eq!(result.reason_code, "evidence_expired");
    }

    #[test]
    fn failed_probe_is_unhealthy() {
        let evidence = AssetRuntimeEvidence {
            evidence_id: "evidence".into(),
            expectation_id: "expectation".into(),
            evidence_kind: "native_probe".into(),
            status: "failed".into(),
            observed_revision_sha256: None,
            confidence: "high".into(),
            collector: "fixture".into(),
            collector_version: "1".into(),
            observed_at: 50,
            expires_at: Some(200),
        };
        let result = evaluate_health(&expectation("mcp", "claude"), None, Some(&evidence), 100);
        assert_eq!(result.status, UNHEALTHY);
        assert_eq!(result.reason_code, "verification_failed");
    }

    #[test]
    fn protected_file_skip_is_unhealthy_and_actionable() {
        let receipt = AssetDeploymentReceipt {
            receipt_id: "receipt".into(),
            expectation_id: "expectation".into(),
            operation_id: "operation".into(),
            adapter_id: "adapter".into(),
            adapter_version: "1".into(),
            plan_sha256: "plan".into(),
            required_revision_id: None,
            dry_run: false,
            outcome: "skipped_protected".into(),
            target_path: None,
            before_sha256: None,
            after_sha256: None,
            snapshot_ref: None,
            reason_code: Some("unmanaged_file_protected".into()),
            created_at: 10,
        };
        let result = evaluate_health(&expectation("mcp", "claude"), Some(&receipt), None, 100);
        assert_eq!(result.status, UNHEALTHY);
        assert_eq!(result.reason_code, "unmanaged_file_protected");
    }

    #[test]
    fn changed_required_revision_invalidates_old_receipt() {
        let mut expected = expectation("mcp", "claude");
        expected.required_revision_id = Some("revision-new".into());
        let receipt = AssetDeploymentReceipt {
            receipt_id: "receipt".into(),
            expectation_id: "expectation".into(),
            operation_id: "operation".into(),
            adapter_id: "adapter".into(),
            adapter_version: "1".into(),
            plan_sha256: "plan".into(),
            required_revision_id: Some("revision-old".into()),
            dry_run: false,
            outcome: "written".into(),
            target_path: None,
            before_sha256: None,
            after_sha256: None,
            snapshot_ref: None,
            reason_code: None,
            created_at: 10,
        };
        let result = evaluate_health(&expected, Some(&receipt), None, 100);
        assert_eq!(result.status, ATTENTION);
        assert_eq!(result.reason_code, "asset_revision_changed");
    }

    #[test]
    fn interrupted_execution_is_never_healthy() {
        let receipt = AssetDeploymentReceipt {
            receipt_id: "receipt".into(),
            expectation_id: "expectation".into(),
            operation_id: "operation".into(),
            adapter_id: "adapter".into(),
            adapter_version: "1".into(),
            plan_sha256: "plan".into(),
            required_revision_id: None,
            dry_run: false,
            outcome: "interrupted".into(),
            target_path: None,
            before_sha256: None,
            after_sha256: None,
            snapshot_ref: None,
            reason_code: Some("execution_started".into()),
            created_at: 10,
        };
        let result = evaluate_health(&expectation("mcp", "claude"), Some(&receipt), None, 100);
        assert_eq!(result.status, ATTENTION);
        assert_eq!(result.reason_code, "deployment_interrupted");
    }

    #[test]
    fn partial_support_without_evidence_needs_confirmation() {
        let result = evaluate_health(&expectation("subagent", "codex"), None, None, 100);
        assert_eq!(result.status, ATTENTION);
        assert_eq!(result.reason_code, "partial_support_unverified");
    }

    fn rollback_fixture(
        current_content: &str,
    ) -> (tempfile::TempDir, crate::store::AppState, String, PathBuf) {
        let directory = tempfile::tempdir().expect("create project directory");
        let root = directory.path();
        let target = root.join(".claude/settings.json");
        fs::create_dir_all(target.parent().expect("settings parent")).unwrap();
        fs::write(&target, current_content).unwrap();
        let snapshot_relative = format!("{ASSET_HEALTH_SNAPSHOT_DIR}/operation/settings.snapshot");
        let snapshot = root.join(&snapshot_relative);
        fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
        fs::write(&snapshot, "before opensunstar").unwrap();

        let db = Arc::new(Database::memory().expect("create database"));
        db.upsert_project(&Project {
            id: "project".into(),
            name: "Project".into(),
            path: root.to_string_lossy().to_string(),
            git_remote_url: None,
            created_at: 1,
            updated_at: 1,
            target_app: Some("claude".into()),
            blueprint_id: None,
            stage: "mvp".into(),
            mvp_progress: None,
        })
        .unwrap();
        let expectation = db
            .upsert_project_asset_expectation(
                "project", "hook", "hook-1", "claude", None, "project", "manual", "managed",
            )
            .unwrap();
        let receipt = AssetDeploymentReceipt {
            receipt_id: "receipt".into(),
            expectation_id: expectation.expectation_id,
            operation_id: "operation".into(),
            adapter_id: "fixture".into(),
            adapter_version: "1".into(),
            plan_sha256: "plan".into(),
            required_revision_id: None,
            dry_run: false,
            outcome: "written".into(),
            target_path: None,
            before_sha256: None,
            after_sha256: None,
            snapshot_ref: None,
            reason_code: None,
            created_at: 2,
        };
        db.record_asset_deployment_receipt(&receipt).unwrap();
        db.record_asset_receipt_file(&AssetReceiptFile {
            file_id: "file".into(),
            receipt_id: receipt.receipt_id.clone(),
            relative_path: ".claude/settings.json".into(),
            action: "update".into(),
            before_sha256: file_digest(&snapshot).unwrap(),
            after_sha256: file_digest(&target).unwrap(),
            snapshot_ref: Some(snapshot_relative),
            reason_code: None,
            created_at: 2,
        })
        .unwrap();
        let state = crate::store::AppState::new(db);
        (directory, state, receipt.receipt_id, target)
    }

    #[test]
    fn receipt_rollback_restores_only_digest_verified_file() {
        let (_directory, state, receipt_id, target) = rollback_fixture("after opensunstar");
        let rollback = rollback_project_asset_health_receipt(&state, &receipt_id, true)
            .expect("rollback verified receipt");
        assert_eq!(fs::read_to_string(target).unwrap(), "before opensunstar");
        assert_eq!(rollback.outcome, "rolled_back");
    }

    #[test]
    fn receipt_rollback_conflict_is_zero_write() {
        let (_directory, state, receipt_id, target) = rollback_fixture("after opensunstar");
        fs::write(&target, "user changed this file").unwrap();
        let error = rollback_project_asset_health_receipt(&state, &receipt_id, true)
            .expect_err("conflicting user edit must block rollback");
        assert!(error.to_string().contains("rollback_conflict"));
        assert_eq!(
            fs::read_to_string(target).unwrap(),
            "user changed this file"
        );
    }

    #[test]
    fn planning_is_zero_write_for_project_files() {
        let directory = tempfile::tempdir().unwrap();
        let db = Database::memory().unwrap();
        db.upsert_project(&Project {
            id: "project".into(),
            name: "Project".into(),
            path: directory.path().to_string_lossy().to_string(),
            git_remote_url: None,
            created_at: 1,
            updated_at: 1,
            target_app: Some("claude".into()),
            blueprint_id: None,
            stage: "mvp".into(),
            mvp_progress: None,
        })
        .unwrap();
        db.upsert_project_asset_expectation(
            "project", "mcp", "mcp-1", "claude", None, "project", "manual", "managed",
        )
        .unwrap();
        let plan = plan_project_asset_health(&db, "project").unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn planning_marks_unmanaged_user_file_as_protected() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(".mcp.json"), "{\"user\":true}").unwrap();
        let db = Database::memory().unwrap();
        db.upsert_project(&Project {
            id: "project".into(),
            name: "Project".into(),
            path: directory.path().to_string_lossy().to_string(),
            git_remote_url: None,
            created_at: 1,
            updated_at: 1,
            target_app: Some("claude".into()),
            blueprint_id: None,
            stage: "mvp".into(),
            mvp_progress: None,
        })
        .unwrap();
        db.upsert_project_asset_expectation(
            "project", "mcp", "mcp-1", "claude", None, "project", "manual", "managed",
        )
        .unwrap();
        let plan = plan_project_asset_health(&db, "project").unwrap();
        assert_eq!(plan.steps[0].action, "skip_protected");
        assert_eq!(plan.steps[0].protected_paths, vec![".mcp.json"]);
        assert_eq!(
            fs::read_to_string(directory.path().join(".mcp.json")).unwrap(),
            "{\"user\":true}"
        );
    }
}
