//! 团队配置包校验器（Local Alpha L2）
//!
//! 职责：
//! - team.toml Schema 完整性校验（必填字段、枚举值、结构）
//! - 资产引用校验（路径存在、无路径穿越、无符号链接）
//! - 策略规则校验（有效 action、有效 asset_type）
//! - 安全扫描接线（复用 security.rs 的 enforce_team_package_security）
//! - 生成结构化校验报告（errors 阻断 / warnings 非阻断）
//!
//! 设计约束（第二版 §9.4 校验管线）：
//! - CRITICAL 发现直接阻止发布和应用
//! - 校验报告不返回匹配的秘密片段（防二次泄漏）

use std::path::Path;

use super::domain::{AssetType, TeamToml};
use super::parser::parse_team_toml;
use super::security::{validate_team_package_security, TeamPackageSecurityReport};

/// 校验报告
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// 阻断性错误（必须修复才能继续）
    pub errors: Vec<ValidationIssue>,
    /// 非阻断警告（可继续但应关注）
    pub warnings: Vec<ValidationIssue>,
    /// 安全扫描报告（如果执行了）
    pub security: Option<TeamPackageSecurityReport>,
    /// 校验是否通过（无 error 且安全扫描未 blocked）
    pub passed: bool,
}

/// 单条校验问题
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub message: String,
    /// 相关文件或字段路径
    pub location: Option<String>,
}

/// 校验问题编码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCode {
    // Schema 错误
    MissingTeamSection,
    MissingProfileId,
    MissingProfileName,
    InvalidAssetType,
    InvalidPolicyAction,
    MissingAssetContentRef,
    // 资产引用错误
    AssetFileNotFound,
    PathTraversal,
    // 策略错误
    PolicyContradiction,
    // 安全
    SecurityBlocked,
    // 兼容性
    UnknownTargetApp,
}

impl ValidationCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingTeamSection => "missing_team_section",
            Self::MissingProfileId => "missing_profile_id",
            Self::MissingProfileName => "missing_profile_name",
            Self::InvalidAssetType => "invalid_asset_type",
            Self::InvalidPolicyAction => "invalid_policy_action",
            Self::MissingAssetContentRef => "missing_asset_content_ref",
            Self::AssetFileNotFound => "asset_file_not_found",
            Self::PathTraversal => "path_traversal",
            Self::PolicyContradiction => "policy_contradiction",
            Self::SecurityBlocked => "security_blocked",
            Self::UnknownTargetApp => "unknown_target_app",
        }
    }
}

/// 校验选项
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// 是否执行安全扫描（连接时建议开启）
    pub run_security_scan: bool,
    /// 是否校验资产文件存在性（需要包根目录）
    pub check_asset_files: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            run_security_scan: true,
            check_asset_files: true,
        }
    }
}

/// 校验团队配置包
///
/// 完整校验管线（对应第二版 §9.4 的子集）：
/// 1. Schema 与枚举校验
/// 2. 路径穿越与文件存在校验
/// 3. 策略冲突检测
/// 4. 安全扫描（密钥、Token、高熵字符串）
pub fn validate_team_package(
    root_dir: &Path,
    team_toml: &TeamToml,
    options: &ValidationOptions,
) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. Schema 校验
    validate_schema(team_toml, &mut errors, &mut warnings);

    // 2. 资产引用校验
    if options.check_asset_files {
        validate_asset_refs(root_dir, team_toml, &mut errors, &mut warnings);
    }

    // 3. 策略冲突检测
    validate_policies(team_toml, &mut errors, &mut warnings);

    // 4. 安全扫描
    let security = if options.run_security_scan {
        match validate_team_package_security(root_dir) {
            Ok(report) => {
                if report.blocked {
                    errors.push(ValidationIssue {
                        code: ValidationCode::SecurityBlocked,
                        message: format!(
                            "security scan blocked: {} critical finding(s)",
                            report.findings.len()
                        ),
                        location: None,
                    });
                }
                Some(report)
            }
            Err(e) => {
                warnings.push(ValidationIssue {
                    code: ValidationCode::SecurityBlocked,
                    message: format!("security scan failed to run: {e}"),
                    location: None,
                });
                None
            }
        }
    } else {
        None
    };

    let passed = errors.is_empty()
        && security.as_ref().map(|s| !s.blocked).unwrap_or(true);

    ValidationReport {
        errors,
        warnings,
        security,
        passed,
    }
}

/// 便捷入口：从目录读取 team.toml 并校验
pub fn validate_team_package_dir(
    root_dir: &Path,
    options: &ValidationOptions,
) -> Result<ValidationReport, super::repository::ConnectError> {
    let team_toml_path = root_dir.join("team.toml");
    let content = std::fs::read_to_string(&team_toml_path)
        .map_err(|e| super::repository::ConnectError::TeamTomlReadError(e.to_string()))?;
    let team_toml = parse_team_toml(&content)
        .map_err(|e| super::repository::ConnectError::TeamTomlParseError(e.to_string()))?;
    Ok(validate_team_package(root_dir, &team_toml, options))
}

