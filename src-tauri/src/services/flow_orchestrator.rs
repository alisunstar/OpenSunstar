//! SDD workflow orchestration: presets, `.specs/` indexing, stage gates (flow-kit compatible).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::services::design_contract::{InstallAuditSummary, InstallFileEntry};
use crate::services::orchestration_plan::{
    execute_text_write_plan, verification, OrchestrationReceipt, OrchestrationStepStatus,
    PlannedTextWrite,
};
use crate::services::project_artifacts::project_workspace_exists;

const OPENSUNSTAR_DIR: &str = ".opensunstar";
const SPECS_DIR: &str = ".specs";
const FLOW_KIT_GO: &str = "flow-kit/GO.md";
const PROFILE_FILENAME: &str = "workflow.profile.json";
const ORCH_LOG_FILENAME: &str = "orchestration.log.jsonl";
const STATE_FILENAME: &str = "STATE.md";
const CI_WORKFLOW_REL: &str = ".github/workflows/opensunstar-flow-gate.yml";

const MODULES_JSON: &str = include_str!("../../assets/workflow/modules.json");
const PRESET_MVP: &str = include_str!("../../assets/workflow/presets/mvp.json");
const PRESET_STANDARD: &str = include_str!("../../assets/workflow/presets/standard.json");
const PRESET_FULL: &str = include_str!("../../assets/workflow/presets/full.json");
const PRESET_BROWNFIELD: &str =
    include_str!("../../assets/workflow/presets/brownfield-intake.json");
const PRESET_REVIEW: &str = include_str!("../../assets/workflow/presets/review-only.json");

const PRESET_BUILTINS: &[&str] = &[
    PRESET_MVP,
    PRESET_STANDARD,
    PRESET_FULL,
    PRESET_BROWNFIELD,
    PRESET_REVIEW,
];

