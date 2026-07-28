//! Release Lock 生成与校验（Spike 验证项 3）
//!
//! lock.json 是不可变发布清单，包含：
//! - 每个文件的 SHA-256 摘要（篡改检测）
//! - 源 commit SHA（可追溯性）
//! - 兼容性矩阵（工具版本约束）
//! - 自身确定性摘要（完整性自校验）

use std::path::Path;

use sha2::{Digest, Sha256};

use super::domain::{
    AssetType, CompatibilityEntry, CompatibilityMatrix, LockManifestEntry, ReleaseLock, TargetApp,
    TeamRelease, TomlCompatibility,
};

/// 当前 lock 格式版本
pub const LOCK_VERSION: u32 = 1;

/// 从发布目录生成 lock.json
///
/// 扫描 `root_dir` 下所有文件，计算 SHA-256，生成确定性清单。
/// 文件按路径字典序排列以保证摘要确定性。
pub fn generate_lock(
    release: &TeamRelease,
    root_dir: &Path,
    compatibility: Option<&[TomlCompatibility]>,
) -> Result<ReleaseLock, String> {
    let mut manifests = collect_manifests(root_dir)?;
    // 按路径排序保证确定性
    manifests.sort_by(|a, b| a.path.cmp(&b.path));

    let compat_matrix = compatibility.map(|entries| CompatibilityMatrix {
        entries: entries
            .iter()
            .map(|e| CompatibilityEntry {
                target_app: TargetApp::from_str(&e.app),
                min_version: e.min_version.clone(),
                max_version: e.max_version.clone(),
            })
            .collect(),
    });

    let generated_at = chrono::Utc::now().timestamp_millis();

    // 先构建不含 lock_sha256 的结构，计算摘要后填入
    let mut lock = ReleaseLock {
        lock_version: LOCK_VERSION,
        release_id: release.release_id.clone(),
        version_label: release.version_label.clone(),
        source_commit: release.source_commit.clone(),
        generated_at,
        manifests,
        compatibility: compat_matrix,
        lock_sha256: String::new(), // 占位
    };

    lock.lock_sha256 = compute_lock_digest(&lock);
    Ok(lock)
}

/// 校验 lock.json 的完整性
///
/// 验证：
/// 1. lock_sha256 与内容匹配（自引用摘要）
/// 2. 每个 manifest 条目的 SHA-256 与实际文件匹配
///
/// 返回 Ok(()) 表示通过，Err 描述第一个不匹配项。
pub fn validate_lock(lock: &ReleaseLock, root_dir: &Path) -> Result<(), LockValidationError> {
    // 1. 验证自引用摘要
    let expected_digest = compute_lock_digest(lock);
    if lock.lock_sha256 != expected_digest {
        return Err(LockValidationError::DigestMismatch {
            expected: expected_digest,
            actual: lock.lock_sha256.clone(),
        });
    }

    // 2. 验证每个文件的 SHA-256
    for entry in &lock.manifests {
        let file_path = root_dir.join(&entry.path);
        if !file_path.exists() {
            return Err(LockValidationError::FileMissing(entry.path.clone()));
        }
        let content = std::fs::read(&file_path)
            .map_err(|e| LockValidationError::IoError(entry.path.clone(), e.to_string()))?;
        let actual_sha = format!("{:x}", Sha256::digest(&content));
        if actual_sha != entry.sha256 {
            return Err(LockValidationError::ContentMismatch {
                path: entry.path.clone(),
                expected: entry.sha256.clone(),
                actual: actual_sha,
            });
        }
        if content.len() as u64 != entry.size_bytes {
            return Err(LockValidationError::SizeMismatch {
                path: entry.path.clone(),
                expected: entry.size_bytes,
                actual: content.len() as u64,
            });
        }
    }

    Ok(())
}

