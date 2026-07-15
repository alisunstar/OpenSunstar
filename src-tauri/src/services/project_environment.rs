//! Project-bound environment snapshots.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::database::{Project, ProjectEnvironmentSnapshot};
use crate::error::AppError;
use crate::prompt_files::prompt_file_path;
use crate::services::marker_merge::{inject_markdown_section, PROMPT_SECTION_ID};
use crate::services::{McpService, PromptService, ProviderService, SkillService};
use crate::store::AppState;

const PAYLOAD_SCHEMA_VERSION: u32 = 2;
const DEFAULT_APPS: &[AppType] = &[
    AppType::Claude,
    AppType::Codex,
    AppType::Gemini,
    AppType::OpenCode,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ProjectEnvironmentDimension {
    Provider,
    Mcp,
    Skills,
    Prompt,
}

impl ProjectEnvironmentDimension {
    pub fn all() -> Vec<Self> {
        vec![Self::Provider, Self::Mcp, Self::Skills, Self::Prompt]
    }
}

fn default_included_dimensions() -> Vec<ProjectEnvironmentDimension> {
    ProjectEnvironmentDimension::all()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppEnvironmentState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default)]
    pub mcp: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentPayload {
    pub schema_version: u32,
    pub captured_at: String,
    #[serde(default = "default_included_dimensions")]
    pub included_dimensions: Vec<ProjectEnvironmentDimension>,
    #[serde(default)]
    pub apps: BTreeMap<String, AppEnvironmentState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentSnapshotDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub payload: ProjectEnvironmentPayload,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_applied_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentDiff {
    pub app: String,
    pub dimension: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentApplyPreview {
    pub snapshot: ProjectEnvironmentSnapshotDto,
    pub diff: Vec<ProjectEnvironmentDiff>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentVerification {
    pub app: String,
    pub dimension: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentApplyReceipt {
    pub operation_id: String,
    pub operation: String,
    pub snapshot_id: String,
    pub project_id: String,
    pub created_at: String,
    pub diff: Vec<ProjectEnvironmentDiff>,
    pub before: ProjectEnvironmentPayload,
    pub target: ProjectEnvironmentPayload,
    pub after: ProjectEnvironmentPayload,
    pub verifications: Vec<ProjectEnvironmentVerification>,
    pub warnings: Vec<String>,
}

fn now_ts() -> i64 {
    Utc::now().timestamp()
}

fn normalize_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("项目环境快照名称不能为空".into()));
    }
    Ok(trimmed.chars().take(80).collect())
}

fn normalize_dimensions(
    dimensions: &[ProjectEnvironmentDimension],
) -> Result<Vec<ProjectEnvironmentDimension>, AppError> {
    let selected: BTreeSet<ProjectEnvironmentDimension> = dimensions.iter().copied().collect();
    if selected.is_empty() {
        return Err(AppError::InvalidInput(
            "项目环境快照至少需要包含一项内容".into(),
        ));
    }
    Ok(ProjectEnvironmentDimension::all()
        .into_iter()
        .filter(|dimension| selected.contains(dimension))
        .collect())
}

fn includes(payload: &ProjectEnvironmentPayload, dimension: ProjectEnvironmentDimension) -> bool {
    payload.included_dimensions.contains(&dimension)
}

fn project_apps(project: &Project) -> Vec<AppType> {
    if let Some(target) = project.target_app.as_deref() {
        if let Ok(app) = AppType::from_str(target) {
            return vec![app];
        }
    }
    DEFAULT_APPS.to_vec()
}

fn parse_snapshot(
    snapshot: ProjectEnvironmentSnapshot,
) -> Result<ProjectEnvironmentSnapshotDto, AppError> {
    let mut payload: ProjectEnvironmentPayload = serde_json::from_str(&snapshot.payload)
        .map_err(|e| AppError::Config(format!("解析项目环境快照失败: {e}")))?;
    payload.included_dimensions = normalize_dimensions(&payload.included_dimensions)?;
    Ok(ProjectEnvironmentSnapshotDto {
        id: snapshot.id,
        project_id: snapshot.project_id,
        name: snapshot.name,
        payload,
        created_at: snapshot.created_at,
        updated_at: snapshot.updated_at,
        last_applied_at: snapshot.last_applied_at,
    })
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn prompt_value(value: &Option<String>) -> Vec<String> {
    value.clone().into_iter().collect()
}

fn vector_value(values: &[String]) -> Vec<String> {
    sorted(values.to_vec())
}

fn app_type_from_payload_key(app: &str) -> Result<AppType, AppError> {
    AppType::from_str(app)
}

pub fn capture_current_environment(
    state: &AppState,
    apps: &[AppType],
    included_dimensions: &[ProjectEnvironmentDimension],
) -> Result<ProjectEnvironmentPayload, AppError> {
    let included_dimensions = normalize_dimensions(included_dimensions)?;
    let mcp_servers = state.db.get_all_mcp_servers()?;
    let skills = state.db.get_all_installed_skills()?;
    let mut payload = ProjectEnvironmentPayload {
        schema_version: PAYLOAD_SCHEMA_VERSION,
        captured_at: Utc::now().to_rfc3339(),
        included_dimensions,
        apps: BTreeMap::new(),
    };

    for app in apps {
        let provider = if !includes(&payload, ProjectEnvironmentDimension::Provider)
            || app.is_additive_mode()
        {
            None
        } else {
            crate::settings::get_effective_current_provider(&state.db, app)?
        };
        let mcp = if includes(&payload, ProjectEnvironmentDimension::Mcp) {
            sorted(
                mcp_servers
                    .values()
                    .filter(|server| server.apps.is_enabled_for(app))
                    .map(|server| server.id.clone())
                    .collect(),
            )
        } else {
            Vec::new()
        };
        let skill_ids = if includes(&payload, ProjectEnvironmentDimension::Skills) {
            sorted(
                skills
                    .values()
                    .filter(|skill| skill.apps.is_enabled_for(app))
                    .map(|skill| skill.id.clone())
                    .collect(),
            )
        } else {
            Vec::new()
        };
        let prompt = if includes(&payload, ProjectEnvironmentDimension::Prompt) {
            state
                .db
                .get_prompts(app.as_str())
                .map(|prompts| {
                    prompts
                        .values()
                        .find(|prompt| prompt.enabled && !prompt.is_fragment)
                        .map(|prompt| prompt.id.clone())
                })
                .unwrap_or(None)
        } else {
            None
        };

        payload.apps.insert(
            app.as_str().to_string(),
            AppEnvironmentState {
                provider,
                mcp,
                skills: skill_ids,
                prompt,
            },
        );
    }

    Ok(payload)
}

fn diff_payload(
    before: &ProjectEnvironmentPayload,
    target: &ProjectEnvironmentPayload,
) -> Vec<ProjectEnvironmentDiff> {
    let mut diffs = Vec::new();
    for (app, target_state) in &target.apps {
        let before_state = before.apps.get(app);
        let before_provider = before_state.and_then(|state| state.provider.clone());
        if includes(target, ProjectEnvironmentDimension::Provider)
            && target_state.provider.is_some()
            && before_provider != target_state.provider
        {
            diffs.push(ProjectEnvironmentDiff {
                app: app.clone(),
                dimension: "provider".into(),
                before: prompt_value(&before_provider),
                after: prompt_value(&target_state.provider),
                action: "switch".into(),
            });
        }

        if includes(target, ProjectEnvironmentDimension::Mcp) {
            let before_mcp = before_state
                .map(|state| vector_value(&state.mcp))
                .unwrap_or_default();
            let after_mcp = vector_value(&target_state.mcp);
            if before_mcp != after_mcp {
                diffs.push(ProjectEnvironmentDiff {
                    app: app.clone(),
                    dimension: "mcp".into(),
                    before: before_mcp,
                    after: after_mcp,
                    action: "sync-set".into(),
                });
            }
        }

        if includes(target, ProjectEnvironmentDimension::Skills) {
            let before_skills = before_state
                .map(|state| vector_value(&state.skills))
                .unwrap_or_default();
            let after_skills = vector_value(&target_state.skills);
            if before_skills != after_skills {
                diffs.push(ProjectEnvironmentDiff {
                    app: app.clone(),
                    dimension: "skills".into(),
                    before: before_skills,
                    after: after_skills,
                    action: "sync-set".into(),
                });
            }
        }

        if includes(target, ProjectEnvironmentDimension::Prompt) {
            let before_prompt = before_state.and_then(|state| state.prompt.clone());
            if before_prompt != target_state.prompt {
                diffs.push(ProjectEnvironmentDiff {
                    app: app.clone(),
                    dimension: "prompt".into(),
                    before: prompt_value(&before_prompt),
                    after: prompt_value(&target_state.prompt),
                    action: "activate".into(),
                });
            }
        }
    }
    diffs
}

fn plan_toggles(
    current: Vec<(String, bool)>,
    target_ids: &[String],
) -> (Vec<(String, bool)>, Vec<String>) {
    let existing: BTreeSet<String> = current.iter().map(|(id, _)| id.clone()).collect();
    let target: BTreeSet<String> = target_ids.iter().cloned().collect();
    let toggles = current
        .into_iter()
        .filter(|(id, enabled)| target.contains(id) != *enabled)
        .map(|(id, enabled)| (id, !enabled))
        .collect();
    let dangling = target
        .into_iter()
        .filter(|id| !existing.contains(id))
        .collect();
    (toggles, dangling)
}

fn clear_prompt_for_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
    let mut prompts = state.db.get_prompts(app.as_str())?;
    for prompt in prompts.values_mut() {
        if prompt.enabled {
            prompt.enabled = false;
            state.db.save_prompt(app.as_str(), prompt)?;
        }
    }
    let path = prompt_file_path(app)?;
    if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let merged = inject_markdown_section(&existing, PROMPT_SECTION_ID, "");
        write_text_file(&path, &merged)?;
    }
    Ok(())
}

