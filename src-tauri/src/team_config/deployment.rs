//! 部署计划生成（Git MVP M2）
//!
//! 将有效配置编译结果（EffectiveConfig）与项目目录当前状态对比，
//! 产出文件级部署计划：Create / Update / Remove / Skip / DisplayOnly。
//!
//! 设计约束（冻结文档 §二 范围约束）：
//! - 可部署资产：Prompt, Rule, Skill, Ignore, Permission
//! - 仅展示（风险标注，不写入）：MCP, Command, Hook, Subagent
//! - 目标工具限 Claude Code + Codex
//! - plan_sha256 对相同输入必须确定性（D11）

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::domain::{AssetType, RiskLevel, TargetApp};
use super::effective_state::{EffectiveConfig, EffectiveDecision, EffectiveItem};

// ─── 部署动作 ──────────────────────────────────────────────────────────────────

/// 单个资产的部署动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentAction {
    /// 目标不存在，将新建
    Create,
    /// 目标已存在但内容不同，将覆盖
    Update,
    /// 资产被策略拒绝但目标仍存在，将移除
    Remove,
    /// 内容已一致或决策为 Skipped，无需操作
    Skip,
    /// 仅展示（MCP/Command/Hook/Subagent），不执行写入
    DisplayOnly,
}

impl DeploymentAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Remove => "remove",
            Self::Skip => "skip",
            Self::DisplayOnly => "display_only",
        }
    }

    /// 是否为写入动作（Create / Update / Remove）
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Create | Self::Update | Self::Remove)
    }
}

// ─── 部署步骤 ──────────────────────────────────────────────────────────────────

/// 单个资产的部署步骤
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStep {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub action: DeploymentAction,
    /// 风险等级（来自安全扫描或资产类型固有）
    pub risk_level: RiskLevel,
    /// 目标路径描述（相对于项目根）
    pub target_path: String,
    /// 期望内容 SHA-256（Create/Update 时有值）
    pub desired_sha256: Option<String>,
    /// 当前内容 SHA-256（Update/Remove 时有值）
    pub current_sha256: Option<String>,
    /// 人类可读的操作说明
    pub explanation: String,
}

// ─── 部署计划 ──────────────────────────────────────────────────────────────────

/// 完整部署计划
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentPlan {
    pub project_id: String,
    pub target_app: TargetApp,
    /// 所有部署步骤（按 asset_type + asset_id 排序，保证确定性）
    pub steps: Vec<DeploymentStep>,
    /// 汇总统计
    pub summary: PlanSummary,
    /// 计划级警告（如需要信任的 MCP、高风险资产）
    pub warnings: Vec<PlanWarning>,
    /// 确定性计划摘要（对 steps 序列化后 SHA-256）
    pub plan_sha256: String,
}

/// 计划汇总
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub total_assets: usize,
    pub create_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub skip_count: usize,
    pub display_only_count: usize,
    /// 需要写入的步骤数（create + update + remove）
    pub write_count: usize,
    /// 是否存在高风险或需要信任的资产
    pub has_high_risk: bool,
}

/// 计划级警告
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWarning {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub code: WarningCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    /// MCP 服务器需要首次显式信任
    RequiresTrust,
    /// 高风险资产（可执行 Hook 等）
    HighRisk,
    /// 目标工具不支持此资产类型
    UnsupportedTarget,
    /// 凭证槽位未绑定
    CredentialUnbound,
}

// ─── 可部署性判定 ──────────────────────────────────────────────────────────────

/// 资产类型是否可部署（MVP 范围：Prompt, Rule, Skill, Ignore, Permission）
pub fn is_deployable(asset_type: &AssetType) -> bool {
    matches!(
        asset_type,
        AssetType::Prompt
            | AssetType::Rule
            | AssetType::Skill
            | AssetType::Ignore
            | AssetType::Permission
    )
}

/// 资产类型的固有风险（未被安全扫描标注时的默认值）
fn inherent_risk(asset_type: &AssetType) -> RiskLevel {
    match asset_type {
        AssetType::Prompt | AssetType::Rule | AssetType::Ignore => RiskLevel::Safe,
        AssetType::Skill => RiskLevel::Low,
        AssetType::Permission => RiskLevel::Medium,
        AssetType::Mcp => RiskLevel::RequiresTrust,
        AssetType::Command | AssetType::Hook | AssetType::Subagent => RiskLevel::High,
    }
}