const RESERVED_SPECS_ENTRIES: &[&str] = &["archive", "health", "adr"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowModule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub source: String,
    pub description: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowModuleCatalog {
    #[serde(default)]
    schema_version: u32,
    modules: Vec<WorkflowModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowArtifactSpec {
    pub file: String,
    #[serde(default = "default_change_scope")]
    pub scope: String,
    #[serde(default)]
    pub optional: bool,
}

fn default_change_scope() -> String {
    "change".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStageSkipWhen {
    #[serde(default)]
    pub project_type: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStage {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub skip_when: Option<WorkflowStageSkipWhen>,
    #[serde(default)]
    pub artifacts: Vec<WorkflowArtifactSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowPresetPaths {
    #[serde(default)]
    pub frontend: Vec<String>,
    #[serde(default)]
    pub backend: Vec<String>,
    #[serde(default)]
    pub cli: Vec<String>,
    #[serde(default)]
    pub mvp: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub description: String,
    #[serde(default)]
    pub r3_tier: Option<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    pub stages: Vec<WorkflowStage>,
    pub paths: WorkflowPresetPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowPresetSummary {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub description: String,
    #[serde(default)]
    pub r3_tier: Option<String>,
    #[serde(default)]
    pub module_count: usize,
    #[serde(default)]
    pub stage_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowProfile {
    pub schema_version: u32,
    pub preset_id: String,
    pub project_type: String,
    pub modules: Vec<String>,
    pub resolved_stages: Vec<String>,
    #[serde(default)]
    pub active_change_id: Option<String>,
    pub exported_at: String,
    #[serde(default)]
    pub opensunstar_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_warnings: Vec<String>,
}

/// R9.6 safety valve: hardcoded guardrails that cannot be weakened by user config.
const R96_MAX_AUTO_RETRY: u32 = 3;
const R96_ROLE_SEPARATION: bool = true;
const R96_REQUIRE_DIFF_BOUNDARY: bool = true;

const FLOW_CONFIG_FILENAME: &str = "flow-config.yaml";
const REVIEW_LENSES_4R: &[&str] = &[
    "review-risk",
    "review-resilience",
    "review-readability",
    "review-reliability",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowConfigGate {
    #[serde(rename = "type")]
    pub gate_type: String,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowConfigStage {
    pub id: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<FlowConfigGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowConfigRules {
    pub max_auto_retry: u32,
    pub role_separation: bool,
    pub require_diff_boundary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowConfigReviewTier {
    pub enabled: bool,
    pub lenses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_lenses: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_lens: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_lines_threshold: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowConfigReviewPolicy {
    pub mode: String,
    pub trivial: FlowConfigReviewTier,
    pub standard: FlowConfigReviewTier,
    pub hot_path: FlowConfigReviewTier,
    pub large_diff: FlowConfigReviewTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowConfig {
    pub schema_version: u32,
    pub preset_id: String,
    pub project_type: String,
    pub modules: Vec<String>,
    pub stages: Vec<FlowConfigStage>,
    pub rules: FlowConfigRules,
    pub review_policy: FlowConfigReviewPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowWritePlan {
    pub files: Vec<InstallFileEntry>,
    pub audit: InstallAuditSummary,
    pub semantic_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactStatus {
    pub file: String,
    pub relative_path: String,
    pub exists: bool,
    pub non_empty: bool,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub total: u32,
    pub pending: u32,
    pub in_progress: u32,
    pub done: u32,
    pub blocked: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecsChangeIndex {
    pub change_id: String,
    pub artifact_completeness: u8,
    pub artifacts: Vec<ArtifactStatus>,
    #[serde(default)]
    pub task_summary: Option<TaskSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecsWorkflowIndex {
    pub project_path: String,
    pub workspace_exists: bool,
    pub has_flow_kit: bool,
    pub has_flow_config: bool,
    pub has_specs_dir: bool,
    #[serde(default)]
    pub active_change_id: Option<String>,
    #[serde(default)]
    pub saved_profile: Option<WorkflowProfile>,
    pub changes: Vec<SpecsChangeIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageGateResult {
    pub allowed: bool,
    pub target_stage: String,
    pub change_id: String,
    pub missing_artifacts: Vec<String>,
    pub satisfied_artifacts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationLogEntry {
    #[serde(default)]
    pub ts: Option<String>,
    pub event: String,
    pub summary: String,
    pub payload: Value,
}

pub fn list_workflow_modules(project_path: Option<&str>) -> Result<Vec<WorkflowModule>, AppError> {
    let catalog: WorkflowModuleCatalog = serde_json::from_str(MODULES_JSON)
        .map_err(|e| AppError::Message(format!("解析 workflow modules 失败: {e}")))?;
    let mut modules = catalog.modules;
    if let Some(pp) = project_path {
        let user_mods = load_user_modules(pp);
        modules.extend(user_mods);
    }
    Ok(modules)
}

fn load_all_presets(project_path: Option<&str>) -> Result<Vec<WorkflowPreset>, AppError> {
    let mut out = Vec::new();
    for raw in PRESET_BUILTINS {
        let preset: WorkflowPreset = serde_json::from_str(raw)
            .map_err(|e| AppError::Message(format!("解析 workflow preset 失败: {e}")))?;
        out.push(preset);
    }
    if let Some(pp) = project_path {
        let user_presets = load_user_presets(pp);
        out.extend(user_presets);
    }
    Ok(out)
}

/// Returns the directory path for user-defined presets.
fn user_presets_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(OPENSUNSTAR_DIR)
        .join("workflow.presets")
}

/// Returns the directory path for user-defined modules.
fn user_modules_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(OPENSUNSTAR_DIR)
        .join("workflow.modules")
}

/// Loads user presets from `.opensunstar/workflow.presets/` directory.
/// Supports .json, .yaml, .yml file extensions.
fn load_user_presets(project_path: &str) -> Vec<WorkflowPreset> {
    let dir = user_presets_dir(project_path);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut presets = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(preset) = load_file_as_preset(&path) {
                presets.push(preset);
            }
        }
    }
    presets
}

/// Attempts to parse a file as a WorkflowPreset (JSON or YAML).
fn load_file_as_preset(path: &Path) -> Option<WorkflowPreset> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "json" | "yaml" | "yml") {
        return None;
    }
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("读取用户 preset {:?} 失败: {e}", path);
            return None;
        }
    };
    let result: Result<WorkflowPreset, String> = if ext == "yaml" || ext == "yml" {
        serde_yaml::from_str::<WorkflowPreset>(&raw).map_err(|e| e.to_string())
    } else {
        serde_json::from_str::<WorkflowPreset>(&raw).map_err(|e| e.to_string())
    };
    match result {
        Ok(preset) => Some(preset),
        Err(e) => {
            log::warn!("解析用户 preset {:?} 失败: {e}", path);
            None
        }
    }
}

/// Loads user modules from `.opensunstar/workflow.modules/` directory.
fn load_user_modules(project_path: &str) -> Vec<WorkflowModule> {
    let dir = user_modules_dir(project_path);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut modules = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(module) = load_file_as_module(&path) {
                modules.push(module);
            }
        }
    }
    modules
}

/// Attempts to parse a file as a WorkflowModule (JSON or YAML).
fn load_file_as_module(path: &Path) -> Option<WorkflowModule> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "json" | "yaml" | "yml") {
        return None;
    }
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("读取用户 module {:?} 失败: {e}", path);
            return None;
        }
    };
    let result: Result<WorkflowModule, String> = if ext == "yaml" || ext == "yml" {
        serde_yaml::from_str::<WorkflowModule>(&raw).map_err(|e| e.to_string())
    } else {
        serde_json::from_str::<WorkflowModule>(&raw).map_err(|e| e.to_string())
    };
    match result {
        Ok(m) => Some(m),
        Err(e) => {
            log::warn!("解析用户 module {:?} 失败: {e}", path);
            None
        }
    }
}

/// Checks if a stage id matches a semantic name (e.g., "dev" matches "4-dev").
fn stage_id_matches(id: &str, semantic: &str) -> bool {
    let lower = id.to_lowercase();
    lower == semantic || lower.ends_with(&format!("-{}", semantic))
}

/// Validates semantic rules S1-S5 on a preset's stage configuration.
/// Returns a list of warning/error messages.
/// `project_type` is needed for S5 (ui-design + backend/cli check).
#[allow(dead_code)]
pub fn validate_preset_semantic(preset: &WorkflowPreset, project_type: &str) -> Vec<String> {
    match resolve_stages_for_preset(preset, project_type) {
        Ok(stage_ids) => validate_effective_stage_semantic(preset, project_type, &stage_ids),
        Err(e) => vec![format!("无法解析流程阶段: {e}")],
    }
}

pub fn validate_effective_stage_semantic(
    preset: &WorkflowPreset,
    project_type: &str,
    effective_stage_ids: &[String],
) -> Vec<String> {
    let mut issues = Vec::new();
    let stage_ids: HashSet<&str> = effective_stage_ids.iter().map(|s| s.as_str()).collect();

    let has_stage =
        |semantic: &str| -> bool { stage_ids.iter().any(|id| stage_id_matches(id, semantic)) };

    // S1: dev enabled → task must be enabled (R2.3: no TASK → no code)
    if has_stage("dev") && !has_stage("task") {
        issues.push("S1: dev 阶段启用但 task 阶段未启用（R2.3：没有 TASK 不能写代码）".into());
    }

    // S2: test enabled → requirement must be enabled (R5.1: tests derive from AC)
    if has_stage("test") && !has_stage("requirement") {
        issues.push("S2: test 阶段启用但 requirement 阶段未启用（R5.1：测试从 AC 派生）".into());
    }

    // S3: review enabled → test should be enabled (R2.5: REVIEW needs TEST)
    if has_stage("review") && !has_stage("test") {
        issues.push("S3: review 阶段启用但 test 阶段未启用（R2.5：REVIEW 需要 TEST 产出）".into());
    }

    // S4: integration enabled → review should be enabled (R2.6: UAT needs REVIEW)
    if has_stage("integration") && !has_stage("review") {
        issues.push("S4: integration 阶段启用但 review 阶段未启用（R2.6：UAT 需要 REVIEW）".into());
    }

    // S5: ui-design stage in backend/cli path without skipWhen is a configuration error
    if project_type == "backend" || project_type == "cli" {
        for stage_id in effective_stage_ids {
            if stage_id_matches(stage_id, "ui-design") {
                let has_skip = preset
                    .stages
                    .iter()
                    .find(|s| s.id == *stage_id)
                    .and_then(|s| s.skip_when.as_ref())
                    .map_or(false, |sw| {
                        sw.project_type.iter().any(|t| t == project_type)
                    });
                if !has_skip {
                    issues.push(format!(
                        "S5: {stage_id} 阶段出现在 {project_type} 路径中但没有 skipWhen 覆盖（UI 设计不适用于 {project_type} 项目）"
                    ));
                }
            }
        }
    }

    issues
}

pub fn list_workflow_presets(
    project_path: Option<&str>,
) -> Result<Vec<WorkflowPresetSummary>, AppError> {
    Ok(load_all_presets(project_path)?
        .into_iter()
        .map(|p| WorkflowPresetSummary {
            id: p.id.clone(),
            name: p.name.clone(),
            name_zh: p.name_zh.clone(),
            description: p.description.clone(),
            r3_tier: p.r3_tier.clone(),
            module_count: p.modules.len(),
            stage_count: p.stages.len(),
        })
        .collect())
}

pub fn get_workflow_preset(
    id: &str,
    project_path: Option<&str>,
) -> Result<WorkflowPreset, AppError> {
    load_all_presets(project_path)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| AppError::InvalidInput(format!("Workflow preset 不存在: {id}")))
}

pub fn resolve_stages_for_preset(
    preset: &WorkflowPreset,
    project_type: &str,
) -> Result<Vec<String>, AppError> {
    let path_key = match project_type {
        "frontend" | "backend" | "cli" | "mvp" => project_type,
        other => {
            log::debug!("未知 projectType {other}，回退 backend");
            "backend"
        }
    };

    let raw = match path_key {
        "frontend" => &preset.paths.frontend,
        "backend" => &preset.paths.backend,
        "cli" => &preset.paths.cli,
        "mvp" => &preset.paths.mvp,
        _ => &preset.paths.backend,
    };

    if raw.is_empty() {
        return Err(AppError::Message(format!(
            "Preset {} 未定义 {} 路径",
            preset.id, path_key
        )));
    }

    let stage_map: HashMap<&str, &WorkflowStage> =
        preset.stages.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut resolved = Vec::new();
    for stage_id in raw {
        let Some(stage) = stage_map.get(stage_id.as_str()) else {
            continue;
        };
        if stage_should_skip(stage, project_type) {
            continue;
        }
        resolved.push(stage_id.clone());
    }
    Ok(resolved)
}

fn stage_should_skip(stage: &WorkflowStage, project_type: &str) -> bool {
    let Some(skip) = &stage.skip_when else {
        return false;
    };
    skip.project_type.iter().any(|t| t == project_type)
}

pub fn validate_change_id(change_id: &str) -> Result<(), AppError> {
    let trimmed = change_id.trim();
    if trimmed != change_id {
        return Err(AppError::InvalidInput(
            "Change ID 不能包含首尾空白字符".into(),
        ));
    }
    if !(3..=80).contains(&change_id.len()) {
        return Err(AppError::InvalidInput(
            "Change ID 长度必须在 3 到 80 个字符之间".into(),
        ));
    }
    if change_id == "." || change_id == ".." {
        return Err(AppError::InvalidInput("Change ID 不能使用 . 或 ..".into()));
    }
    if !change_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(AppError::InvalidInput(
            "Change ID 只能包含英文字母、数字、点、下划线和短横线".into(),
        ));
    }
    Ok(())
}

pub fn sanitize_change_id_seed(seed: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for c in seed.trim().chars() {
        let next = if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            c.to_ascii_lowercase()
        } else {
            '-'
        };
        if next == '-' {
            if last_dash {
                continue;
            }
            last_dash = true;
        } else {
            last_dash = false;
        }
        out.push(next);
    }

    let trimmed = out
        .trim_matches(|c| matches!(c, '.' | '_' | '-'))
        .to_string();
    let mut safe: String = if trimmed.is_empty() {
        "change".into()
    } else {
        trimmed.chars().take(60).collect()
    };
    while safe.len() < 3 {
        safe.push('x');
    }
    safe
}

fn empty_install_audit() -> InstallAuditSummary {
    InstallAuditSummary {
        files_scanned: 0,
        total_findings: 0,
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        blocked: false,
        findings: Vec::new(),
    }
}

fn opensunstar_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(OPENSUNSTAR_DIR)
}

fn profile_path(project_path: &str) -> PathBuf {
    opensunstar_dir(project_path).join(PROFILE_FILENAME)
}

fn specs_root(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(SPECS_DIR)
}

fn read_saved_profile(project_path: &str) -> Option<WorkflowProfile> {
    let path = profile_path(project_path);
    if !path.is_file() {
        return None;
    }
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn parse_active_change_from_state(project_path: &str) -> Option<String> {
    let state_path = PathBuf::from(project_path).join(STATE_FILENAME);
    let raw = fs::read_to_string(state_path).ok()?;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("active_change:") || trimmed.starts_with("activeChange:") {
            let value = trimmed.split(':').nth(1)?.trim();
            if !value.is_empty() && value != "null" {
                return Some(value.to_string());
            }
        }
        if trimmed.starts_with("change-id:") || trimmed.starts_with("changeId:") {
            let value = trimmed.split(':').nth(1)?.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn artifact_exists_nonempty(path: &Path) -> (bool, bool) {
    if !path.is_file() {
        return (false, false);
    }
    match fs::metadata(path) {
        Ok(meta) => {
            let non_empty = meta.len() > 0;
            (true, non_empty)
        }
        Err(_) => (false, false),
    }
}

fn collect_required_artifacts_for_preset(preset: &WorkflowPreset) -> Vec<WorkflowArtifactSpec> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for stage in &preset.stages {
        for artifact in &stage.artifacts {
            if artifact.optional {
                continue;
            }
            let key = format!("{}:{}", artifact.scope, artifact.file);
            if seen.insert(key) {
                out.push(artifact.clone());
            }
        }
    }
    out
}

fn artifact_path(
    project_path: &str,
    change_id: &str,
    artifact: &WorkflowArtifactSpec,
) -> (PathBuf, String) {
    let root = PathBuf::from(project_path);
    let (path, relative) = match artifact.scope.as_str() {
        "specs-root" => {
            let p = specs_root(project_path).join(&artifact.file);
            (p, format!(".specs/{}", artifact.file))
        }
        "project-root" => {
            let p = root.join(&artifact.file);
            (p, artifact.file.clone())
        }
        _ => {
            let p = specs_root(project_path)
                .join(change_id)
                .join(&artifact.file);
            (p, format!(".specs/{}/{}", change_id, artifact.file))
        }
    };
    let _ = root;
    (path, relative)
}

fn parse_task_summary(task_path: &Path) -> Option<TaskSummary> {
    let raw = fs::read_to_string(task_path).ok()?;
    let mut summary = TaskSummary {
        total: 0,
        pending: 0,
        in_progress: 0,
        done: 0,
        blocked: 0,
    };

    for line in raw.lines() {
        let line = line.trim();
        if !line.starts_with("<task") {
            continue;
        }
        summary.total += 1;
        let status = extract_xml_attr(line, "status").unwrap_or_else(|| "pending".into());
        match status.as_str() {
            "done" => summary.done += 1,
            "in_progress" | "in-progress" => summary.in_progress += 1,
            "blocked" => summary.blocked += 1,
            _ => summary.pending += 1,
        }
    }

    if summary.total == 0 {
        None
    } else {
        Some(summary)
    }
}

fn extract_xml_attr(line: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

pub fn scan_project_specs_workflow(
    project_path: &str,
    preset_id: Option<&str>,
    project_type: Option<&str>,
) -> Result<SpecsWorkflowIndex, AppError> {
    let workspace_exists = project_workspace_exists(project_path);
    let has_flow_kit = workspace_exists && PathBuf::from(project_path).join(FLOW_KIT_GO).is_file();
    let has_flow_config = opensunstar_dir(project_path).join(FLOW_CONFIG_FILENAME).is_file();
    let specs_dir = specs_root(project_path);
    let has_specs_dir = specs_dir.is_dir();

    let saved_profile = read_saved_profile(project_path);
    let active_change_id = saved_profile
        .as_ref()
        .and_then(|p| p.active_change_id.clone())
        .or_else(|| parse_active_change_from_state(project_path));

    let preset = if let Some(id) = preset_id {
        get_workflow_preset(id, Some(project_path))?
    } else if let Some(ref profile) = saved_profile {
        get_workflow_preset(&profile.preset_id, Some(project_path))?
    } else {
        get_workflow_preset("standard", Some(project_path))?
    };

    let _ptype = project_type
        .or_else(|| saved_profile.as_ref().map(|p| p.project_type.as_str()))
        .unwrap_or("backend");

    let required = collect_required_artifacts_for_preset(&preset);
    let mut changes = Vec::new();

    if has_specs_dir {
        if let Ok(entries) = fs::read_dir(&specs_dir) {
            for entry in entries.flatten() {
                let file_type = match entry.file_type() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                if !file_type.is_dir() {
                    continue;
                }
                let change_id = entry.file_name().to_string_lossy().to_string();
                if RESERVED_SPECS_ENTRIES.contains(&change_id.as_str()) {
                    continue;
                }

                let mut artifacts = Vec::new();
                let mut required_count = 0u8;
                let mut satisfied = 0u8;

                for spec in &required {
                    if spec.scope != "change" {
                        continue;
                    }
                    let (path, relative) = artifact_path(project_path, &change_id, spec);
                    let (exists, non_empty) = artifact_exists_nonempty(&path);
                    if !spec.optional {
                        required_count += 1;
                        if exists && non_empty {
                            satisfied += 1;
                        }
                    }
                    artifacts.push(ArtifactStatus {
                        file: spec.file.clone(),
                        relative_path: relative,
                        exists,
                        non_empty,
                        optional: spec.optional,
                    });
                }

                let task_path = specs_dir.join(&change_id).join("TASK.md");
                let task_summary = parse_task_summary(&task_path);

                let artifact_completeness = if required_count == 0 {
                    100
                } else {
                    ((satisfied as u16 * 100) / required_count as u16) as u8
                };

                changes.push(SpecsChangeIndex {
                    change_id,
                    artifact_completeness,
                    artifacts,
                    task_summary,
                });
            }
        }
    }

    changes.sort_by(|a, b| a.change_id.cmp(&b.change_id));

    Ok(SpecsWorkflowIndex {
        project_path: project_path.to_string(),
        workspace_exists,
        has_flow_kit,
        has_flow_config,
        has_specs_dir,
        active_change_id,
        saved_profile,
        changes,
    })
}

pub fn validate_workflow_stage_gate(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    change_id: &str,
    target_stage: &str,
) -> Result<StageGateResult, AppError> {
    if !project_workspace_exists(project_path) {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }
    validate_change_id(change_id)?;

    let preset = get_workflow_preset(preset_id, Some(project_path))?;
    let resolved = resolve_stages_for_preset(&preset, project_type)?;

    if !resolved.iter().any(|s| s == target_stage) {
        return Ok(StageGateResult {
            allowed: true,
            target_stage: target_stage.to_string(),
            change_id: change_id.to_string(),
            missing_artifacts: Vec::new(),
            satisfied_artifacts: Vec::new(),
            warnings: vec![format!(
                "阶段 {target_stage} 不在 preset {preset_id} / {project_type} 解析路径中，跳过门禁"
            )],
        });
    }

    let stage_map: HashMap<&str, &WorkflowStage> =
        preset.stages.iter().map(|s| (s.id.as_str(), s)).collect();

    let target_idx = resolved
        .iter()
        .position(|s| s == target_stage)
        .ok_or_else(|| AppError::InvalidInput(format!("无效 targetStage: {target_stage}")))?;

    let mut missing = Vec::new();
    let mut satisfied = Vec::new();
    let mut warnings = Vec::new();

    for stage_id in resolved.iter().take(target_idx + 1) {
        let Some(stage) = stage_map.get(stage_id.as_str()) else {
            continue;
        };
        if stage_should_skip(stage, project_type) {
            continue;
        }
        for artifact in &stage.artifacts {
            if artifact.optional {
                continue;
            }
            let (path, relative) = match artifact.scope.as_str() {
                "specs-root" => artifact_path(project_path, change_id, artifact),
                _ => artifact_path(project_path, change_id, artifact),
            };
            let (exists, non_empty) = artifact_exists_nonempty(&path);
            if exists && non_empty {
                if !satisfied.contains(&relative) {
                    satisfied.push(relative);
                }
            } else if !missing.contains(&relative) {
                missing.push(relative);
            }
        }
    }

    if stage_id_requires_change_folder(target_stage)
        && !specs_root(project_path).join(change_id).is_dir()
    {
        let rel = format!(".specs/{change_id}/");
        if !missing.contains(&rel) {
            missing.push(rel);
        }
    }

    // A resolved project design system is implementation input, not documentation only.
    // Require the recipe-generated snapshot for change-scoped stages so Agents and Specs
    // consume the same locked package, prototype, responsive, and accessibility context.
    if std::path::Path::new(project_path)
        .join(".opensunstar/design-system.json")
        .is_file()
        && stage_id_requires_change_folder(target_stage)
    {
        let rel = format!(".specs/{change_id}/design-context.md");
        let (exists, non_empty) =
            artifact_exists_nonempty(&std::path::Path::new(project_path).join(&rel));
        if exists && non_empty {
            if !satisfied.contains(&rel) {
                satisfied.push(rel);
            }
        } else if !missing.contains(&rel) {
            missing.push(rel);
        }
    }

    let allowed = missing.is_empty();
    if !allowed {
        warnings.push("规则 R2.7：目标阶段缺少上游工件，应回到对应阶段补齐后再继续。".into());
    }

    Ok(StageGateResult {
        allowed,
        target_stage: target_stage.to_string(),
        change_id: change_id.to_string(),
        missing_artifacts: missing,
        satisfied_artifacts: satisfied,
        warnings,
    })
}

fn stage_id_requires_change_folder(stage_id: &str) -> bool {
    !matches!(
        stage_id,
        "I-intel-scan" | "A-architect" | "A-evolve" | "M-health" | "L-restyle"
    )
}

fn build_project_workflow_profile(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    active_change_id: Option<&str>,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
    enforce_semantics: bool,
) -> Result<WorkflowProfile, AppError> {
    if !project_workspace_exists(project_path) {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }
    if let Some(change_id) = active_change_id {
        validate_change_id(change_id)?;
    }

    let preset = get_workflow_preset(preset_id, Some(project_path))?;
    let mut resolved_stages = resolve_stages_for_preset(&preset, project_type)?;

    if let Some(disabled) = disabled_stages {
        let disabled_set: HashSet<&str> = disabled.iter().map(|s| s.as_str()).collect();
        resolved_stages.retain(|s| !disabled_set.contains(s.as_str()));
    }

    let semantic_issues =
        validate_effective_stage_semantic(&preset, project_type, &resolved_stages);
    for issue in &semantic_issues {
        log::warn!("Preset 语义规则: {issue}");
    }
    if enforce_semantics && !semantic_issues.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "严格流程校验失败：{}",
            semantic_issues.join("；")
        )));
    }

    let modules = match selected_modules {
        Some(m) => m.to_vec(),
        None => preset.modules.clone(),
    };

    Ok(WorkflowProfile {
        schema_version: 1,
        preset_id: preset_id.to_string(),
        project_type: project_type.to_string(),
        modules,
        resolved_stages,
        active_change_id: active_change_id.map(str::to_string),
        exported_at: Utc::now().to_rfc3339(),
        opensunstar_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        semantic_warnings: semantic_issues,
    })
}