fn apply_payload(
    state: &AppState,
    payload: &ProjectEnvironmentPayload,
) -> Result<Vec<String>, AppError> {
    let mut warnings = Vec::new();

    for (app_key, target_state) in &payload.apps {
        let app = app_type_from_payload_key(app_key)?;
        let app_str = app.as_str();

        if includes(payload, ProjectEnvironmentDimension::Provider) {
            if let Some(provider_id) = target_state.provider.as_deref() {
                let providers = state.db.get_all_providers(app_str)?;
                if providers.contains_key(provider_id) {
                    let current = crate::settings::get_effective_current_provider(&state.db, &app)?;
                    if current.as_deref() != Some(provider_id) {
                        match ProviderService::switch(state, app.clone(), provider_id) {
                            Ok(result) => warnings.extend(result.warnings),
                            Err(e) => warnings
                                .push(format!("[{app_str}] 切换供应商 {provider_id} 失败: {e}")),
                        }
                    }
                } else {
                    warnings.push(format!("[{app_str}] 供应商 {provider_id} 已不存在，跳过"));
                }
            }
        }

        if includes(payload, ProjectEnvironmentDimension::Mcp) {
            let servers = state.db.get_all_mcp_servers()?;
            let current_mcp = servers
                .values()
                .map(|server| (server.id.clone(), server.apps.is_enabled_for(&app)))
                .collect();
            let (mcp_toggles, dangling_mcp) = plan_toggles(current_mcp, &target_state.mcp);
            for id in dangling_mcp {
                warnings.push(format!("[{app_str}] MCP {id} 已不存在，跳过"));
            }
            for (id, enabled) in mcp_toggles {
                if let Err(e) = McpService::toggle_app(state, &id, app.clone(), enabled) {
                    warnings.push(format!("[{app_str}] MCP {id} -> {enabled} 失败: {e}"));
                }
            }
        }

        if includes(payload, ProjectEnvironmentDimension::Skills) {
            let skills = state.db.get_all_installed_skills()?;
            let current_skills = skills
                .values()
                .map(|skill| (skill.id.clone(), skill.apps.is_enabled_for(&app)))
                .collect();
            let (skill_toggles, dangling_skills) =
                plan_toggles(current_skills, &target_state.skills);
            for id in dangling_skills {
                warnings.push(format!("[{app_str}] Skill {id} 已不存在，跳过"));
            }
            for (id, enabled) in skill_toggles {
                if let Err(e) = SkillService::toggle_app(&state.db, &id, &app, enabled) {
                    warnings.push(format!("[{app_str}] Skill {id} -> {enabled} 失败: {e}"));
                }
            }
        }

        if includes(payload, ProjectEnvironmentDimension::Prompt) {
            match target_state.prompt.as_deref() {
                Some(prompt_id) => {
                    let prompts = state.db.get_prompts(app_str)?;
                    if prompts.contains_key(prompt_id) {
                        if let Err(e) = PromptService::enable_prompt(state, &app, prompt_id) {
                            warnings.push(format!("[{app_str}] 启用 Prompt {prompt_id} 失败: {e}"));
                        }
                    } else {
                        warnings.push(format!("[{app_str}] Prompt {prompt_id} 已不存在，跳过"));
                    }
                }
                None => {
                    if let Err(e) = clear_prompt_for_app(state, &app) {
                        warnings.push(format!("[{app_str}] 清空 Prompt 激活项失败: {e}"));
                    }
                }
            }
        }
    }

    Ok(warnings)
}