// ─── 目标路径解析 ──────────────────────────────────────────────────────────────

/// 解析资产在目标工具中的相对路径（相对于项目根目录）
///
/// 返回 None 表示该资产类型在目标工具中无对应路径（不支持）
pub fn resolve_target_relative_path(
    asset_type: &AssetType,
    asset_id: &str,
    target_app: &TargetApp,
) -> Option<String> {
    match target_app {
        TargetApp::ClaudeCode => match asset_type {
            AssetType::Prompt | AssetType::Rule => Some("CLAUDE.md".to_string()),
            AssetType::Skill => Some(format!(".claude/skills/{asset_id}")),
            AssetType::Ignore => Some(".claudeignore".to_string()),
            AssetType::Permission => Some(".claude/settings.json".to_string()),
            AssetType::Mcp => Some(".mcp.json".to_string()),
            AssetType::Command => Some(format!(".claude/commands/{asset_id}.md")),
            AssetType::Hook => Some(".claude/settings.json".to_string()),
            AssetType::Subagent => Some(format!(".claude/agents/{asset_id}.md")),
        },
        TargetApp::Codex => match asset_type {
            AssetType::Prompt | AssetType::Rule => Some("AGENTS.md".to_string()),
            AssetType::Skill => Some(format!(".codex/skills/{asset_id}")),
            AssetType::Ignore => Some(".codexignore".to_string()),
            AssetType::Permission => Some(".codex/config.toml".to_string()),
            AssetType::Mcp => Some(".mcp.json".to_string()),
            AssetType::Command => Some(format!(".codex/commands/{asset_id}.md")),
            AssetType::Hook => Some(".codex/config.toml".to_string()),
            AssetType::Subagent => Some(format!(".codex/agents/{asset_id}.toml")),
        },
        // MVP 仅支持 Claude Code + Codex
        _ => None,
    }
}

/// 解析资产在项目中的绝对路径
pub fn resolve_target_absolute_path(
    project_root: &Path,
    asset_type: &AssetType,
    asset_id: &str,
    target_app: &TargetApp,
) -> Option<PathBuf> {
    resolve_target_relative_path(asset_type, asset_id, target_app).map(|rel| project_root.join(rel))
}

// ─── 当前状态扫描 ──────────────────────────────────────────────────────────────

/// 扫描项目中某个资产的当前内容 SHA-256
///
/// 对于文件级资产（Skill, Command, Subagent）：读取文件内容计算 SHA-256
/// 对于节级资产（Prompt/Rule in CLAUDE.md, Permission in settings.json）：
///   检查目标文件是否存在（存在则返回文件整体 SHA-256 作为粗略标记）
///
/// 返回 None 表示目标不存在
pub fn scan_current_sha256(
    project_root: &Path,
    asset_type: &AssetType,
    asset_id: &str,
    target_app: &TargetApp,
) -> Option<String> {
    let path = resolve_target_absolute_path(project_root, asset_type, asset_id, target_app)?;

    match asset_type {
        // 目录型资产（Skill）：检查目录是否存在
        AssetType::Skill => {
            if path.is_dir() {
                // 对目录内所有文件内容做聚合哈希
                Some(hash_directory(&path))
            } else {
                None
            }
        }
        // 文件型资产
        _ => {
            if path.is_file() {
                std::fs::read(&path)
                    .ok()
                    .map(|content| super::release::sha256_of_content(&content))
            } else {
                None
            }
        }
    }
}

/// 对目录内所有文件做确定性聚合哈希
fn hash_directory(dir: &Path) -> String {
    let mut hasher = Sha256::new();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(content) = std::fs::read(&path) {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    entries.push((name, content));
                }
            }
        }
    }

    // 按文件名排序保证确定性
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, content) in &entries {
        hasher.update(name.as_bytes());
        hasher.update(content);
    }

    format!("{:x}", hasher.finalize())
}

// ─── 计划生成 ──────────────────────────────────────────────────────────────────

