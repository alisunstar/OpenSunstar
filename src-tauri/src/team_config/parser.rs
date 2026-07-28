//! team.toml 解析器：TOML 文本 → 领域类型
//!
//! 将 team.toml 的声明式配置转换为编译器可消费的领域对象：
//! - TomlProfile → TeamProfile（含 ProfileAsset 列表）
//! - TomlPolicyRule → PolicyRule
//! - TomlCredentialSlot → CredentialSlot
//! - TomlCompatibility → CompatibilityMatrix 输入

use super::domain::{
    AssetType, CredentialSlot, PolicyRule, ProfileAsset, RiskLevel, TargetApp, TeamProfile,
    TeamToml, TomlAssetRef, TomlPolicyRule,
};
use super::requirements::PolicyAction;

/// 解析 team.toml 文本内容
pub fn parse_team_toml(content: &str) -> Result<TeamToml, TeamTomlError> {
    toml::from_str(content).map_err(|e| TeamTomlError::ParseError(e.to_string()))
}

/// 从解析后的 TeamToml 构建 TeamProfile 列表
///
/// 每个 [[profiles]] 条目生成一个 TeamProfile。
/// 资产内容引用（path/content）保留原始声明，SHA-256 在发布时由 release 模块计算。
pub fn build_profiles(
    team_toml: &TeamToml,
    credential_slots: &[CredentialSlot],
) -> Result<Vec<TeamProfile>, TeamTomlError> {
    let now = chrono::Utc::now().timestamp_millis();

    team_toml
        .profiles
        .iter()
        .map(|profile| {
            let assets = profile
                .assets
                .iter()
                .map(|asset_ref| build_profile_asset(asset_ref))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(TeamProfile {
                profile_id: profile.id.clone(),
                name: profile.name.clone(),
                description: profile.description.clone(),
                assets,
                credential_slots: credential_slots.to_vec(),
                created_at: now,
                updated_at: now,
            })
        })
        .collect()
}

/// 从解析后的 TeamToml 构建策略规则列表
pub fn build_policies(team_toml: &TeamToml) -> Result<Vec<PolicyRule>, TeamTomlError> {
    team_toml
        .policies
        .iter()
        .enumerate()
        .map(|(idx, rule)| build_policy_rule(rule, idx))
        .collect()
}

/// 从解析后的 TeamToml 构建凭证槽位列表
pub fn build_credential_slots(team_toml: &TeamToml) -> Vec<CredentialSlot> {
    team_toml
        .credential_slots
        .iter()
        .map(|slot| CredentialSlot {
            slot_id: slot.id.clone(),
            kind: slot.kind.clone(),
            provider: slot.provider.clone(),
            description: slot.description.clone(),
            required: slot.required,
        })
        .collect()
}

/// 一站式解析：team.toml 文本 → (profiles, policies, credential_slots)
pub fn parse_team_package(
    content: &str,
) -> Result<(Vec<TeamProfile>, Vec<PolicyRule>, Vec<CredentialSlot>), TeamTomlError> {
    let team_toml = parse_team_toml(content)?;
    let credential_slots = build_credential_slots(&team_toml);
    let profiles = build_profiles(&team_toml, &credential_slots)?;
    let policies = build_policies(&team_toml)?;
    Ok((profiles, policies, credential_slots))
}

// ─── 内部转换 ─────────────────────────────────────────────────────────────────

fn build_profile_asset(asset_ref: &TomlAssetRef) -> Result<ProfileAsset, TeamTomlError> {
    let asset_type = AssetType::from_str(&asset_ref.asset_type)
        .ok_or_else(|| TeamTomlError::InvalidAssetType(asset_ref.asset_type.clone()))?;

    // content_ref: 优先使用 path，其次使用内联 content 的虚拟引用
    let content_ref = if let Some(path) = &asset_ref.path {
        path.clone()
    } else if asset_ref.content.is_some() {
        // 内联内容：使用 "inline:{asset_id}" 作为引用标识
        format!("inline:{}", asset_ref.id)
    } else {
        return Err(TeamTomlError::MissingContentRef(asset_ref.id.clone()));
    };

    let target_apps = if asset_ref.targets.is_empty() {
        None
    } else {
        Some(
            asset_ref
                .targets
                .iter()
                .map(|t| TargetApp::from_str(t))
                .collect(),
        )
    };

    // 风险等级推断：MCP/Hook/Command 默认 RequiresTrust，其余 Safe
    let risk_level = Some(match asset_type {
        AssetType::Mcp | AssetType::Hook | AssetType::Command => RiskLevel::RequiresTrust,
        _ => RiskLevel::Safe,
    });

    Ok(ProfileAsset {
        asset_type,
        asset_id: asset_ref.id.clone(),
        content_ref,
        content_sha256: None, // 发布时计算
        target_apps,
        risk_level,
    })
}

