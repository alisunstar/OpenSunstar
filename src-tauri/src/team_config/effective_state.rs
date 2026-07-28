//! 有效配置编译器（第二版 §11.4）
//!
//! 将多来源要求（Team Policy > Team Profile > Project > Personal > Tool Default）
//! 编译为每个 (project, target_app) 的有效配置状态，附带完整来源解释。
//!
//! 编译输出必须包括：
//! - 最终期望值
//! - 所有候选来源和决策链
//! - 冲突与阻塞原因
//! - 需要的凭证槽位
//! - 目标工具适配器与能力限制
//! - 将生成的部署步骤

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::domain::{AssetType, CredentialSlot, PolicyRule, TargetApp, TeamProfile};
use super::requirements::PolicyAction;

// ─── 来源层级（第二版 §7.2） ───────────────────────────────────────────────────

/// 来源层级优先级（数值越大优先级越高）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceTier {
    /// 目标 AI 工具默认值（最低）
    ToolDefault = 0,
    /// 个人全局偏好
    Personal = 10,
    /// 本地项目配置
    ProjectLocal = 20,
    /// 项目共享配置 / 项目 Git 文件
    ProjectShared = 30,
    /// 团队 Profile 默认值
    TeamProfile = 40,
    /// 团队受管策略（最高）
    TeamPolicy = 50,
}

impl SourceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolDefault => "tool_default",
            Self::Personal => "personal",
            Self::ProjectLocal => "project_local",
            Self::ProjectShared => "project_shared",
            Self::TeamProfile => "team_profile",
            Self::TeamPolicy => "team_policy",
        }
    }
}

// ─── 编译输入 ──────────────────────────────────────────────────────────────────

/// 编译器输入：所有来源的配置集合
#[derive(Debug, Clone)]
pub struct CompilerInput {
    /// 团队 Profile（已解析的资产列表）
    pub team_profiles: Vec<TeamProfile>,
    /// 团队策略规则
    pub team_policies: Vec<PolicyRule>,
    /// 项目级资产期望（来自 project_asset_expectations 或本地 .opensunstar/）
    pub project_assets: Vec<ProjectAssetInput>,
    /// 个人偏好
    pub personal_overrides: Vec<PersonalOverride>,
    /// 目标工具
    pub target_app: TargetApp,
    /// 项目 ID
    pub project_id: String,
}

/// 项目级资产输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssetInput {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub content_ref: String,
    pub content_sha256: Option<String>,
    pub source_tier: SourceTier,
}

/// 个人偏好覆盖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalOverride {
    pub asset_type: AssetType,
    pub asset_id: String,
    /// 个人选择：enabled / disabled / custom_content
    pub preference: PersonalPreference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalPreference {
    Enabled,
    Disabled,
    Custom(String),
}

// ─── 编译输出 ──────────────────────────────────────────────────────────────────

/// 有效配置编译结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConfig {
    pub project_id: String,
    pub target_app: TargetApp,
    /// 每个资产的有效状态
    pub items: Vec<EffectiveItem>,
    /// 编译过程中发现的冲突
    pub conflicts: Vec<EffectiveConflict>,
    /// 需要的凭证槽位（去重后）
    pub required_credentials: Vec<CredentialSlot>,
    /// 编译摘要（确定性 SHA-256）
    pub config_sha256: String,
}

/// 单个资产的有效状态（含完整来源解释）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveItem {
    pub asset_type: AssetType,
    pub asset_id: String,
    /// 最终决策
    pub decision: EffectiveDecision,
    /// 最终内容引用（如果 enabled）
    pub content_ref: Option<String>,
    pub content_sha256: Option<String>,
    /// 决策来源链（从高到低优先级）
    pub provenance: Vec<ProvenanceEntry>,
    /// 风险等级
    pub risk_level: Option<super::domain::RiskLevel>,
}

/// 有效决策
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveDecision {
    /// 启用（必须或推荐且未被拒绝）
    Enabled,
    /// 禁用（被策略拒绝）
    Denied,
    /// 推荐但用户显式跳过
    Skipped,
    /// 冲突（无法自动决策）
    Conflicted,
}

/// 来源解释条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceEntry {
    pub tier: SourceTier,
    pub source_id: String,
    pub action: PolicyAction,
    /// 人类可读的决策说明
    pub explanation: String,
}