fn build_flow_config(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
    enforce_semantics: bool,
) -> Result<FlowConfig, AppError> {
    if !project_workspace_exists(project_path) {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }

    let preset = get_workflow_preset(preset_id, Some(project_path))?;
    let mut resolved_stage_ids = resolve_stages_for_preset(&preset, project_type)?;

    let disabled_set: HashSet<&str> = disabled_stages
        .unwrap_or(&[])
        .iter()
        .map(|s| s.as_str())
        .collect();

    let stage_map: HashMap<&str, &WorkflowStage> =
        preset.stages.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut fc_stages = Vec::new();
    for sid in &resolved_stage_ids {
        let enabled = !disabled_set.contains(sid.as_str());
        let stage = match stage_map.get(sid.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let gates: Vec<FlowConfigGate> = if !stage.artifacts.is_empty() {
            let artifact_files: Vec<String> = stage
                .artifacts
                .iter()
                .filter(|a| !a.optional)
                .map(|a| a.file.clone())
                .collect();
            if artifact_files.is_empty() {
                vec![]
            } else {
                vec![FlowConfigGate {
                    gate_type: "artifact-exists".to_string(),
                    artifacts: artifact_files,
                }]
            }
        } else {
            vec![]
        };

        fc_stages.push(FlowConfigStage {
            id: sid.clone(),
            enabled,
            depends_on: stage.depends_on.clone(),
            gates,
        });
    }

    resolved_stage_ids.retain(|s| !disabled_set.contains(s.as_str()));

    let modules = match selected_modules {
        Some(m) => m.to_vec(),
        None => preset.modules.clone(),
    };

    let semantic_issues =
        validate_effective_stage_semantic(&preset, project_type, &resolved_stage_ids);
    for issue in &semantic_issues {
        log::warn!("FlowConfig 语义规则: {issue}");
    }
    if enforce_semantics && !semantic_issues.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "严格流程校验失败：{}",
            semantic_issues.join("；")
        )));
    }

    Ok(FlowConfig {
        schema_version: 1,
        preset_id: preset_id.to_string(),
        project_type: project_type.to_string(),
        modules,
        stages: fc_stages,
        rules: FlowConfigRules {
            max_auto_retry: R96_MAX_AUTO_RETRY,
            role_separation: R96_ROLE_SEPARATION,
            require_diff_boundary: R96_REQUIRE_DIFF_BOUNDARY,
        },
        review_policy: default_review_policy(),
        semantic_warnings: semantic_issues,
    })
}