fn build_policy_rule(rule: &TomlPolicyRule, idx: usize) -> Result<PolicyRule, TeamTomlError> {
    let asset_type = AssetType::from_str(&rule.asset_type)
        .ok_or_else(|| TeamTomlError::InvalidAssetType(rule.asset_type.clone()))?;

    let action = match rule.action.as_str() {
        "required" => PolicyAction::Required,
        "recommended" => PolicyAction::Recommended,
        "denied" => PolicyAction::Denied,
        other => return Err(TeamTomlError::InvalidAction(other.to_string())),
    };

    let target_apps = if rule.targets.is_empty() {
        None
    } else {
        Some(
            rule.targets
                .iter()
                .map(|t| TargetApp::from_str(t))
                .collect(),
        )
    };

    Ok(PolicyRule {
        rule_id: format!("policy-{idx}"),
        asset_type,
        asset_pattern: rule.pattern.clone(),
        action,
        target_apps,
        constraint_json: "{}".to_string(),
        reason: rule.reason.clone(),
    })
}

// ─── 错误类型 ─────────────────────────────────────────────────────────────────

/// team.toml 解析/转换错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamTomlError {
    /// TOML 语法错误
    ParseError(String),
    /// 无法识别的资产类型
    InvalidAssetType(String),
    /// 无法识别的策略动作
    InvalidAction(String),
    /// 资产缺少 path 和 content（二选一）
    MissingContentRef(String),
}

impl std::fmt::Display for TeamTomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "team_toml_parse_error: {msg}"),
            Self::InvalidAssetType(t) => write!(f, "team_toml_invalid_asset_type: {t}"),
            Self::InvalidAction(a) => write!(f, "team_toml_invalid_action: {a}"),
            Self::MissingContentRef(id) => {
                write!(
                    f,
                    "team_toml_missing_content_ref: asset '{id}' has neither path nor content"
                )
            }
        }
    }
}

impl std::error::Error for TeamTomlError {}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TEAM_TOML: &str = r#"
[team]
name = "Backend Team"
version = "1.2.0"
description = "后端团队标准配置"

[[team.compatibility]]
app = "claude_code"
min_version = "1.0.0"

[[team.compatibility]]
app = "codex"

[[profiles]]
id = "profile-backend"
name = "Backend Profile"
description = "后端开发标准配置"

[[profiles.assets]]
type = "prompt"
id = "backend-system"
path = "prompts/backend.md"

[[profiles.assets]]
type = "permission"
id = "default-permissions"
path = "permissions/default.json"

[[profiles.assets]]
type = "mcp"
id = "github-mcp"
path = "mcp/github.json"
targets = ["claude_code"]

[[policies]]
type = "permission"
pattern = "Bash"
action = "denied"
reason = "安全策略：禁止直接 Bash 执行"

[[policies]]
type = "prompt"
pattern = "backend-system"
action = "required"
targets = ["claude_code", "codex"]

[[credential_slots]]
id = "TEAM_GITHUB"
kind = "oauth"
provider = "github"
description = "GitHub MCP 服务器凭证"
required = true