/// 编译冲突
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConflict {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub code: ConflictCode,
    pub source_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCode {
    /// 策略同时 require 和 deny
    PolicyContradiction,
    /// 多个来源固定了不同修订
    RevisionMismatch,
    /// 同 ID 不同内容（无法自动合并）
    ContentDivergence,
    /// 凭证槽位缺失
    CredentialMissing,
}

// ─── 编译器实现 ────────────────────────────────────────────────────────────────

/// 编译有效配置
///
/// 算法（第二版 §7.2）：
/// 1. 收集所有来源的资产条目，按 (asset_type, asset_id) 分组
/// 2. 对每组应用策略解析：deny 单调、required 并集、推荐可跳过
/// 3. 确定最终内容：最高优先级的 enabled 来源胜出
/// 4. 生成来源解释链
/// 5. 收集冲突和凭证需求
pub fn compile_effective_config(input: &CompilerInput) -> EffectiveConfig {
    let mut items: Vec<EffectiveItem> = Vec::new();
    let mut conflicts: Vec<EffectiveConflict> = Vec::new();
    let mut credential_set: BTreeMap<String, CredentialSlot> = BTreeMap::new();

    // 1. 收集所有资产 key
    let mut asset_groups: BTreeMap<(AssetType, String), Vec<CandidateEntry>> = BTreeMap::new();

    // 从团队 Profile 收集
    for profile in &input.team_profiles {
        for asset in &profile.assets {
            // 检查 target_app 过滤
            if let Some(targets) = &asset.target_apps {
                if !targets.contains(&input.target_app) {
                    continue;
                }
            }
            let key = (asset.asset_type.clone(), asset.asset_id.clone());
            asset_groups.entry(key).or_default().push(CandidateEntry {
                tier: SourceTier::TeamProfile,
                source_id: format!("profile:{}", profile.profile_id),
                action: PolicyAction::Recommended, // Profile 资产默认推荐
                content_ref: Some(asset.content_ref.clone()),
                content_sha256: asset.content_sha256.clone(),
                risk_level: asset.risk_level,
            });
        }
        // 收集凭证槽位
        for slot in &profile.credential_slots {
            credential_set
                .entry(slot.slot_id.clone())
                .or_insert_with(|| slot.clone());
        }
    }

    // 从项目级收集
    for project_asset in &input.project_assets {
        let key = (
            project_asset.asset_type.clone(),
            project_asset.asset_id.clone(),
        );
        asset_groups.entry(key).or_default().push(CandidateEntry {
            tier: project_asset.source_tier,
            source_id: format!("project:{}", project_asset.asset_id),
            action: PolicyAction::Recommended,
            content_ref: Some(project_asset.content_ref.clone()),
            content_sha256: project_asset.content_sha256.clone(),
            risk_level: None,
        });
    }

    // 叠加团队策略
    //
    // 策略是决策修饰符，不是资产来源：只能作用于上面已收集到的资产组，
    // 不能凭空建组。否则一条 `[[policies]] type="permission" pattern="Bash"
    // action="denied"` 会生成名为 "Bash" 的幽灵 Permission 资产，解析到
    // .claude/settings.json 并被 Remove 整体删除。
    for policy in &input.team_policies {
        if let Some(targets) = &policy.target_apps {
            if !targets.contains(&input.target_app) {
                continue;
            }
        }

        let matched: Vec<(AssetType, String)> = asset_groups
            .keys()
            .filter(|(asset_type, asset_id)| {
                *asset_type == policy.asset_type
                    && policy_pattern_matches(&policy.asset_pattern, asset_id)
            })
            .cloned()
            .collect();

        for key in matched {
            asset_groups.entry(key).or_default().push(CandidateEntry {
                tier: SourceTier::TeamPolicy,
                source_id: format!("policy:{}", policy.rule_id),
                action: policy.action,
                content_ref: None,
                content_sha256: None,
                risk_level: None,
            });
        }
    }

    // 2. 对每组应用解析规则
    for ((asset_type, asset_id), candidates) in &asset_groups {
        let (item, item_conflicts) =
            resolve_asset_group(asset_type, asset_id, candidates, &input.personal_overrides);
        items.push(item);
        conflicts.extend(item_conflicts);
    }

    // 3. 计算确定性摘要
    let config_sha256 = {
        use sha2::{Digest, Sha256};
        let serialized = serde_json::to_vec(&items).expect("items serializable");
        format!("{:x}", Sha256::digest(serialized))
    };

    // 4. 只保留 required 的凭证槽位
    let required_credentials: Vec<CredentialSlot> = credential_set
        .into_values()
        .filter(|s| s.required)
        .collect();

    EffectiveConfig {
        project_id: input.project_id.clone(),
        target_app: input.target_app.clone(),
        items,
        conflicts,
        required_credentials,
        config_sha256,
    }
}

/// 策略 pattern 匹配资产 ID：精确匹配或 "*" 通配（见 domain::PolicyRule::asset_pattern）
fn policy_pattern_matches(pattern: &str, asset_id: &str) -> bool {
    pattern == "*" || pattern == asset_id
}

/// 内部候选条目
#[derive(Debug, Clone)]
struct CandidateEntry {
    tier: SourceTier,
    source_id: String,
    action: PolicyAction,
    content_ref: Option<String>,
    content_sha256: Option<String>,
    risk_level: Option<super::domain::RiskLevel>,
}

/// 解析单个资产组
fn resolve_asset_group(
    asset_type: &AssetType,
    asset_id: &str,
    candidates: &[CandidateEntry],
    personal_overrides: &[PersonalOverride],
) -> (EffectiveItem, Vec<EffectiveConflict>) {
    let mut conflicts = Vec::new();

    // 按优先级排序（高 → 低）
    let mut sorted: Vec<&CandidateEntry> = candidates.iter().collect();
    sorted.sort_by(|a, b| b.tier.cmp(&a.tier));

    // 检查 deny 单调性
    let has_deny = sorted.iter().any(|c| c.action == PolicyAction::Denied);
    let has_required = sorted.iter().any(|c| c.action == PolicyAction::Required);

    // 检查个人偏好
    let personal = personal_overrides
        .iter()
        .find(|p| p.asset_type == *asset_type && p.asset_id == asset_id);

    // 构建来源解释链
    let provenance: Vec<ProvenanceEntry> = sorted
        .iter()
        .map(|c| ProvenanceEntry {
            tier: c.tier,
            source_id: c.source_id.clone(),
            action: c.action,
            explanation: format_explanation(c),
        })
        .collect();

    // 决策逻辑
    let decision = if has_deny && has_required {
        // 冲突：同时 require 和 deny
        conflicts.push(EffectiveConflict {
            asset_type: asset_type.clone(),
            asset_id: asset_id.to_string(),
            code: ConflictCode::PolicyContradiction,
            source_ids: sorted.iter().map(|c| c.source_id.clone()).collect(),
            message: format!("资产 {asset_id} 同时被 require 和 deny，无法自动决策"),
        });
        EffectiveDecision::Conflicted
    } else if has_deny {
        // Deny 单调：任何来源 deny → 最终 deny
        EffectiveDecision::Denied
    } else if let Some(p) = personal {
        match p.preference {
            PersonalPreference::Disabled => EffectiveDecision::Skipped,
            PersonalPreference::Enabled | PersonalPreference::Custom(_) => {
                EffectiveDecision::Enabled
            }
        }
    } else {
        EffectiveDecision::Enabled
    };

    // 确定内容：最高优先级的有内容来源胜出
    let content_source = if decision == EffectiveDecision::Enabled {
        // 个人自定义内容优先
        if let Some(p) = personal {
            if let PersonalPreference::Custom(content) = &p.preference {
                Some((content.clone(), None))
            } else {
                sorted
                    .iter()
                    .find(|c| c.content_ref.is_some())
                    .map(|c| (c.content_ref.clone().unwrap(), c.content_sha256.clone()))
            }
        } else {
            sorted
                .iter()
                .find(|c| c.content_ref.is_some())
                .map(|c| (c.content_ref.clone().unwrap(), c.content_sha256.clone()))
        }
    } else {
        None
    };

    // 检查内容分歧（同 ID 不同 SHA）
    let content_shas: std::collections::BTreeSet<_> = sorted
        .iter()
        .filter_map(|c| c.content_sha256.as_ref())
        .collect();
    if content_shas.len() > 1 && decision == EffectiveDecision::Enabled {
        conflicts.push(EffectiveConflict {
            asset_type: asset_type.clone(),
            asset_id: asset_id.to_string(),
            code: ConflictCode::ContentDivergence,
            source_ids: sorted
                .iter()
                .filter(|c| c.content_sha256.is_some())
                .map(|c| c.source_id.clone())
                .collect(),
            message: format!("资产 {asset_id} 存在多个不同内容版本，已选择最高优先级来源"),
        });
    }

    let risk_level = sorted.iter().find_map(|c| c.risk_level);

    let item = EffectiveItem {
        asset_type: asset_type.clone(),
        asset_id: asset_id.to_string(),
        decision,
        content_ref: content_source.as_ref().map(|(r, _)| r.clone()),
        content_sha256: content_source.and_then(|(_, s)| s),
        provenance,
        risk_level,
    };

    (item, conflicts)
}

/// 生成来源解释文本
fn format_explanation(entry: &CandidateEntry) -> String {
    let tier_name = match entry.tier {
        SourceTier::TeamPolicy => "团队策略",
        SourceTier::TeamProfile => "团队 Profile",
        SourceTier::ProjectShared => "项目共享配置",
        SourceTier::ProjectLocal => "项目本地配置",
        SourceTier::Personal => "个人偏好",
        SourceTier::ToolDefault => "工具默认值",
    };
    let action_name = match entry.action {
        PolicyAction::Required => "必需",
        PolicyAction::Recommended => "推荐",
        PolicyAction::Denied => "禁止",
    };
    format!("[{}] {} → {}", tier_name, entry.source_id, action_name)
}

#[cfg(test)]
mod tests {
    use super::super::domain::{ProfileAsset, RiskLevel};
    use super::*;

    fn backend_profile() -> TeamProfile {
        TeamProfile {
            profile_id: "profile-backend".to_string(),
            name: "Backend Profile".to_string(),
            description: Some("后端开发标准配置".to_string()),
            assets: vec![
                ProfileAsset {
                    asset_type: AssetType::Prompt,
                    asset_id: "backend-system".to_string(),
                    content_ref: "prompts/backend.md".to_string(),
                    content_sha256: Some("aaa111".to_string()),
                    target_apps: None,
                    risk_level: Some(RiskLevel::Safe),
                },
                ProfileAsset {
                    asset_type: AssetType::Permission,
                    asset_id: "default-permissions".to_string(),
                    content_ref: "permissions/default.json".to_string(),
                    content_sha256: Some("bbb222".to_string()),
                    target_apps: None,
                    risk_level: Some(RiskLevel::Low),
                },
                ProfileAsset {
                    asset_type: AssetType::Mcp,
                    asset_id: "github-mcp".to_string(),
                    content_ref: "mcp/github.json".to_string(),
                    content_sha256: Some("ccc333".to_string()),
                    target_apps: Some(vec![TargetApp::ClaudeCode]),
                    risk_level: Some(RiskLevel::RequiresTrust),
                },
            ],
            credential_slots: vec![CredentialSlot {
                slot_id: "TEAM_GITHUB".to_string(),
                kind: "oauth".to_string(),
                provider: Some("github".to_string()),
                description: Some("GitHub MCP 服务器凭证".to_string()),
                required: true,
            }],
            created_at: 1784736000000,
            updated_at: 1784736000000,
        }
    }

    #[test]
    fn compiles_backend_profile_for_claude_code() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        assert_eq!(config.project_id, "project-test");
        assert_eq!(config.target_app, TargetApp::ClaudeCode);
        // 3 个资产全部对 Claude Code 可见
        assert_eq!(config.items.len(), 3);
        assert!(config.conflicts.is_empty());
        assert!(!config.config_sha256.is_empty());

        // 验证 MCP 资产存在（target_apps 包含 ClaudeCode）
        let mcp_item = config.items.iter().find(|i| i.asset_id == "github-mcp");
        assert!(mcp_item.is_some());
        assert_eq!(mcp_item.unwrap().decision, EffectiveDecision::Enabled);
        assert_eq!(mcp_item.unwrap().risk_level, Some(RiskLevel::RequiresTrust));
    }

    #[test]
    fn filters_assets_by_target_app() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::Codex, // github-mcp 只对 ClaudeCode
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        // github-mcp 被过滤掉（target_apps = [ClaudeCode]）
        assert_eq!(config.items.len(), 2);
        assert!(config.items.iter().all(|i| i.asset_id != "github-mcp"));
    }

    #[test]
    fn deny_policy_overrides_profile_recommendation() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![PolicyRule {
                rule_id: "deny-github-mcp".to_string(),
                asset_type: AssetType::Mcp,
                asset_pattern: "github-mcp".to_string(),
                action: PolicyAction::Denied,
                target_apps: None,
                constraint_json: "{}".to_string(),
                reason: Some("安全策略：禁止外部 MCP".to_string()),
            }],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        let mcp_item = config.items.iter().find(|i| i.asset_id == "github-mcp");
        assert!(mcp_item.is_some());
        assert_eq!(mcp_item.unwrap().decision, EffectiveDecision::Denied);
        // 来源解释应包含两条记录
        assert_eq!(mcp_item.unwrap().provenance.len(), 2);
    }

    #[test]
    fn deny_policy_without_matching_asset_creates_no_phantom_item() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![PolicyRule {
                rule_id: "deny-bash".to_string(),
                asset_type: AssetType::Permission,
                asset_pattern: "Bash".to_string(),
                action: PolicyAction::Denied,
                target_apps: None,
                constraint_json: "{}".to_string(),
                reason: None,
            }],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        // 策略只是决策修饰符，不是资产。凭空生成条目会让 Permission 解析到
        // .claude/settings.json 并被 Remove 整体删除。
        assert!(
            config.items.iter().all(|i| i.asset_id != "Bash"),
            "策略 pattern 不得凭空生成资产条目, items={:?}",
            config.items.iter().map(|i| &i.asset_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn wildcard_policy_applies_to_all_assets_of_type() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![PolicyRule {
                rule_id: "deny-all-mcp".to_string(),
                asset_type: AssetType::Mcp,
                asset_pattern: "*".to_string(),
                action: PolicyAction::Denied,
                target_apps: None,
                constraint_json: "{}".to_string(),
                reason: None,
            }],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        let mcp_item = config.items.iter().find(|i| i.asset_id == "github-mcp");
        assert_eq!(
            mcp_item.map(|i| &i.decision),
            Some(&EffectiveDecision::Denied),
            "'*' 通配应作用于该类型全部资产"
        );
        assert!(
            config.items.iter().all(|i| i.asset_id != "*"),
            "通配符本身不得成为资产条目"
        );
    }

    #[test]
    fn personal_skip_overrides_recommendation() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![],
            project_assets: vec![],
            personal_overrides: vec![PersonalOverride {
                asset_type: AssetType::Prompt,
                asset_id: "backend-system".to_string(),
                preference: PersonalPreference::Disabled,
            }],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        let prompt_item = config.items.iter().find(|i| i.asset_id == "backend-system");
        assert_eq!(prompt_item.unwrap().decision, EffectiveDecision::Skipped);
    }

    #[test]
    fn required_and_deny_produces_conflict() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![
                PolicyRule {
                    rule_id: "require-mcp".to_string(),
                    asset_type: AssetType::Mcp,
                    asset_pattern: "github-mcp".to_string(),
                    action: PolicyAction::Required,
                    target_apps: None,
                    constraint_json: "{}".to_string(),
                    reason: None,
                },
                PolicyRule {
                    rule_id: "deny-mcp".to_string(),
                    asset_type: AssetType::Mcp,
                    asset_pattern: "github-mcp".to_string(),
                    action: PolicyAction::Denied,
                    target_apps: None,
                    constraint_json: "{}".to_string(),
                    reason: None,
                },
            ],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);

        assert!(!config.conflicts.is_empty());
        assert_eq!(config.conflicts[0].code, ConflictCode::PolicyContradiction);
        let mcp_item = config.items.iter().find(|i| i.asset_id == "github-mcp");
        assert_eq!(mcp_item.unwrap().decision, EffectiveDecision::Conflicted);
    }

    #[test]
    fn config_digest_is_deterministic() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config1 = compile_effective_config(&input);
        let config2 = compile_effective_config(&input);
        assert_eq!(config1.config_sha256, config2.config_sha256);
    }

    #[test]
    fn collects_required_credential_slots() {
        let input = CompilerInput {
            team_profiles: vec![backend_profile()],
            team_policies: vec![],
            project_assets: vec![],
            personal_overrides: vec![],
            target_app: TargetApp::ClaudeCode,
            project_id: "project-test".to_string(),
        };

        let config = compile_effective_config(&input);
        assert_eq!(config.required_credentials.len(), 1);
        assert_eq!(config.required_credentials[0].slot_id, "TEAM_GITHUB");
    }
}