fn default_review_policy() -> FlowConfigReviewPolicy {
    let lenses_4r = REVIEW_LENSES_4R.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    FlowConfigReviewPolicy {
        mode: "risk-aware".to_string(),
        trivial: FlowConfigReviewTier {
            enabled: true,
            lenses: vec![],
            max_lenses: Some(0),
            default_lens: None,
            paths: vec![],
            changed_lines_threshold: None,
        },
        standard: FlowConfigReviewTier {
            enabled: true,
            lenses: vec![
                "review-readability".to_string(),
                "review-reliability".to_string(),
                "review-resilience".to_string(),
                "review-risk".to_string(),
            ],
            max_lenses: Some(1),
            default_lens: Some("review-readability".to_string()),
            paths: vec![],
            changed_lines_threshold: None,
        },
        hot_path: FlowConfigReviewTier {
            enabled: true,
            lenses: lenses_4r.clone(),
            max_lenses: Some(4),
            default_lens: None,
            paths: vec![
                "**/auth/**".to_string(),
                "**/security/**".to_string(),
                "**/payments/**".to_string(),
                "**/permission/**".to_string(),
                "**/permissions/**".to_string(),
                "**/proxy/**".to_string(),
                "**/sync/**".to_string(),
            ],
            changed_lines_threshold: None,
        },
        large_diff: FlowConfigReviewTier {
            enabled: true,
            lenses: lenses_4r,
            max_lenses: Some(4),
            default_lens: None,
            paths: vec![],
            changed_lines_threshold: Some(400),
        },
    }
}