/// 生成部署计划
///
/// 将有效配置编译结果与项目目录当前状态对比，产出文件级部署计划。
///
/// # 参数
/// - `config`: 有效配置编译结果（来自 effective_state 编译器）
/// - `project_root`: 项目根目录路径
///
/// # 确定性保证
/// 相同的 (config, project_root 文件状态) 输入必须产出相同的 plan_sha256。
/// steps 按 (asset_type, asset_id) 排序后序列化计算哈希。
pub fn generate_deployment_plan(config: &EffectiveConfig, project_root: &Path) -> DeploymentPlan {
    let mut steps: Vec<DeploymentStep> = Vec::new();
    let mut warnings: Vec<PlanWarning> = Vec::new();

    for item in &config.items {
        let step = build_step(item, &config.target_app, project_root);

        // 收集警告
        collect_warnings(item, &step, &mut warnings);

        steps.push(step);
    }

    // 确定性排序：按 asset_type 字符串 + asset_id
    steps.sort_by(|a, b| {
        let type_cmp = a.asset_type.as_str().cmp(b.asset_type.as_str());
        if type_cmp == std::cmp::Ordering::Equal {
            a.asset_id.cmp(&b.asset_id)
        } else {
            type_cmp
        }
    });

    let summary = compute_summary(&steps);
    let plan_sha256 = compute_plan_sha256(&steps);

    DeploymentPlan {
        project_id: config.project_id.clone(),
        target_app: config.target_app.clone(),
        steps,
        summary,
        warnings,
        plan_sha256,
    }
}

/// 为单个 EffectiveItem 构建部署步骤
fn build_step(item: &EffectiveItem, target_app: &TargetApp, project_root: &Path) -> DeploymentStep {
    let risk_level = item
        .risk_level
        .unwrap_or_else(|| inherent_risk(&item.asset_type));

    let target_path = resolve_target_relative_path(&item.asset_type, &item.asset_id, target_app)
        .unwrap_or_else(|| format!("<unsupported:{:?}>", target_app));

    // 不可部署资产 → DisplayOnly
    if !is_deployable(&item.asset_type) {
        return DeploymentStep {
            asset_type: item.asset_type.clone(),
            asset_id: item.asset_id.clone(),
            action: DeploymentAction::DisplayOnly,
            risk_level,
            target_path,
            desired_sha256: item.content_sha256.clone(),
            current_sha256: None,
            explanation: format!(
                "{} '{}' 仅展示，MVP 不执行写入",
                item.asset_type.as_str(),
                item.asset_id
            ),
        };
    }

    // 不支持的目标工具 → Skip
    if resolve_target_relative_path(&item.asset_type, &item.asset_id, target_app).is_none() {
        return DeploymentStep {
            asset_type: item.asset_type.clone(),
            asset_id: item.asset_id.clone(),
            action: DeploymentAction::Skip,
            risk_level,
            target_path,
            desired_sha256: None,
            current_sha256: None,
            explanation: format!("目标工具 {:?} 不支持此资产类型", target_app),
        };
    }

    let current_sha256 =
        scan_current_sha256(project_root, &item.asset_type, &item.asset_id, target_app);

    let type_str = item.asset_type.as_str();
    let id_str = &item.asset_id;

    match &item.decision {
        EffectiveDecision::Enabled => match &current_sha256 {
            None => {
                let explanation = format!("新建 {type_str} '{id_str}' → {target_path}");
                DeploymentStep {
                    asset_type: item.asset_type.clone(),
                    asset_id: item.asset_id.clone(),
                    action: DeploymentAction::Create,
                    risk_level,
                    target_path,
                    desired_sha256: item.content_sha256.clone(),
                    current_sha256: None,
                    explanation,
                }
            }
            Some(current) => {
                let desired = item.content_sha256.as_deref().unwrap_or("");
                if desired == current {
                    let explanation = format!("{type_str} '{id_str}' 内容已一致，无需操作");
                    DeploymentStep {
                        asset_type: item.asset_type.clone(),
                        asset_id: item.asset_id.clone(),
                        action: DeploymentAction::Skip,
                        risk_level,
                        target_path,
                        desired_sha256: item.content_sha256.clone(),
                        current_sha256: Some(current.clone()),
                        explanation,
                    }
                } else {
                    let explanation =
                        format!("更新 {type_str} '{id_str}' → {target_path}（内容变更）");
                    DeploymentStep {
                        asset_type: item.asset_type.clone(),
                        asset_id: item.asset_id.clone(),
                        action: DeploymentAction::Update,
                        risk_level,
                        target_path,
                        desired_sha256: item.content_sha256.clone(),
                        current_sha256: Some(current.clone()),
                        explanation,
                    }
                }
            }
        },
        EffectiveDecision::Denied => {
            if current_sha256.is_some() {
                let explanation = format!("移除 {type_str} '{id_str}'（策略拒绝）← {target_path}");
                DeploymentStep {
                    asset_type: item.asset_type.clone(),
                    asset_id: item.asset_id.clone(),
                    action: DeploymentAction::Remove,
                    risk_level,
                    target_path,
                    desired_sha256: None,
                    current_sha256,
                    explanation,
                }
            } else {
                let explanation = format!("{type_str} '{id_str}' 被策略拒绝且目标不存在，无需操作");
                DeploymentStep {
                    asset_type: item.asset_type.clone(),
                    asset_id: item.asset_id.clone(),
                    action: DeploymentAction::Skip,
                    risk_level,
                    target_path,
                    desired_sha256: None,
                    current_sha256: None,
                    explanation,
                }
            }
        }
        EffectiveDecision::Skipped | EffectiveDecision::Conflicted => {
            let explanation = format!(
                "{type_str} '{id_str}' 决策为 {:?}，不执行写入",
                item.decision
            );
            DeploymentStep {
                asset_type: item.asset_type.clone(),
                asset_id: item.asset_id.clone(),
                action: DeploymentAction::Skip,
                risk_level,
                target_path,
                desired_sha256: item.content_sha256.clone(),
                current_sha256,
                explanation,
            }
        }
    }
}