// ─── 内部校验函数 ─────────────────────────────────────────────────────────────

/// Schema 完整性校验
fn validate_schema(
    team_toml: &TeamToml,
    errors: &mut Vec<ValidationIssue>,
    warnings: &mut Vec<ValidationIssue>,
) {
    // [team] 元数据
    if team_toml.team.name.is_none() {
        warnings.push(ValidationIssue {
            code: ValidationCode::MissingTeamSection,
            message: "[team].name is not set".to_string(),
            location: Some("team.toml [team]".to_string()),
        });
    }

    // Profiles
    for (idx, profile) in team_toml.profiles.iter().enumerate() {
        if profile.id.is_empty() {
            errors.push(ValidationIssue {
                code: ValidationCode::MissingProfileId,
                message: format!("profiles[{idx}].id is empty"),
                location: Some(format!("team.toml [[profiles]] #{idx}")),
            });
        }
        if profile.name.is_empty() {
            errors.push(ValidationIssue {
                code: ValidationCode::MissingProfileName,
                message: format!("profiles[{idx}].name is empty"),
                location: Some(format!("team.toml [[profiles]] #{idx}")),
            });
        }

        // 资产引用
        for asset in &profile.assets {
            if AssetType::from_str(&asset.asset_type).is_none() {
                errors.push(ValidationIssue {
                    code: ValidationCode::InvalidAssetType,
                    message: format!(
                        "profiles[{idx}].assets: unknown asset type '{}'",
                        asset.asset_type
                    ),
                    location: Some(format!("asset '{}'", asset.id)),
                });
            }
            if asset.path.is_none() && asset.content.is_none() {
                errors.push(ValidationIssue {
                    code: ValidationCode::MissingAssetContentRef,
                    message: format!(
                        "asset '{}' has neither path nor content",
                        asset.id
                    ),
                    location: Some(format!("profiles[{idx}].assets")),
                });
            }
        }
    }

    // Policies
    for (idx, policy) in team_toml.policies.iter().enumerate() {
        if AssetType::from_str(&policy.asset_type).is_none() {
            errors.push(ValidationIssue {
                code: ValidationCode::InvalidAssetType,
                message: format!(
                    "policies[{idx}]: unknown asset type '{}'",
                    policy.asset_type
                ),
                location: Some(format!("team.toml [[policies]] #{idx}")),
            });
        }
        match policy.action.as_str() {
            "required" | "recommended" | "denied" => {}
            other => {
                errors.push(ValidationIssue {
                    code: ValidationCode::InvalidPolicyAction,
                    message: format!("policies[{idx}]: invalid action '{other}'"),
                    location: Some(format!("team.toml [[policies]] #{idx}")),
                });
            }
        }
    }
}

/// 资产引用校验：路径存在性 + 路径穿越检测
fn validate_asset_refs(
    root_dir: &Path,
    team_toml: &TeamToml,
    errors: &mut Vec<ValidationIssue>,
    _warnings: &mut Vec<ValidationIssue>,
) {
    for profile in &team_toml.profiles {
        for asset in &profile.assets {
            if let Some(path_str) = &asset.path {
                // 路径穿越检测
                if path_str.contains("..") || path_str.starts_with('/') || path_str.starts_with('\\') {
                    errors.push(ValidationIssue {
                        code: ValidationCode::PathTraversal,
                        message: format!(
                            "asset '{}': path '{}' may traverse outside package",
                            asset.id, path_str
                        ),
                        location: Some(path_str.clone()),
                    });
                    continue;
                }

                // 符号链接检测（安全红线：团队包内不允许 symlink）
                let asset_path = root_dir.join(path_str);
                if let Ok(meta) = std::fs::symlink_metadata(&asset_path) {
                    if meta.file_type().is_symlink() {
                        errors.push(ValidationIssue {
                            code: ValidationCode::PathTraversal,
                            message: format!(
                                "asset '{}': path '{}' is a symlink (not allowed in team packages)",
                                asset.id, path_str
                            ),
                            location: Some(path_str.clone()),
                        });
                        continue;
                    }
                }

                // 解析后路径必须仍在包根目录内
                if asset_path.exists() {
                    if let (Ok(resolved), Ok(root_resolved)) =
                        (asset_path.canonicalize(), root_dir.canonicalize())
                    {
                        if !resolved.starts_with(&root_resolved) {
                            errors.push(ValidationIssue {
                                code: ValidationCode::PathTraversal,
                                message: format!(
                                    "asset '{}': resolved path escapes package root",
                                    asset.id
                                ),
                                location: Some(path_str.clone()),
                            });
                            continue;
                        }
                    }
                }

                // 文件存在性
                if !asset_path.exists() {
                    errors.push(ValidationIssue {
                        code: ValidationCode::AssetFileNotFound,
                        message: format!(
                            "asset '{}': referenced file '{}' not found",
                            asset.id, path_str
                        ),
                        location: Some(path_str.clone()),
                    });
                }
            }
        }
    }
}