fn verify_payload(
    after: &ProjectEnvironmentPayload,
    target: &ProjectEnvironmentPayload,
) -> Vec<ProjectEnvironmentVerification> {
    let mut verifications = Vec::new();
    for (app, target_state) in &target.apps {
        let after_state = after.apps.get(app);

        if includes(target, ProjectEnvironmentDimension::Provider) {
            if let Some(target_provider) = target_state.provider.as_deref() {
                let actual = after_state.and_then(|state| state.provider.as_deref());
                verifications.push(ProjectEnvironmentVerification {
                    app: app.clone(),
                    dimension: "provider".into(),
                    passed: actual == Some(target_provider),
                    detail: actual.unwrap_or("none").to_string(),
                });
            }
        }

        if includes(target, ProjectEnvironmentDimension::Mcp) {
            let actual_mcp = after_state
                .map(|state| vector_value(&state.mcp))
                .unwrap_or_default();
            let expected_mcp = vector_value(&target_state.mcp);
            verifications.push(ProjectEnvironmentVerification {
                app: app.clone(),
                dimension: "mcp".into(),
                passed: actual_mcp == expected_mcp,
                detail: format!("{}/{}", actual_mcp.len(), expected_mcp.len()),
            });
        }

        if includes(target, ProjectEnvironmentDimension::Skills) {
            let actual_skills = after_state
                .map(|state| vector_value(&state.skills))
                .unwrap_or_default();
            let expected_skills = vector_value(&target_state.skills);
            verifications.push(ProjectEnvironmentVerification {
                app: app.clone(),
                dimension: "skills".into(),
                passed: actual_skills == expected_skills,
                detail: format!("{}/{}", actual_skills.len(), expected_skills.len()),
            });
        }

        if includes(target, ProjectEnvironmentDimension::Prompt) {
            let actual_prompt = after_state.and_then(|state| state.prompt.clone());
            verifications.push(ProjectEnvironmentVerification {
                app: app.clone(),
                dimension: "prompt".into(),
                passed: actual_prompt == target_state.prompt,
                detail: actual_prompt.unwrap_or_else(|| "none".into()),
            });
        }
    }
    verifications
}