fn flow_write_file_entry(path: PathBuf, rel_path: &str, new_content: String) -> InstallFileEntry {
    let existing_content = if path.is_file() {
        fs::read_to_string(&path).ok()
    } else {
        None
    };
    InstallFileEntry {
        path: rel_path.to_string(),
        status: if existing_content.is_some() {
            "overwrite".into()
        } else {
            "create".into()
        },
        new_content: Some(new_content),
        existing_content,
    }
}

fn flow_create_file_entry(path: PathBuf, rel_path: &str, new_content: String) -> InstallFileEntry {
    let existing_content = if path.is_file() {
        fs::read_to_string(&path).ok()
    } else {
        None
    };
    InstallFileEntry {
        path: rel_path.to_string(),
        status: if existing_content.is_some() {
            "skip".into()
        } else {
            "create".into()
        },
        new_content: if existing_content.is_some() { None } else { Some(new_content) },
        existing_content,
    }
}

fn ci_workflow_path(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(CI_WORKFLOW_REL)
}

fn generate_ci_workflow(project_type: &str, target_stage: &str) -> String {
    format!(
        r#"name: OpenSunstar Flow Gate

on:
  pull_request:
  push:
    branches: [ main, master ]

jobs:
  flow-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - name: Install OpenSunstar CLI
        run: npm install -g opensunstar-os
      - name: Validate OpenSunstar workflow artifacts
        shell: bash
        run: |
          set -euo pipefail
          CHANGE_ID=""
          if [ -f STATE.md ]; then
            CHANGE_ID="$(awk -F: '/^change_id:/ {{ gsub(/^[ \t]+|[ \t]+$/, "", $2); print $2; exit }}' STATE.md)"
          fi
          if [ -z "$CHANGE_ID" ] && [ -d .specs ]; then
            CHANGE_ID="$(find .specs -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort | head -n 1)"
          fi
          if [ -z "$CHANGE_ID" ]; then
            echo "No change id found. Create STATE.md or .specs/<change-id>/ first."
            exit 1
          fi
          os flow validate --project-path . --project-type {project_type} --change-id "$CHANGE_ID" --target-stage {target_stage} --strict --json
"#
    )
}

pub fn preview_project_workflow_profile_export(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    active_change_id: Option<&str>,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
) -> Result<FlowWritePlan, AppError> {
    let profile = build_project_workflow_profile(
        project_path,
        preset_id,
        project_type,
        active_change_id,
        selected_modules,
        disabled_stages,
        false,
    )?;
    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| AppError::Message(format!("序列化 workflow profile 失败: {e}")))?;
    Ok(FlowWritePlan {
        files: vec![flow_write_file_entry(
            profile_path(project_path),
            ".opensunstar/workflow.profile.json",
            json,
        )],
        audit: empty_install_audit(),
        semantic_warnings: profile.semantic_warnings,
    })
}

pub fn preview_flow_config_export(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
) -> Result<FlowWritePlan, AppError> {
    let config = build_flow_config(
        project_path,
        preset_id,
        project_type,
        selected_modules,
        disabled_stages,
        false,
    )?;
    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| AppError::Message(format!("序列化 flow-config.yaml 失败: {e}")))?;
    let target_stage = config
        .stages
        .iter()
        .rev()
        .find(|s| s.enabled)
        .map(|s| s.id.as_str())
        .unwrap_or("6-review");
    Ok(FlowWritePlan {
        files: vec![
            flow_write_file_entry(
                opensunstar_dir(project_path).join(FLOW_CONFIG_FILENAME),
                ".opensunstar/flow-config.yaml",
                yaml,
            ),
            flow_create_file_entry(
                ci_workflow_path(project_path),
                CI_WORKFLOW_REL,
                generate_ci_workflow(project_type, target_stage),
            ),
        ],
        audit: empty_install_audit(),
        semantic_warnings: config.semantic_warnings,
    })
}