/// 收集计划级警告
fn collect_warnings(item: &EffectiveItem, step: &DeploymentStep, warnings: &mut Vec<PlanWarning>) {
    // MCP 需要显式信任
    if item.asset_type == AssetType::Mcp && item.decision == EffectiveDecision::Enabled {
        warnings.push(PlanWarning {
            asset_type: item.asset_type.clone(),
            asset_id: item.asset_id.clone(),
            code: WarningCode::RequiresTrust,
            message: format!("MCP 服务器 '{}' 需要首次显式信任后才能使用", item.asset_id),
        });
    }

    // 高风险资产
    if step.risk_level >= RiskLevel::High && step.action.is_write() {
        warnings.push(PlanWarning {
            asset_type: item.asset_type.clone(),
            asset_id: item.asset_id.clone(),
            code: WarningCode::HighRisk,
            message: format!(
                "{} '{}' 为高风险资产，请确认后再部署",
                item.asset_type.as_str(),
                item.asset_id
            ),
        });
    }

    // 凭证槽位未绑定（从 required_credentials 检查）
    // 注意：这里只标注，实际绑定在 M7 credentials.rs 处理
}

/// 计算计划汇总
fn compute_summary(steps: &[DeploymentStep]) -> PlanSummary {
    let mut summary = PlanSummary {
        total_assets: steps.len(),
        create_count: 0,
        update_count: 0,
        remove_count: 0,
        skip_count: 0,
        display_only_count: 0,
        write_count: 0,
        has_high_risk: false,
    };

    for step in steps {
        match step.action {
            DeploymentAction::Create => summary.create_count += 1,
            DeploymentAction::Update => summary.update_count += 1,
            DeploymentAction::Remove => summary.remove_count += 1,
            DeploymentAction::Skip => summary.skip_count += 1,
            DeploymentAction::DisplayOnly => summary.display_only_count += 1,
        }
        if step.action.is_write() {
            summary.write_count += 1;
        }
        if step.risk_level >= RiskLevel::High {
            summary.has_high_risk = true;
        }
    }

    summary
}

/// 计算确定性计划摘要
///
/// 对排序后的 steps 做规范化序列化（BTreeMap 保证字段序），再 SHA-256。
/// 同一输入集合必须产生相同 plan_sha256（D11）。
fn compute_plan_sha256(steps: &[DeploymentStep]) -> String {
    let mut hasher = Sha256::new();

    for step in steps {
        // 规范化序列化：使用 BTreeMap 保证字段顺序确定性
        let mut map = BTreeMap::new();
        map.insert("asset_type", step.asset_type.as_str().to_string());
        map.insert("asset_id", step.asset_id.clone());
        map.insert("action", step.action.as_str().to_string());
        map.insert(
            "desired_sha256",
            step.desired_sha256.clone().unwrap_or_default(),
        );
        map.insert(
            "current_sha256",
            step.current_sha256.clone().unwrap_or_default(),
        );

        // 逐字段写入哈希（避免 serde 格式变化影响）
        for (key, value) in &map {
            hasher.update(key.as_bytes());
            hasher.update(b"=");
            hasher.update(value.as_bytes());
            hasher.update(b";");
        }
    }

    format!("{:x}", hasher.finalize())
}