fn apps_from_payload(payload: &ProjectEnvironmentPayload) -> Result<Vec<AppType>, AppError> {
    payload
        .apps
        .keys()
        .map(|app| app_type_from_payload_key(app))
        .collect()
}

fn write_environment_log(project: &Project, receipt: &ProjectEnvironmentApplyReceipt) {
    let passed = receipt
        .verifications
        .iter()
        .filter(|item| item.passed)
        .count();
    let total = receipt.verifications.len();
    if let Err(e) = crate::services::flow_orchestrator::append_orchestration_log(
        &project.path,
        serde_json::json!({
            "event": "project_environment_snapshot",
            "operation": receipt.operation,
            "snapshotId": receipt.snapshot_id,
            "includedDimensions": receipt.target.included_dimensions,
            "diffCount": receipt.diff.len(),
            "warnings": receipt.warnings,
            "verification": {
                "passed": passed,
                "total": total
            }
        }),
    ) {
        log::warn!("写入项目环境快照时间线失败: {e}");
    }
}

pub struct ProjectEnvironmentService;

impl ProjectEnvironmentService {
    pub fn list(
        state: &AppState,
        project_id: &str,
    ) -> Result<Vec<ProjectEnvironmentSnapshotDto>, AppError> {
        state
            .db
            .get_project_environment_snapshots(project_id)?
            .into_iter()
            .map(parse_snapshot)
            .collect()
    }