pub fn export_project_workflow_profile(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    active_change_id: Option<&str>,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
) -> Result<WorkflowProfile, AppError> {
    export_project_workflow_profile_with_semantic_enforcement(
        project_path,
        preset_id,
        project_type,
        active_change_id,
        selected_modules,
        disabled_stages,
        false,
    )
}

pub fn export_project_workflow_profile_strict(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    active_change_id: Option<&str>,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
) -> Result<WorkflowProfile, AppError> {
    export_project_workflow_profile_with_semantic_enforcement(
        project_path,
        preset_id,
        project_type,
        active_change_id,
        selected_modules,
        disabled_stages,
        true,
    )
}

fn export_project_workflow_profile_with_semantic_enforcement(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    active_change_id: Option<&str>,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
    enforce_semantics: bool,
) -> Result<WorkflowProfile, AppError> {
    let profile = build_project_workflow_profile(
        project_path,
        preset_id,
        project_type,
        active_change_id,
        selected_modules,
        disabled_stages,
        enforce_semantics,
    )?;
    let out_path = opensunstar_dir(project_path).join(PROFILE_FILENAME);
    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| AppError::Message(format!("序列化 workflow profile 失败: {e}")))?;
    let receipt = execute_text_write_plan(
        project_path,
        "workflow-profile-export",
        vec![PlannedTextWrite::replace(
            "profile",
            "保存项目流程",
            out_path.clone(),
            json,
        )],
        false,
        vec![verification(
            "profile-exists",
            "workflow.profile.json 已生成",
            true,
            Some(out_path.to_string_lossy().to_string()),
        )],
    )?;
    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "profile_export",
            "presetId": preset_id,
            "projectType": project_type,
            "activeChangeId": active_change_id,
            "resolvedStageCount": profile.resolved_stages.len(),
            "semanticEnforcement": enforce_semantics,
            "receiptStepCount": receipt.steps.len(),
            "rollbackSnapshots": receipt.steps.iter().filter(|s| s.snapshot_path.is_some()).count(),
        }),
    )?;
    append_post_apply_verification_log(project_path, "workflow-profile-export", &receipt)?;
    Ok(profile)
}

/// Export flow-config.yaml from profile data, with R9.6 safety valve enforcement.
pub fn export_flow_config(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
) -> Result<FlowConfig, AppError> {
    export_flow_config_with_semantic_enforcement(
        project_path,
        preset_id,
        project_type,
        selected_modules,
        disabled_stages,
        false,
    )
}

pub fn export_flow_config_strict(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
) -> Result<FlowConfig, AppError> {
    export_flow_config_with_semantic_enforcement(
        project_path,
        preset_id,
        project_type,
        selected_modules,
        disabled_stages,
        true,
    )
}

fn export_flow_config_with_semantic_enforcement(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    selected_modules: Option<&[String]>,
    disabled_stages: Option<&[String]>,
    enforce_semantics: bool,
) -> Result<FlowConfig, AppError> {
    let config = build_flow_config(
        project_path,
        preset_id,
        project_type,
        selected_modules,
        disabled_stages,
        enforce_semantics,
    )?;
    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| AppError::Message(format!("序列化 flow-config.yaml 失败: {e}")))?;
    let out_path = opensunstar_dir(project_path).join(FLOW_CONFIG_FILENAME);
    let target_stage = config
        .stages
        .iter()
        .rev()
        .find(|s| s.enabled)
        .map(|s| s.id.as_str())
        .unwrap_or("6-review");
    let ci_path = ci_workflow_path(project_path);
    let receipt = execute_text_write_plan(
        project_path,
        "flow-config-export",
        vec![
            PlannedTextWrite::replace(
                "flow-config",
                "导出门禁配置",
                out_path.clone(),
                yaml,
            ),
            PlannedTextWrite::create_if_missing(
                "ci-workflow",
                "接入 CI 门禁模板",
                ci_path.clone(),
                generate_ci_workflow(project_type, target_stage),
            ),
        ],
        false,
        vec![
            verification(
                "flow-config-exists",
                "flow-config.yaml 已生成",
                true,
                Some(out_path.to_string_lossy().to_string()),
            ),
            verification(
                "ci-workflow-present",
                "CI workflow 已存在或已创建",
                true,
                Some(ci_path.to_string_lossy().to_string()),
            ),
        ],
    )?;
    let ci_workflow_created = receipt
        .steps
        .iter()
        .any(|s| s.id == "ci-workflow" && s.status == OrchestrationStepStatus::Applied);
    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "flow_config_export",
            "presetId": preset_id,
            "projectType": project_type,
            "stageCount": config.stages.len(),
            "r96Enforced": true,
            "semanticEnforcement": enforce_semantics,
            "ciWorkflow": CI_WORKFLOW_REL,
            "ciWorkflowCreated": ci_workflow_created,
            "receiptStepCount": receipt.steps.len(),
            "rollbackSnapshots": receipt.steps.iter().filter(|s| s.snapshot_path.is_some()).count(),
        }),
    )?;
    append_post_apply_verification_log(project_path, "flow-config-export", &receipt)?;
    Ok(config)
}

fn append_post_apply_verification_log(
    project_path: &str,
    operation: &str,
    receipt: &OrchestrationReceipt,
) -> Result<(), AppError> {
    let total = receipt.verifications.len();
    let passed = receipt.verifications.iter().filter(|v| v.passed).count();
    let failed = total.saturating_sub(passed);
    let failed_items = receipt
        .verifications
        .iter()
        .filter(|v| !v.passed)
        .map(|v| {
            serde_json::json!({
                "id": v.id,
                "label": v.label,
                "detail": v.detail,
            })
        })
        .collect::<Vec<_>>();
    append_orchestration_log(
        project_path,
        serde_json::json!({
            "event": "post_apply_verification",
            "operation": operation,
            "passed": passed,
            "failed": failed,
            "total": total,
            "failedItems": failed_items,
        }),
    )
}

pub fn append_orchestration_log(
    project_path: &str,
    mut payload: serde_json::Value,
) -> Result<(), AppError> {
    let obj = payload
        .as_object_mut()
        .ok_or_else(|| AppError::Message("orchestration log payload 必须是 object".into()))?;
    obj.insert(
        "ts".into(),
        serde_json::Value::String(Utc::now().to_rfc3339()),
    );

    let log_path = opensunstar_dir(project_path).join(ORCH_LOG_FILENAME);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let line = serde_json::to_string(&payload)
        .map_err(|e| AppError::Message(format!("序列化 orchestration log 失败: {e}")))?;
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| AppError::io(&log_path, e))?;
    writeln!(file, "{line}").map_err(|e| AppError::io(&log_path, e))?;
    if let Err(e) = crate::services::project_config_sync::sync_orchestration_agent_context(project_path) {
        log::warn!("刷新 OpenSunstar Agent 上下文失败: {e}");
    }
    Ok(())
}