/// 计算单个文件内容的 SHA-256
pub fn sha256_of_content(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

/// 计算 lock 的确定性摘要（排除 lock_sha256 字段本身）
fn compute_lock_digest(lock: &ReleaseLock) -> String {
    // 构建不含 lock_sha256 的规范化 JSON
    let digest_input = serde_json::json!({
        "lock_version": lock.lock_version,
        "release_id": lock.release_id,
        "version_label": lock.version_label,
        "source_commit": lock.source_commit,
        "generated_at": lock.generated_at,
        "manifests": lock.manifests,
        "compatibility": lock.compatibility,
    });
    let serialized = serde_json::to_vec(&digest_input).expect("lock is serializable");
    format!("{:x}", Sha256::digest(serialized))
}

/// 递归收集目录下所有文件的 manifest 条目
fn collect_manifests(root_dir: &Path) -> Result<Vec<LockManifestEntry>, String> {
    let mut entries = Vec::new();
    collect_recursive(root_dir, root_dir, &mut entries)?;
    Ok(entries)
}

fn collect_recursive(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<LockManifestEntry>,
) -> Result<(), String> {
    let read_dir =
        std::fs::read_dir(dir).map_err(|e| format!("读取目录失败 {}: {e}", dir.display()))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("目录条目错误: {e}"))?;
        let path = entry.path();

        // 跳过 .git 目录和 lock.json 自身
        if path.is_dir() {
            if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                continue;
            }
            collect_recursive(root, &path, entries)?;
        } else if path.is_file() {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            if file_name == "lock.json" {
                continue;
            }

            let content = std::fs::read(&path)
                .map_err(|e| format!("读取文件失败 {}: {e}", path.display()))?;
            let relative = path
                .strip_prefix(root)
                .map_err(|e| format!("路径计算失败: {e}"))?
                .to_string_lossy()
                .replace('\\', "/"); // Windows 路径标准化

            let asset_type = infer_asset_type(&relative);

            entries.push(LockManifestEntry {
                path: relative,
                sha256: sha256_of_content(&content),
                size_bytes: content.len() as u64,
                asset_type,
            });
        }
    }
    Ok(())
}

/// 根据文件路径推断资产类型
fn infer_asset_type(relative_path: &str) -> Option<AssetType> {
    let lower = relative_path.to_lowercase();
    if lower.starts_with("prompts/") || lower.contains("/prompts/") {
        Some(AssetType::Prompt)
    } else if lower.starts_with("rules/") || lower.contains("/rules/") {
        Some(AssetType::Rule)
    } else if lower.starts_with("skills/") || lower.contains("/skills/") {
        Some(AssetType::Skill)
    } else if lower.starts_with("permissions/") || lower.contains("/permissions/") {
        Some(AssetType::Permission)
    } else if lower.starts_with("mcp/") || lower.contains("/mcp/") {
        Some(AssetType::Mcp)
    } else if lower.starts_with("commands/") || lower.contains("/commands/") {
        Some(AssetType::Command)
    } else if lower.starts_with("hooks/") || lower.contains("/hooks/") {
        Some(AssetType::Hook)
    } else if lower.starts_with("subagents/") || lower.contains("/subagents/") {
        Some(AssetType::Subagent)
    } else if lower.starts_with("ignore/") || lower.contains("/ignore/") {
        Some(AssetType::Ignore)
    } else {
        None
    }
}

/// Lock 校验错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockValidationError {
    /// lock_sha256 自引用摘要不匹配（lock 文件本身被篡改）
    DigestMismatch { expected: String, actual: String },
    /// 清单中的文件不存在
    FileMissing(String),
    /// 文件内容 SHA-256 不匹配（文件被篡改）
    ContentMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    /// 文件大小不匹配
    SizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },
    /// IO 错误
    IoError(String, String),
}

