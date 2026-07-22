//! Security gate for team configuration imports and releases.
//!
//! Findings intentionally omit matched snippets. A security report is allowed
//! to cross IPC and enter logs, so it must never become a second secret leak.

use crate::audit::engine::{BlockThreshold, Severity};
use crate::audit::{scan_dir, AuditContext, AuditSource};
use crate::AppError;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MAX_SCANNED_FILE_SIZE: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPackageSecurityFinding {
    pub rule_id: String,
    pub severity: String,
    pub category: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPackageSecurityReport {
    pub files_scanned: usize,
    pub blocked: bool,
    pub findings: Vec<TeamPackageSecurityFinding>,
}

pub fn validate_team_package_security(
    package_dir: &Path,
) -> Result<TeamPackageSecurityReport, AppError> {
    if !package_dir.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "团队配置包目录不存在: {}",
            package_dir.display()
        )));
    }

    let audit = scan_dir(
        package_dir,
        &AuditContext {
            source: AuditSource::TeamRelease {
                release_id: "preflight".to_string(),
            },
            threshold: BlockThreshold::High,
        },
    )?;
    let mut findings = audit
        .findings
        .into_iter()
        .map(|finding| TeamPackageSecurityFinding {
            rule_id: finding.rule_id,
            severity: finding.severity.label().to_string(),
            category: finding.category,
            file: finding.file,
            line: finding.line,
            message: finding.message,
        })
        .collect::<Vec<_>>();

    let files = collect_text_files(package_dir)?;
    for file in &files {
        scan_sensitive_content(package_dir, file, &mut findings)?;
    }
    findings.sort_by(|left, right| {
        (&left.file, left.line, &left.rule_id).cmp(&(&right.file, right.line, &right.rule_id))
    });
    findings.dedup_by(|left, right| {
        left.file == right.file && left.line == right.line && left.rule_id == right.rule_id
    });
    let blocked = findings.iter().any(|finding| {
        finding.severity == Severity::Critical.label() || finding.severity == Severity::High.label()
    });

    Ok(TeamPackageSecurityReport {
        files_scanned: files.len(),
        blocked,
        findings,
    })
}

/// Mandatory import/release gate. The returned error contains counts only and
/// is therefore safe for logs and IPC error surfaces.
pub fn enforce_team_package_security(
    package_dir: &Path,
) -> Result<TeamPackageSecurityReport, AppError> {
    let report = validate_team_package_security(package_dir)?;
    if report.blocked {
        return Err(AppError::InvalidInput(format!(
            "team_package_security_blocked: {} finding(s)",
            report.findings.len()
        )));
    }
    Ok(report)
}

fn collect_text_files(root: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut files = Vec::new();
    collect_text_files_recursive(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_text_files_recursive(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(current).map_err(|error| AppError::io(current, error))? {
        let entry = entry.map_err(|error| AppError::io(current, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| AppError::io(&path, error))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let name = entry.file_name();
            if name != ".git" && name != "node_modules" {
                collect_text_files_recursive(root, &path, files)?;
            }
            continue;
        }
        if file_type.is_file()
            && entry
                .metadata()
                .map_err(|error| AppError::io(&path, error))?
                .len()
                <= MAX_SCANNED_FILE_SIZE
            && fs::read_to_string(&path).is_ok()
        {
            debug_assert!(path.starts_with(root));
            files.push(path);
        }
    }
    Ok(())
}

fn scan_sensitive_content(
    root: &Path,
    file: &Path,
    findings: &mut Vec<TeamPackageSecurityFinding>,
) -> Result<(), AppError> {
    let content = fs::read_to_string(file).map_err(|error| AppError::io(file, error))?;
    let relative = file
        .strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/");

    for (index, line) in content.lines().enumerate() {
        if private_key_pattern().is_match(line) {
            findings.push(team_finding(
                "team-private-key",
                &relative,
                index + 1,
                "检测到私钥正文；团队配置只能声明凭证槽位",
            ));
        }
        if let Some(captures) = sensitive_assignment_pattern().captures(line) {
            let value = captures
                .name("value")
                .map(|capture| capture.as_str())
                .unwrap_or("");
            if !is_safe_secret_placeholder(value) {
                findings.push(team_finding(
                    "team-sensitive-assignment",
                    &relative,
                    index + 1,
                    "检测到敏感字段明文；请改用凭证槽位或本机 Keychain 绑定",
                ));
            }
        }
    }
    Ok(())
}

fn team_finding(
    rule_id: &str,
    file: &str,
    line: usize,
    message: &str,
) -> TeamPackageSecurityFinding {
    TeamPackageSecurityFinding {
        rule_id: rule_id.to_string(),
        severity: Severity::Critical.label().to_string(),
        category: "team-secret".to_string(),
        file: file.to_string(),
        line,
        message: message.to_string(),
    }
}

fn private_key_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?i)-----BEGIN(?: [A-Z0-9]+)? PRIVATE KEY-----")
            .expect("private key regex is valid")
    })
}

fn sensitive_assignment_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r#"(?i)[\"']?[A-Z0-9_.-]*(?:API[_-]?KEY|TOKEN|SECRET|PASSWORD|PRIVATE[_-]?KEY)[A-Z0-9_.-]*[\"']?\s*[:=]\s*[\"']?(?P<value>[^\"',\s]+)"#,
        )
        .expect("sensitive assignment regex is valid")
    })
}

fn is_safe_secret_placeholder(raw: &str) -> bool {
    let value = raw.trim().trim_matches(['"', '\'']);
    value.is_empty()
        || (value.starts_with("${") && value.ends_with('}'))
        || value.starts_with("keychain://ref/")
        || value.starts_with("credential_slot://")
        || value.starts_with("slot://")
}