pub fn read_orchestration_log(
    project_path: &str,
    limit: Option<usize>,
) -> Result<Vec<OrchestrationLogEntry>, AppError> {
    let log_path = opensunstar_dir(project_path).join(ORCH_LOG_FILENAME);
    if !log_path.is_file() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(&log_path).map_err(|e| AppError::io(&log_path, e))?;
    let take = limit.unwrap_or(20).clamp(1, 100);
    let mut entries = Vec::new();
    for line in text.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let event = value
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let ts = value.get("ts").and_then(Value::as_str).map(str::to_string);
        entries.push(OrchestrationLogEntry {
            summary: orchestration_event_summary(&value, &event),
            event,
            ts,
            payload: value,
        });
        if entries.len() >= take {
            break;
        }
    }
    Ok(entries)
}

fn orchestration_event_summary(value: &Value, event: &str) -> String {
    let str_field = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or("-");
    match event {
        "profile_export" => format!(
            "保存项目流程：{} / {}，阶段 {} 个",
            str_field("presetId"),
            str_field("projectType"),
            value
                .get("resolvedStageCount")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into())
        ),
        "flow_config_export" => format!(
            "生成自动检查配置：{} / {}，阶段 {} 个",
            str_field("presetId"),
            str_field("projectType"),
            value
                .get("stageCount")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into())
        ),
        "stage_gate" => format!(
            "检查是否能进入下一步：{} → {}，{}",
            str_field("changeId"),
            str_field("targetStage"),
            if value.get("allowed").and_then(Value::as_bool).unwrap_or(false) {
                "通过"
            } else {
                "未通过"
            }
        ),
        "recipe_export" => format!("保存改动模板：{}", str_field("name")),
        "recipe_install" => format!(
            "生成改动模板：{} / {}",
            str_field("name"),
            str_field("changeId")
        ),
        "design_contract_export" => format!("导出 UI 设计约束：{}", str_field("name")),
        "design_contract_install" => format!("安装 UI 设计约束：{}", str_field("name")),
        "orchestration_rollback" => format!(
            "恢复最近一次编排：{} 个文件操作",
            value
                .get("stepCount")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into())
        ),
        "post_apply_verification" => format!(
            "写后验证：{}/{} 通过{}",
            value
                .get("passed")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
            value
                .get("total")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
            match value.get("failed").and_then(Value::as_u64).unwrap_or(0) {
                0 => "".to_string(),
                n => format!("，{} 项失败", n),
            }
        ),
        _ => event.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opensunstar-flow-orch-{}", uuid_simple()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn uuid_simple() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    #[test]
    fn list_presets_includes_standard() {
        let presets = list_workflow_presets(None).unwrap();
        assert!(presets.iter().any(|p| p.id == "standard"));
    }

    #[test]
    fn backend_standard_skips_ui_design_stage() {
        let preset = get_workflow_preset("standard", None).unwrap();
        let stages = resolve_stages_for_preset(&preset, "backend").unwrap();
        assert!(!stages.iter().any(|s| s == "2a-ui-design"));
        assert!(stages.iter().any(|s| s == "3-task"));
    }

    #[test]
    fn stage_gate_blocks_task_without_design() {
        let root = temp_project();
        let change_id = "feat-auth";
        let change_dir = root.join(".specs").join(change_id);
        fs::create_dir_all(&change_dir).unwrap();
        fs::write(change_dir.join("CHANGE.md"), "# change").unwrap();
        fs::write(change_dir.join("REQUIREMENT.md"), "# req").unwrap();

        let result = validate_workflow_stage_gate(
            root.to_str().unwrap(),
            "standard",
            "backend",
            change_id,
            "3-task",
        )
        .unwrap();

        assert!(!result.allowed);
        assert!(result
            .missing_artifacts
            .iter()
            .any(|p| p.contains("DESIGN.md")));
    }

    #[test]
    fn scan_empty_specs_returns_no_changes() {
        let root = temp_project();
        fs::create_dir_all(root.join(".specs")).unwrap();
        let index = scan_project_specs_workflow(root.to_str().unwrap(), None, None).unwrap();
        assert!(index.changes.is_empty());
        assert!(index.has_specs_dir);
    }

    #[test]
    fn export_writes_profile_json() {
        let root = temp_project();
        let profile = export_project_workflow_profile(
            root.to_str().unwrap(),
            "mvp",
            "backend",
            Some("demo-change"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(profile.preset_id, "mvp");
        let path = root.join(".opensunstar").join("workflow.profile.json");
        assert!(path.is_file());
        let log = root.join(".opensunstar").join("orchestration.log.jsonl");
        assert!(log.is_file());
    }

    #[test]
    fn change_id_validation_rejects_path_traversal() {
        assert!(validate_change_id("feat-auth").is_ok());
        assert!(validate_change_id("feat.auth_01").is_ok());
        assert!(validate_change_id("../secret").is_err());
        assert!(validate_change_id("nested/path").is_err());
        assert!(validate_change_id(" bad").is_err());
        assert!(validate_change_id("..").is_err());
    }

    #[test]
    fn profile_preview_marks_existing_file_as_overwrite() {
        let root = temp_project();
        let first = preview_project_workflow_profile_export(
            root.to_str().unwrap(),
            "mvp",
            "backend",
            Some("demo-change"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(first.files[0].status, "create");

        export_project_workflow_profile(
            root.to_str().unwrap(),
            "mvp",
            "backend",
            Some("demo-change"),
            None,
            None,
        )
        .unwrap();

        let second = preview_project_workflow_profile_export(
            root.to_str().unwrap(),
            "mvp",
            "backend",
            Some("demo-change"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(second.files[0].status, "overwrite");
        assert!(second.files[0].existing_content.is_some());
        assert!(second.files[0].new_content.is_some());
    }

    #[test]
    fn user_preset_loaded_from_project_dir() {
        let root = temp_project();
        let presets_dir = root.join(".opensunstar").join("workflow.presets");
        fs::create_dir_all(&presets_dir).unwrap();
        let custom_preset = r#"{
            "id": "custom-test",
            "name": "Custom Test",
            "description": "A user-defined preset",
            "modules": ["openspec-change"],
            "stages": [
                {"id": "6-review", "name": "Review", "artifacts": [{"file": "REVIEW.md", "scope": "change", "optional": false}]}
            ],
            "paths": {"frontend": ["6-review"], "backend": ["6-review"], "cli": ["6-review"], "mvp": ["6-review"]}
        }"#;
        fs::write(presets_dir.join("custom-test.json"), custom_preset).unwrap();

        let presets = list_workflow_presets(Some(root.to_str().unwrap())).unwrap();
        assert!(presets.iter().any(|p| p.id == "custom-test"));
        // builtins still present
        assert!(presets.iter().any(|p| p.id == "standard"));
    }

    #[test]
    fn user_preset_yaml_loaded() {
        let root = temp_project();
        let presets_dir = root.join(".opensunstar").join("workflow.presets");
        fs::create_dir_all(&presets_dir).unwrap();
        let yaml_content = r#"id: yaml-test
name: YAML Test
description: A YAML preset
modules:
  - openspec-change
stages:
  - id: "6-review"
    name: Review
    artifacts:
      - file: REVIEW.md
        scope: change
        optional: false
paths:
  frontend: ["6-review"]
  backend: ["6-review"]
  cli: ["6-review"]
  mvp: ["6-review"]
"#;
        fs::write(presets_dir.join("yaml-test.yaml"), yaml_content).unwrap();

        let preset = get_workflow_preset("yaml-test", Some(root.to_str().unwrap()));
        assert!(preset.is_ok());
        assert_eq!(preset.unwrap().id, "yaml-test");
    }

    #[test]
    fn semantic_rule_s1_dev_without_task() {
        let root = temp_project();
        let presets_dir = root.join(".opensunstar").join("workflow.presets");
        fs::create_dir_all(&presets_dir).unwrap();
        // Preset with dev but no task → should trigger S1
        let bad_preset = r#"{
            "id": "bad-s1",
            "name": "Bad S1",
            "description": "dev without task",
            "modules": [],
            "stages": [
                {"id": "4-dev", "name": "Dev", "artifacts": []},
                {"id": "6-review", "name": "Review", "artifacts": [{"file": "REVIEW.md", "scope": "change", "optional": false}]}
            ],
            "paths": {"frontend": ["4-dev", "6-review"], "backend": ["4-dev", "6-review"], "cli": ["4-dev", "6-review"], "mvp": ["4-dev", "6-review"]}
        }"#;
        fs::write(presets_dir.join("bad-s1.json"), bad_preset).unwrap();

        let preset = get_workflow_preset("bad-s1", Some(root.to_str().unwrap())).unwrap();
        let issues = validate_preset_semantic(&preset, "frontend");
        assert!(issues.iter().any(|i| i.starts_with("S1:")));
    }

    #[test]
    fn semantic_rule_s5_ui_design_in_backend_path() {
        // Preset with ui-design in backend path without skipWhen → should trigger S5
        let preset = WorkflowPreset {
            id: "bad-s5".into(),
            name: "Bad S5".into(),
            name_zh: None,
            description: "ui-design in backend without skipWhen".into(),
            r3_tier: None,
            modules: vec![],
            stages: vec![
                WorkflowStage {
                    id: "2a-ui-design".into(),
                    name: "UI Design".into(),
                    prompt: None,
                    depends_on: vec![],
                    skip_when: None,
                    artifacts: vec![],
                },
                WorkflowStage {
                    id: "3-task".into(),
                    name: "Task".into(),
                    prompt: None,
                    depends_on: vec![],
                    skip_when: None,
                    artifacts: vec![],
                },
            ],
            paths: WorkflowPresetPaths {
                frontend: vec!["2a-ui-design".into(), "3-task".into()],
                backend: vec!["2a-ui-design".into(), "3-task".into()],
                cli: vec!["3-task".into()],
                mvp: vec!["2a-ui-design".into(), "3-task".into()],
            },
        };

        // backend with ui-design and no skipWhen → S5 fires
        let issues = validate_preset_semantic(&preset, "backend");
        assert!(
            issues.iter().any(|i| i.starts_with("S5:")),
            "Expected S5 warning for ui-design in backend path, got: {issues:?}"
        );

        // frontend should NOT trigger S5
        let issues_fe = validate_preset_semantic(&preset, "frontend");
        assert!(
            !issues_fe.iter().any(|i| i.starts_with("S5:")),
            "S5 should not fire for frontend"
        );

        // Now add skipWhen for backend → S5 should not fire
        let mut preset_with_skip = preset.clone();
        preset_with_skip.stages[0].skip_when = Some(WorkflowStageSkipWhen {
            project_type: vec!["backend".into()],
        });
        let issues_skip = validate_preset_semantic(&preset_with_skip, "backend");
        assert!(
            !issues_skip.iter().any(|i| i.starts_with("S5:")),
            "S5 should not fire when skipWhen covers backend"
        );
    }

    #[test]
    fn artifact_path_project_root_scope() {
        let spec = WorkflowArtifactSpec {
            file: "README.md".into(),
            scope: "project-root".into(),
            optional: false,
        };
        let (path, relative) = artifact_path("/tmp/test-project", "my-change", &spec);
        assert_eq!(path, PathBuf::from("/tmp/test-project/README.md"));
        assert_eq!(relative, "README.md");
    }

    #[test]
    fn export_flow_config_writes_yaml() {
        let root = temp_project();
        let config =
            export_flow_config(root.to_str().unwrap(), "mvp", "backend", None, None).unwrap();
        assert_eq!(config.preset_id, "mvp");
        // R9.6 safety valve always enforced
        assert_eq!(config.rules.max_auto_retry, R96_MAX_AUTO_RETRY);
        assert!(config.rules.role_separation);
        assert!(config.rules.require_diff_boundary);
        let path = root.join(".opensunstar").join(FLOW_CONFIG_FILENAME);
        assert!(path.is_file());
    }

    #[test]
    fn strict_profile_export_rejects_semantically_invalid_stage_selection() {
        let root = temp_project();

        let error = export_project_workflow_profile_strict(
            root.to_str().unwrap(),
            "standard",
            "backend",
            None,
            None,
            Some(&["3-task".to_string()]),
        )
        .unwrap_err();

        assert!(error.to_string().contains("S1"));
        assert!(!root.join(".opensunstar/workflow.profile.json").exists());
    }

    #[test]
    fn strict_flow_config_export_rejects_semantically_invalid_stage_selection() {
        let root = temp_project();

        let error = export_flow_config_strict(
            root.to_str().unwrap(),
            "standard",
            "backend",
            None,
            Some(&["3-task".to_string()]),
        )
        .unwrap_err();

        assert!(error.to_string().contains("S1"));
        assert!(!root.join(".opensunstar/flow-config.yaml").exists());
    }

    #[test]
    fn disabled_stages_excluded_from_profile() {
        let root = temp_project();
        let disabled = vec!["3-task".to_string()];
        let profile = export_project_workflow_profile(
            root.to_str().unwrap(),
            "mvp",
            "backend",
            Some("demo-change"),
            None,
            Some(&disabled),
        )
        .unwrap();
        assert!(!profile.resolved_stages.contains(&"3-task".to_string()));
        // other stages still present
        assert!(profile
            .resolved_stages
            .contains(&"1-requirement".to_string()));
    }

    #[test]
    fn flow_config_r96_safety_valve_enforced() {
        let root = temp_project();
        let config =
            export_flow_config(root.to_str().unwrap(), "standard", "backend", None, None).unwrap();
        // Even though user could theoretically request weaker rules,
        // R9.6 constants are hardcoded and always enforced
        assert_eq!(config.rules.max_auto_retry, 3);
        assert!(config.rules.role_separation);
        assert!(config.rules.require_diff_boundary);
        // Gates should be populated for stages with required artifacts
        let has_gates = config.stages.iter().any(|s| !s.gates.is_empty());
        assert!(
            has_gates,
            "stages with required artifacts should have gates"
        );
    }
}