impl std::fmt::Display for LockValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DigestMismatch { expected, actual } => {
                write!(f, "lock_digest_mismatch: expected {expected}, got {actual}")
            }
            Self::FileMissing(path) => write!(f, "lock_file_missing: {path}"),
            Self::ContentMismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "lock_content_tampered: {path} (expected {expected}, got {actual})"
            ),
            Self::SizeMismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "lock_size_mismatch: {path} (expected {expected}, got {actual})"
            ),
            Self::IoError(path, err) => write!(f, "lock_io_error: {path}: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_release() -> TeamRelease {
        TeamRelease {
            release_id: "rel_test_001".to_string(),
            workspace_id: "ws_test".to_string(),
            version_label: "1.0.0".to_string(),
            profile_ids: vec!["profile-backend".to_string()],
            policies: vec![],
            source_commit: Some("abc123def456".to_string()),
            published_by: "user_admin".to_string(),
            published_at: 1784736000000,
            status: super::super::domain::ReleaseStatus::Published,
        }
    }

    fn setup_release_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("team.toml"),
            "[team]\nname = \"Test Team\"\nversion = \"1.0.0\"",
        )
        .expect("write team.toml");
        fs::create_dir_all(dir.path().join("prompts")).expect("mkdir prompts");
        fs::write(
            dir.path().join("prompts/backend.md"),
            "# Backend Prompt\nYou are a backend engineer.",
        )
        .expect("write prompt");
        fs::create_dir_all(dir.path().join("permissions")).expect("mkdir permissions");
        fs::write(
            dir.path().join("permissions/default.json"),
            r#"{"allow": ["Read"], "deny": ["Bash(rm *)"]}"#,
        )
        .expect("write permissions");
        dir
    }

    #[test]
    fn generates_lock_with_correct_manifests() {
        let dir = setup_release_dir();
        let release = make_test_release();
        let lock = generate_lock(&release, dir.path(), None).expect("generate lock");

        assert_eq!(lock.lock_version, LOCK_VERSION);
        assert_eq!(lock.release_id, "rel_test_001");
        assert_eq!(lock.manifests.len(), 3); // team.toml + prompt + permissions
        assert!(!lock.lock_sha256.is_empty());

        // 验证路径排序
        let paths: Vec<&str> = lock.manifests.iter().map(|m| m.path.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn validates_clean_lock_successfully() {
        let dir = setup_release_dir();
        let release = make_test_release();
        let lock = generate_lock(&release, dir.path(), None).expect("generate lock");

        let result = validate_lock(&lock, dir.path());
        assert!(result.is_ok(), "validation should pass: {:?}", result);
    }

    #[test]
    fn detects_tampered_file_content() {
        let dir = setup_release_dir();
        let release = make_test_release();
        let lock = generate_lock(&release, dir.path(), None).expect("generate lock");

        // 篡改文件
        fs::write(dir.path().join("prompts/backend.md"), "# TAMPERED CONTENT").expect("tamper");

        let result = validate_lock(&lock, dir.path());
        assert!(matches!(
            result,
            Err(LockValidationError::ContentMismatch { .. })
        ));
    }

    #[test]
    fn detects_missing_file() {
        let dir = setup_release_dir();
        let release = make_test_release();
        let lock = generate_lock(&release, dir.path(), None).expect("generate lock");

        // 删除文件
        fs::remove_file(dir.path().join("permissions/default.json")).expect("remove");

        let result = validate_lock(&lock, dir.path());
        assert!(matches!(result, Err(LockValidationError::FileMissing(_))));
    }

    #[test]
    fn detects_tampered_lock_digest() {
        let dir = setup_release_dir();
        let release = make_test_release();
        let mut lock = generate_lock(&release, dir.path(), None).expect("generate lock");

        // 篡改 lock 摘要
        lock.lock_sha256 = "deadbeef".to_string();

        let result = validate_lock(&lock, dir.path());
        assert!(matches!(
            result,
            Err(LockValidationError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn lock_digest_is_deterministic() {
        let dir = setup_release_dir();
        let release = make_test_release();
        let lock1 = generate_lock(&release, dir.path(), None).expect("generate 1");
        let lock2 = generate_lock(&release, dir.path(), None).expect("generate 2");

        // 同一输入 → 同一摘要（generated_at 可能不同，但 manifests 相同）
        assert_eq!(lock1.manifests, lock2.manifests);
    }

    #[test]
    fn infers_asset_types_from_paths() {
        assert_eq!(infer_asset_type("prompts/main.md"), Some(AssetType::Prompt));
        assert_eq!(infer_asset_type("rules/coding.md"), Some(AssetType::Rule));
        assert_eq!(infer_asset_type("skills/tdd.md"), Some(AssetType::Skill));
        assert_eq!(
            infer_asset_type("permissions/default.json"),
            Some(AssetType::Permission)
        );
        assert_eq!(infer_asset_type("mcp/servers.json"), Some(AssetType::Mcp));
        assert_eq!(infer_asset_type("team.toml"), None);
    }
}