/// 策略冲突检测：同一 (asset_type, pattern) 同时 required 和 denied
fn validate_policies(
    team_toml: &TeamToml,
    errors: &mut Vec<ValidationIssue>,
    _warnings: &mut Vec<ValidationIssue>,
) {
    use std::collections::HashMap;

    let mut action_map: HashMap<(String, String), Vec<String>> = HashMap::new();

    for policy in &team_toml.policies {
        let key = (policy.asset_type.clone(), policy.pattern.clone());
        action_map.entry(key).or_default().push(policy.action.clone());
    }

    for ((asset_type, pattern), actions) in &action_map {
        let has_required = actions.iter().any(|a| a == "required");
        let has_denied = actions.iter().any(|a| a == "denied");
        if has_required && has_denied {
            errors.push(ValidationIssue {
                code: ValidationCode::PolicyContradiction,
                message: format!(
                    "policy contradiction: '{pattern}' ({asset_type}) is both required and denied"
                ),
                location: Some(format!("{asset_type}:{pattern}")),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_valid_package() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            r#"
[team]
name = "Valid Team"
version = "1.0.0"

[[profiles]]
id = "p1"
name = "Base"

[[profiles.assets]]
type = "prompt"
id = "main"
path = "prompts/main.md"

[[policies]]
type = "permission"
pattern = "Bash"
action = "denied"
"#,
        )
        .expect("write team.toml");
        fs::create_dir_all(dir.path().join("prompts")).expect("mkdir");
        fs::write(dir.path().join("prompts/main.md"), "# Main").expect("write");
        dir
    }

    #[test]
    fn validates_clean_package() {
        let dir = setup_valid_package();
        let options = ValidationOptions {
            run_security_scan: false, // 跳过安全扫描加速测试
            check_asset_files: true,
        };
        let report = validate_team_package_dir(dir.path(), &options).expect("validate");
        assert!(report.passed, "errors: {:?}", report.errors);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn detects_missing_asset_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            r#"
[team]
name = "T"
version = "1.0.0"

[[profiles]]
id = "p1"
name = "Base"

[[profiles.assets]]
type = "prompt"
id = "ghost"
path = "prompts/ghost.md"
"#,
        )
        .expect("write");
        let options = ValidationOptions {
            run_security_scan: false,
            check_asset_files: true,
        };
        let report = validate_team_package_dir(dir.path(), &options).expect("validate");
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.code == ValidationCode::AssetFileNotFound));
    }

    #[test]
    fn detects_path_traversal() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            r#"
[team]
name = "T"
version = "1.0.0"

[[profiles]]
id = "p1"
name = "Base"

[[profiles.assets]]
type = "prompt"
id = "evil"
path = "../../etc/passwd"
"#,
        )
        .expect("write");
        let options = ValidationOptions {
            run_security_scan: false,
            check_asset_files: true,
        };
        let report = validate_team_package_dir(dir.path(), &options).expect("validate");
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.code == ValidationCode::PathTraversal));
    }

    #[test]
    fn detects_policy_contradiction() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            r#"
[team]
name = "T"
version = "1.0.0"

[[policies]]
type = "permission"
pattern = "Bash"
action = "required"

[[policies]]
type = "permission"
pattern = "Bash"
action = "denied"
"#,
        )
        .expect("write");
        let options = ValidationOptions {
            run_security_scan: false,
            check_asset_files: false,
        };
        let report = validate_team_package_dir(dir.path(), &options).expect("validate");
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.code == ValidationCode::PolicyContradiction));
    }

    #[test]
    fn detects_invalid_asset_type() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            r#"
[team]
name = "T"
version = "1.0.0"

[[profiles]]
id = "p1"
name = "Base"

[[profiles.assets]]
type = "unknown_thing"
id = "x"
path = "x.md"
"#,
        )
        .expect("write");
        let options = ValidationOptions {
            run_security_scan: false,
            check_asset_files: false,
        };
        let report = validate_team_package_dir(dir.path(), &options).expect("validate");
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.code == ValidationCode::InvalidAssetType));
    }

    #[test]
    fn security_scan_blocks_plaintext_secrets() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            "[team]\nname = \"T\"\nversion = \"1.0.0\"\n",
        )
        .expect("write");
        fs::write(
            dir.path().join("secret.env"),
            "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890",
        )
        .expect("write secret");

        let options = ValidationOptions {
            run_security_scan: true,
            check_asset_files: false,
        };
        let report = validate_team_package_dir(dir.path(), &options).expect("validate");
        assert!(!report.passed);
        assert!(report.security.is_some());
        assert!(report.security.unwrap().blocked);
    }
}