[[credential_slots]]
id = "TEAM_OPENAI"
kind = "api_key"
provider = "openai"
required = false
"#;

    #[test]
    fn parses_sample_team_toml() {
        let team_toml = parse_team_toml(SAMPLE_TEAM_TOML).expect("parse");
        assert_eq!(team_toml.team.name.as_deref(), Some("Backend Team"));
        assert_eq!(team_toml.team.version.as_deref(), Some("1.2.0"));
        assert_eq!(team_toml.profiles.len(), 1);
        assert_eq!(team_toml.policies.len(), 2);
        assert_eq!(team_toml.credential_slots.len(), 2);
        assert_eq!(team_toml.team.compatibility.len(), 2);
    }

    #[test]
    fn builds_profiles_with_assets() {
        let team_toml = parse_team_toml(SAMPLE_TEAM_TOML).expect("parse");
        let slots = build_credential_slots(&team_toml);
        let profiles = build_profiles(&team_toml, &slots).expect("build profiles");

        assert_eq!(profiles.len(), 1);
        let profile = &profiles[0];
        assert_eq!(profile.profile_id, "profile-backend");
        assert_eq!(profile.assets.len(), 3);

        // 验证 MCP 资产只对 claude_code
        let mcp = profile
            .assets
            .iter()
            .find(|a| a.asset_id == "github-mcp")
            .unwrap();
        assert_eq!(mcp.asset_type, AssetType::Mcp);
        assert_eq!(mcp.target_apps, Some(vec![TargetApp::ClaudeCode]));
        assert_eq!(mcp.risk_level, Some(RiskLevel::RequiresTrust));

        // 验证 prompt 资产对所有工具
        let prompt = profile
            .assets
            .iter()
            .find(|a| a.asset_id == "backend-system")
            .unwrap();
        assert_eq!(prompt.target_apps, None);
        assert_eq!(prompt.risk_level, Some(RiskLevel::Safe));
    }

    #[test]
    fn builds_policies_with_correct_actions() {
        let team_toml = parse_team_toml(SAMPLE_TEAM_TOML).expect("parse");
        let policies = build_policies(&team_toml).expect("build policies");

        assert_eq!(policies.len(), 2);
        assert_eq!(policies[0].action, PolicyAction::Denied);
        assert_eq!(policies[0].asset_pattern, "Bash");
        assert_eq!(policies[1].action, PolicyAction::Required);
        assert_eq!(
            policies[1].target_apps,
            Some(vec![TargetApp::ClaudeCode, TargetApp::Codex])
        );
    }

    #[test]
    fn builds_credential_slots_with_required_flag() {
        let team_toml = parse_team_toml(SAMPLE_TEAM_TOML).expect("parse");
        let slots = build_credential_slots(&team_toml);

        assert_eq!(slots.len(), 2);
        let github = slots.iter().find(|s| s.slot_id == "TEAM_GITHUB").unwrap();
        assert!(github.required);
        assert_eq!(github.provider.as_deref(), Some("github"));

        let openai = slots.iter().find(|s| s.slot_id == "TEAM_OPENAI").unwrap();
        assert!(!openai.required);
    }

    #[test]
    fn parse_team_package_one_stop() {
        let (profiles, policies, slots) =
            parse_team_package(SAMPLE_TEAM_TOML).expect("parse package");

        assert_eq!(profiles.len(), 1);
        assert_eq!(policies.len(), 2);
        assert_eq!(slots.len(), 2);
        // Profile 应包含凭证槽位
        assert_eq!(profiles[0].credential_slots.len(), 2);
    }

    #[test]
    fn rejects_invalid_asset_type() {
        let bad_toml = r#"
[[profiles]]
id = "p1"
name = "Bad"

[[profiles.assets]]
type = "unknown_type"
id = "x"
path = "x.md"
"#;
        let result = parse_team_package(bad_toml);
        assert!(matches!(result, Err(TeamTomlError::InvalidAssetType(_))));
    }

    #[test]
    fn rejects_missing_content_ref() {
        let bad_toml = r#"
[[profiles]]
id = "p1"
name = "Bad"

[[profiles.assets]]
type = "prompt"
id = "no-content"
"#;
        let result = parse_team_package(bad_toml);
        assert!(matches!(result, Err(TeamTomlError::MissingContentRef(_))));
    }

    #[test]
    fn rejects_invalid_policy_action() {
        let bad_toml = r#"
[[policies]]
type = "permission"
pattern = "Bash"
action = "forbidden"
"#;
        let result = parse_team_package(bad_toml);
        assert!(matches!(result, Err(TeamTomlError::InvalidAction(_))));
    }
}