    pub fn create(
        state: &AppState,
        project_id: &str,
        name: &str,
        included_dimensions: &[ProjectEnvironmentDimension],
    ) -> Result<ProjectEnvironmentSnapshotDto, AppError> {
        let project = state
            .db
            .get_project(project_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("项目不存在: {project_id}")))?;
        let name = normalize_name(name)?;
        let included_dimensions = normalize_dimensions(included_dimensions)?;
        let apps = project_apps(&project);
        let payload = capture_current_environment(state, &apps, &included_dimensions)?;
        let now = now_ts();
        let snapshot = ProjectEnvironmentSnapshot {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            name,
            payload: serde_json::to_string(&payload)
                .map_err(|e| AppError::Config(format!("序列化项目环境快照失败: {e}")))?,
            created_at: now,
            updated_at: now,
            last_applied_at: None,
            last_apply_receipt: None,
        };
        state.db.save_project_environment_snapshot(&snapshot)?;
        parse_snapshot(snapshot)
    }

    pub fn delete(state: &AppState, snapshot_id: &str) -> Result<bool, AppError> {
        state.db.delete_project_environment_snapshot(snapshot_id)
    }

    pub fn preview_apply(
        state: &AppState,
        snapshot_id: &str,
    ) -> Result<ProjectEnvironmentApplyPreview, AppError> {
        let raw = state
            .db
            .get_project_environment_snapshot(snapshot_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("项目环境快照不存在: {snapshot_id}")))?;
        let snapshot = parse_snapshot(raw)?;
        let apps = apps_from_payload(&snapshot.payload)?;
        let before =
            capture_current_environment(state, &apps, &snapshot.payload.included_dimensions)?;
        let diff = diff_payload(&before, &snapshot.payload);
        Ok(ProjectEnvironmentApplyPreview {
            snapshot,
            diff,
            warnings: Vec::new(),
        })
    }

    pub fn apply(
        state: &AppState,
        snapshot_id: &str,
    ) -> Result<ProjectEnvironmentApplyReceipt, AppError> {
        let raw = state
            .db
            .get_project_environment_snapshot(snapshot_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("项目环境快照不存在: {snapshot_id}")))?;
        let snapshot = parse_snapshot(raw)?;
        let project = state.db.get_project(&snapshot.project_id)?.ok_or_else(|| {
            AppError::InvalidInput(format!("项目不存在: {}", snapshot.project_id))
        })?;
        let apps = apps_from_payload(&snapshot.payload)?;
        let before =
            capture_current_environment(state, &apps, &snapshot.payload.included_dimensions)?;
        let diff = diff_payload(&before, &snapshot.payload);
        let warnings = apply_payload(state, &snapshot.payload)?;
        let after =
            capture_current_environment(state, &apps, &snapshot.payload.included_dimensions)?;
        let verifications = verify_payload(&after, &snapshot.payload);
        let receipt = ProjectEnvironmentApplyReceipt {
            operation_id: Uuid::new_v4().to_string(),
            operation: "apply".into(),
            snapshot_id: snapshot.id.clone(),
            project_id: snapshot.project_id.clone(),
            created_at: Utc::now().to_rfc3339(),
            diff,
            before,
            target: snapshot.payload,
            after,
            verifications,
            warnings,
        };
        let receipt_json = serde_json::to_string(&receipt)
            .map_err(|e| AppError::Config(format!("序列化项目环境快照回执失败: {e}")))?;
        state.db.update_project_environment_snapshot_receipt(
            &receipt.snapshot_id,
            now_ts(),
            &receipt_json,
        )?;
        write_environment_log(&project, &receipt);
        Ok(receipt)
    }

    pub fn rollback(
        state: &AppState,
        snapshot_id: &str,
    ) -> Result<ProjectEnvironmentApplyReceipt, AppError> {
        let raw = state
            .db
            .get_project_environment_snapshot(snapshot_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("项目环境快照不存在: {snapshot_id}")))?;
        let project = state
            .db
            .get_project(&raw.project_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("项目不存在: {}", raw.project_id)))?;
        let last_receipt = raw
            .last_apply_receipt
            .as_deref()
            .ok_or_else(|| AppError::InvalidInput("该项目环境快照还没有可回滚的应用回执".into()))?;
        let previous: ProjectEnvironmentApplyReceipt = serde_json::from_str(last_receipt)
            .map_err(|e| AppError::Config(format!("解析项目环境快照回滚回执失败: {e}")))?;
        let target = previous.before;
        let apps = apps_from_payload(&target)?;
        let before = capture_current_environment(state, &apps, &target.included_dimensions)?;
        let diff = diff_payload(&before, &target);
        let warnings = apply_payload(state, &target)?;
        let after = capture_current_environment(state, &apps, &target.included_dimensions)?;
        let verifications = verify_payload(&after, &target);
        let receipt = ProjectEnvironmentApplyReceipt {
            operation_id: Uuid::new_v4().to_string(),
            operation: "rollback".into(),
            snapshot_id: raw.id.clone(),
            project_id: raw.project_id.clone(),
            created_at: Utc::now().to_rfc3339(),
            diff,
            before,
            target,
            after,
            verifications,
            warnings,
        };
        let receipt_json = serde_json::to_string(&receipt)
            .map_err(|e| AppError::Config(format!("序列化项目环境快照回滚回执失败: {e}")))?;
        state.db.update_project_environment_snapshot_receipt(
            &receipt.snapshot_id,
            now_ts(),
            &receipt_json,
        )?;
        write_environment_log(&project, &receipt);
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(
        provider: &str,
        mcp: &[&str],
        skills: &[&str],
        prompt: Option<&str>,
    ) -> ProjectEnvironmentPayload {
        let mut apps = BTreeMap::new();
        apps.insert(
            "claude".into(),
            AppEnvironmentState {
                provider: Some(provider.into()),
                mcp: mcp.iter().map(|v| v.to_string()).collect(),
                skills: skills.iter().map(|v| v.to_string()).collect(),
                prompt: prompt.map(str::to_string),
            },
        );
        ProjectEnvironmentPayload {
            schema_version: PAYLOAD_SCHEMA_VERSION,
            captured_at: "test".into(),
            included_dimensions: ProjectEnvironmentDimension::all(),
            apps,
        }
    }

    fn scoped_payload(
        dimensions: &[ProjectEnvironmentDimension],
        provider: &str,
        mcp: &[&str],
        skills: &[&str],
        prompt: Option<&str>,
    ) -> ProjectEnvironmentPayload {
        let mut payload = payload(provider, mcp, skills, prompt);
        payload.included_dimensions = dimensions.to_vec();
        payload
    }

    #[test]
    fn diff_reports_changed_dimensions() {
        let before = payload("p1", &["m1"], &["s1"], Some("pr1"));
        let after = payload("p2", &["m1", "m2"], &[], None);
        let diff = diff_payload(&before, &after);
        assert_eq!(diff.len(), 4);
        assert!(diff.iter().any(|item| item.dimension == "provider"));
        assert!(diff.iter().any(|item| item.dimension == "mcp"));
        assert!(diff.iter().any(|item| item.dimension == "skills"));
        assert!(diff.iter().any(|item| item.dimension == "prompt"));
    }

    #[test]
    fn legacy_payload_defaults_to_all_dimensions() {
        let payload: ProjectEnvironmentPayload = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "capturedAt": "test",
            "apps": {}
        }))
        .unwrap();

        assert_eq!(
            payload.included_dimensions,
            ProjectEnvironmentDimension::all()
        );
    }

    #[test]
    fn diff_ignores_unselected_dimensions() {
        let before = payload("p1", &["m1"], &["s1"], Some("pr1"));
        let target = scoped_payload(
            &[ProjectEnvironmentDimension::Skills],
            "p2",
            &["m2"],
            &["s2"],
            Some("pr2"),
        );

        let diff = diff_payload(&before, &target);

        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].dimension, "skills");
    }

    #[test]
    fn selected_empty_dimension_means_clear() {
        let before = payload("p1", &["m1"], &["s1"], Some("pr1"));
        let target = scoped_payload(
            &[ProjectEnvironmentDimension::Mcp],
            "p1",
            &[],
            &["s1"],
            Some("pr1"),
        );

        let diff = diff_payload(&before, &target);

        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].dimension, "mcp");
        assert!(diff[0].after.is_empty());
    }

    #[test]
    fn empty_scope_is_rejected() {
        assert!(normalize_dimensions(&[]).is_err());
    }
}