// ─── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team_config::effective_state::{
        EffectiveConfig, EffectiveConflict, EffectiveDecision, EffectiveItem, ProvenanceEntry,
        SourceTier,
    };
    use crate::team_config::requirements::PolicyAction;
    use std::fs;
    use tempfile::TempDir;

    fn make_item(
        asset_type: AssetType,
        asset_id: &str,
        decision: EffectiveDecision,
        content_sha256: Option<&str>,
    ) -> EffectiveItem {
        EffectiveItem {
            asset_type,
            asset_id: asset_id.to_string(),
            decision,
            content_ref: Some(format!("assets/{asset_id}.md")),
            content_sha256: content_sha256.map(|s| s.to_string()),
            provenance: vec![ProvenanceEntry {
                tier: SourceTier::TeamProfile,
                source_id: "profile/default".to_string(),
                action: PolicyAction::Recommended,
                explanation: "团队 Profile 推荐".to_string(),
            }],
            risk_level: None,
        }
    }

    fn make_config(items: Vec<EffectiveItem>, target_app: TargetApp) -> EffectiveConfig {
        EffectiveConfig {
            project_id: "proj_test".to_string(),
            target_app,
            items,
            conflicts: Vec::<EffectiveConflict>::new(),
            required_credentials: vec![],
            config_sha256: "abc123".to_string(),
        }
    }

    #[test]
    fn deployable_asset_types() {
        assert!(is_deployable(&AssetType::Prompt));
        assert!(is_deployable(&AssetType::Rule));
        assert!(is_deployable(&AssetType::Skill));
        assert!(is_deployable(&AssetType::Ignore));
        assert!(is_deployable(&AssetType::Permission));
        assert!(!is_deployable(&AssetType::Mcp));
        assert!(!is_deployable(&AssetType::Command));
        assert!(!is_deployable(&AssetType::Hook));
        assert!(!is_deployable(&AssetType::Subagent));
    }

    #[test]
    fn target_path_resolution_claude() {
        assert_eq!(
            resolve_target_relative_path(&AssetType::Prompt, "greeting", &TargetApp::ClaudeCode),
            Some("CLAUDE.md".to_string())
        );
        assert_eq!(
            resolve_target_relative_path(&AssetType::Skill, "code-review", &TargetApp::ClaudeCode),
            Some(".claude/skills/code-review".to_string())
        );
        assert_eq!(
            resolve_target_relative_path(&AssetType::Ignore, "secrets", &TargetApp::ClaudeCode),
            Some(".claudeignore".to_string())
        );
        assert_eq!(
            resolve_target_relative_path(
                &AssetType::Permission,
                "allow-read",
                &TargetApp::ClaudeCode
            ),
            Some(".claude/settings.json".to_string())
        );
    }

    #[test]
    fn target_path_resolution_codex() {
        assert_eq!(
            resolve_target_relative_path(&AssetType::Prompt, "greeting", &TargetApp::Codex),
            Some("AGENTS.md".to_string())
        );
        assert_eq!(
            resolve_target_relative_path(&AssetType::Subagent, "reviewer", &TargetApp::Codex),
            Some(".codex/agents/reviewer.toml".to_string())
        );
    }

    #[test]
    fn unsupported_target_returns_none() {
        assert_eq!(
            resolve_target_relative_path(&AssetType::Prompt, "x", &TargetApp::Cursor),
            None
        );
    }

    #[test]
    fn plan_create_for_new_asset() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(
            vec![make_item(
                AssetType::Ignore,
                "secrets",
                EffectiveDecision::Enabled,
                Some("aaa111"),
            )],
            TargetApp::ClaudeCode,
        );

        let plan = generate_deployment_plan(&config, tmp.path());
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].action, DeploymentAction::Create);
        assert_eq!(plan.summary.create_count, 1);
        assert_eq!(plan.summary.write_count, 1);
    }

    #[test]
    fn plan_skip_when_content_matches() {
        let tmp = TempDir::new().unwrap();
        // 创建 .claudeignore 并计算其 SHA-256
        let content = b"*.secret\n.env\n";
        fs::write(tmp.path().join(".claudeignore"), content).unwrap();
        let sha = super::super::release::sha256_of_content(content);

        let config = make_config(
            vec![make_item(
                AssetType::Ignore,
                "secrets",
                EffectiveDecision::Enabled,
                Some(&sha),
            )],
            TargetApp::ClaudeCode,
        );

        let plan = generate_deployment_plan(&config, tmp.path());
        assert_eq!(plan.steps[0].action, DeploymentAction::Skip);
        assert_eq!(plan.summary.skip_count, 1);
        assert_eq!(plan.summary.write_count, 0);
    }

    #[test]
    fn plan_update_when_content_differs() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".claudeignore"), b"old content").unwrap();

        let config = make_config(
            vec![make_item(
                AssetType::Ignore,
                "secrets",
                EffectiveDecision::Enabled,
                Some("new_hash_different"),
            )],
            TargetApp::ClaudeCode,
        );

        let plan = generate_deployment_plan(&config, tmp.path());
        assert_eq!(plan.steps[0].action, DeploymentAction::Update);
        assert_eq!(plan.summary.update_count, 1);
    }

    #[test]
    fn plan_remove_denied_existing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".claudeignore"), b"to be removed").unwrap();

        let config = make_config(
            vec![make_item(
                AssetType::Ignore,
                "secrets",
                EffectiveDecision::Denied,
                None,
            )],
            TargetApp::ClaudeCode,
        );

        let plan = generate_deployment_plan(&config, tmp.path());
        assert_eq!(plan.steps[0].action, DeploymentAction::Remove);
        assert_eq!(plan.summary.remove_count, 1);
    }

    #[test]
    fn plan_display_only_for_mcp() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(
            vec![make_item(
                AssetType::Mcp,
                "github-server",
                EffectiveDecision::Enabled,
                Some("mcp_hash"),
            )],
            TargetApp::ClaudeCode,
        );

        let plan = generate_deployment_plan(&config, tmp.path());
        assert_eq!(plan.steps[0].action, DeploymentAction::DisplayOnly);
        assert_eq!(plan.summary.display_only_count, 1);
        // MCP 应产生 RequiresTrust 警告
        assert!(plan
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::RequiresTrust));
    }

    #[test]
    fn plan_sha256_is_deterministic() {
        let tmp = TempDir::new().unwrap();
        let items = vec![
            make_item(
                AssetType::Prompt,
                "b-second",
                EffectiveDecision::Enabled,
                Some("h1"),
            ),
            make_item(
                AssetType::Ignore,
                "a-first",
                EffectiveDecision::Enabled,
                Some("h2"),
            ),
        ];
        let config = make_config(items, TargetApp::ClaudeCode);

        let plan1 = generate_deployment_plan(&config, tmp.path());
        let plan2 = generate_deployment_plan(&config, tmp.path());
        assert_eq!(plan1.plan_sha256, plan2.plan_sha256);

        // 步骤应按 asset_type + asset_id 排序
        assert_eq!(plan1.steps[0].asset_id, "a-first"); // ignore < prompt
        assert_eq!(plan1.steps[1].asset_id, "b-second");
    }

    #[test]
    fn plan_sha256_changes_with_different_input() {
        let tmp = TempDir::new().unwrap();
        let config_a = make_config(
            vec![make_item(
                AssetType::Ignore,
                "x",
                EffectiveDecision::Enabled,
                Some("h1"),
            )],
            TargetApp::ClaudeCode,
        );
        let config_b = make_config(
            vec![make_item(
                AssetType::Ignore,
                "x",
                EffectiveDecision::Enabled,
                Some("h2"),
            )],
            TargetApp::ClaudeCode,
        );

        let plan_a = generate_deployment_plan(&config_a, tmp.path());
        let plan_b = generate_deployment_plan(&config_b, tmp.path());
        assert_ne!(plan_a.plan_sha256, plan_b.plan_sha256);
    }

    #[test]
    fn skill_directory_hashing() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp
            .path()
            .join(".claude")
            .join("skills")
            .join("code-review");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), b"# Code Review Skill").unwrap();
        fs::write(skill_dir.join("config.json"), b"{}").unwrap();

        let sha = scan_current_sha256(
            tmp.path(),
            &AssetType::Skill,
            "code-review",
            &TargetApp::ClaudeCode,
        );
        assert!(sha.is_some());

        // 修改内容后哈希应变化
        fs::write(skill_dir.join("SKILL.md"), b"# Updated Skill").unwrap();
        let sha2 = scan_current_sha256(
            tmp.path(),
            &AssetType::Skill,
            "code-review",
            &TargetApp::ClaudeCode,
        );
        assert_ne!(sha, sha2);
    }
}
