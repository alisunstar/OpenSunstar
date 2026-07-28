//! Release Diff 引擎（Local Alpha L3）
//!
//! 比对两份 lock manifest（或 manifest vs 当前目录），
//! 产出文件级变更集：Added / Removed / Modified / Unchanged。

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use super::domain::{AssetType, LockManifestEntry, ReleaseLock};
use super::release::sha256_of_content;

/// 单个文件的变更类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffAction {
    Added,
    Removed,
    Modified,
    Unchanged,
}

/// 单个文件的 Diff 条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffEntry {
    pub path: String,
    pub action: DiffAction,
    pub asset_type: Option<AssetType>,
    /// 旧 SHA-256（Added 时为 None）
    pub old_sha256: Option<String>,
    /// 新 SHA-256（Removed 时为 None）
    pub new_sha256: Option<String>,
    /// 旧文件大小
    pub old_size: Option<u64>,
    /// 新文件大小
    pub new_size: Option<u64>,
}

/// 两份 manifest 的完整 Diff 结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseDiff {
    /// 基线标识（旧版本 release_id 或 "directory"）
    pub base_ref: String,
    /// 目标标识（新版本 release_id 或 "directory"）
    pub target_ref: String,
    pub added: Vec<DiffEntry>,
    pub removed: Vec<DiffEntry>,
    pub modified: Vec<DiffEntry>,
    pub unchanged_count: usize,
    /// 汇总统计
    pub summary: DiffSummary,
}

/// Diff 汇总统计
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummary {
    pub total_files_base: usize,
    pub total_files_target: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub unchanged_count: usize,
    /// 是否有任何变更
    pub has_changes: bool,
}

/// 比对两份 manifest，产出文件级变更集
///
/// `base` = 旧版本（基线），`target` = 新版本（目标）
pub fn diff_manifests(
    base_ref: &str,
    base: &[LockManifestEntry],
    target_ref: &str,
    target: &[LockManifestEntry],
) -> ReleaseDiff {
    let base_map: HashMap<&str, &LockManifestEntry> =
        base.iter().map(|e| (e.path.as_str(), e)).collect();
    let target_map: HashMap<&str, &LockManifestEntry> =
        target.iter().map(|e| (e.path.as_str(), e)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut unchanged_count = 0usize;

    // 遍历 target：新增 or 修改 or 不变
    for (path, target_entry) in &target_map {
        match base_map.get(path) {
            None => {
                added.push(DiffEntry {
                    path: path.to_string(),
                    action: DiffAction::Added,
                    asset_type: target_entry.asset_type.clone(),
                    old_sha256: None,
                    new_sha256: Some(target_entry.sha256.clone()),
                    old_size: None,
                    new_size: Some(target_entry.size_bytes),
                });
            }
            Some(base_entry) => {
                if base_entry.sha256 != target_entry.sha256 {
                    modified.push(DiffEntry {
                        path: path.to_string(),
                        action: DiffAction::Modified,
                        asset_type: target_entry.asset_type.clone(),
                        old_sha256: Some(base_entry.sha256.clone()),
                        new_sha256: Some(target_entry.sha256.clone()),
                        old_size: Some(base_entry.size_bytes),
                        new_size: Some(target_entry.size_bytes),
                    });
                } else {
                    unchanged_count += 1;
                }
            }
        }
    }

    // 遍历 base：在 target 中不存在 = 删除
    for (path, base_entry) in &base_map {
        if !target_map.contains_key(path) {
            removed.push(DiffEntry {
                path: path.to_string(),
                action: DiffAction::Removed,
                asset_type: base_entry.asset_type.clone(),
                old_sha256: Some(base_entry.sha256.clone()),
                new_sha256: None,
                old_size: Some(base_entry.size_bytes),
                new_size: None,
            });
        }
    }

    // 排序保证确定性输出
    added.sort_by(|a, b| a.path.cmp(&b.path));
    removed.sort_by(|a, b| a.path.cmp(&b.path));
    modified.sort_by(|a, b| a.path.cmp(&b.path));

    let has_changes = !added.is_empty() || !removed.is_empty() || !modified.is_empty();

    let summary = DiffSummary {
        total_files_base: base.len(),
        total_files_target: target.len(),
        added_count: added.len(),
        removed_count: removed.len(),
        modified_count: modified.len(),
        unchanged_count,
        has_changes,
    };

    ReleaseDiff {
        base_ref: base_ref.to_string(),
        target_ref: target_ref.to_string(),
        added,
        removed,
        modified,
        unchanged_count,
        summary,
    }
}

/// 比对 Release Lock 与当前目录状态
///
/// 扫描 `dir` 下所有文件计算 SHA-256，与 lock.manifests 比对。
/// 用于检测"自上次 Release 以来有哪些本地变更"。
pub fn diff_lock_vs_directory(lock: &ReleaseLock, dir: &Path) -> Result<ReleaseDiff, String> {
    let current_manifests = scan_directory_manifests(dir)?;
    Ok(diff_manifests(
        &lock.release_id,
        &lock.manifests,
        "working_directory",
        &current_manifests,
    ))
}

/// 比对两个 Release Lock
pub fn diff_two_locks(base: &ReleaseLock, target: &ReleaseLock) -> ReleaseDiff {
    diff_manifests(
        &base.release_id,
        &base.manifests,
        &target.release_id,
        &target.manifests,
    )
}

/// 扫描目录生成 manifest 列表（复用 release.rs 的逻辑模式）
fn scan_directory_manifests(root_dir: &Path) -> Result<Vec<LockManifestEntry>, String> {
    if !root_dir.is_dir() {
        return Err(format!("目录不存在: {}", root_dir.display()));
    }
    let mut entries = Vec::new();
    scan_recursive(root_dir, root_dir, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn scan_recursive(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<LockManifestEntry>,
) -> Result<(), String> {
    let read_dir =
        std::fs::read_dir(dir).map_err(|e| format!("读取目录失败 {}: {e}", dir.display()))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("目录条目错误: {e}"))?;
        let path = entry.path();

        if path.is_dir() {
            if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                continue;
            }
            scan_recursive(root, &path, entries)?;
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
                .replace('\\', "/");

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

/// 根据文件路径推断资产类型（与 release.rs 保持一致）
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn entry(path: &str, sha: &str, size: u64) -> LockManifestEntry {
        LockManifestEntry {
            path: path.to_string(),
            sha256: sha.to_string(),
            size_bytes: size,
            asset_type: None,
        }
    }

    #[test]
    fn detects_added_files() {
        let base = vec![entry("a.md", "aaa", 10)];
        let target = vec![entry("a.md", "aaa", 10), entry("b.md", "bbb", 20)];

        let diff = diff_manifests("v1", &base, "v2", &target);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].path, "b.md");
        assert_eq!(diff.added[0].action, DiffAction::Added);
        assert!(diff.summary.has_changes);
    }

    #[test]
    fn detects_removed_files() {
        let base = vec![entry("a.md", "aaa", 10), entry("b.md", "bbb", 20)];
        let target = vec![entry("a.md", "aaa", 10)];

        let diff = diff_manifests("v1", &base, "v2", &target);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].path, "b.md");
        assert_eq!(diff.removed[0].action, DiffAction::Removed);
    }

    #[test]
    fn detects_modified_files() {
        let base = vec![entry("a.md", "aaa", 10)];
        let target = vec![entry("a.md", "bbb", 15)];

        let diff = diff_manifests("v1", &base, "v2", &target);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].path, "a.md");
        assert_eq!(diff.modified[0].old_sha256, Some("aaa".to_string()));
        assert_eq!(diff.modified[0].new_sha256, Some("bbb".to_string()));
    }

    #[test]
    fn unchanged_files_counted_not_listed() {
        let base = vec![entry("a.md", "aaa", 10), entry("b.md", "bbb", 20)];
        let target = vec![entry("a.md", "aaa", 10), entry("b.md", "bbb", 20)];

        let diff = diff_manifests("v1", &base, "v2", &target);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
        assert_eq!(diff.unchanged_count, 2);
        assert!(!diff.summary.has_changes);
    }

    #[test]
    fn diff_lock_vs_directory_detects_local_changes() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("a.md"), "hello").expect("write");
        fs::write(dir.path().join("b.md"), "world").expect("write");

        // 构建一个 lock，其中 b.md 的 SHA 与实际不同
        let manifests = vec![
            LockManifestEntry {
                path: "a.md".to_string(),
                sha256: sha256_of_content(b"hello"),
                size_bytes: 5,
                asset_type: None,
            },
            LockManifestEntry {
                path: "b.md".to_string(),
                sha256: "stale_hash".to_string(),
                size_bytes: 5,
                asset_type: None,
            },
        ];
        let lock = ReleaseLock {
            lock_version: 1,
            release_id: "rel_001".to_string(),
            version_label: "1.0.0".to_string(),
            source_commit: None,
            generated_at: 0,
            manifests,
            compatibility: None,
            lock_sha256: "dummy".to_string(),
        };

        let diff = diff_lock_vs_directory(&lock, dir.path()).expect("diff");
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].path, "b.md");
        assert_eq!(diff.unchanged_count, 1);
    }

    #[test]
    fn diff_two_locks_cross_release() {
        let lock_a = ReleaseLock {
            lock_version: 1,
            release_id: "rel_a".to_string(),
            version_label: "1.0.0".to_string(),
            source_commit: None,
            generated_at: 0,
            manifests: vec![entry("x.md", "xxx", 5)],
            compatibility: None,
            lock_sha256: "a".to_string(),
        };
        let lock_b = ReleaseLock {
            lock_version: 1,
            release_id: "rel_b".to_string(),
            version_label: "2.0.0".to_string(),
            source_commit: None,
            generated_at: 0,
            manifests: vec![entry("x.md", "xxx", 5), entry("y.md", "yyy", 8)],
            compatibility: None,
            lock_sha256: "b".to_string(),
        };

        let diff = diff_two_locks(&lock_a, &lock_b);
        assert_eq!(diff.base_ref, "rel_a");
        assert_eq!(diff.target_ref, "rel_b");
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].path, "y.md");
    }

    #[test]
    fn output_is_deterministic() {
        let base = vec![entry("z.md", "zzz", 1), entry("a.md", "aaa", 2)];
        let target = vec![entry("m.md", "mmm", 3), entry("b.md", "bbb", 4)];

        let d1 = diff_manifests("v1", &base, "v2", &target);
        let d2 = diff_manifests("v1", &base, "v2", &target);
        assert_eq!(d1, d2);
        // 验证排序
        assert_eq!(d1.added[0].path, "b.md");
        assert_eq!(d1.added[1].path, "m.md");
        assert_eq!(d1.removed[0].path, "a.md");
        assert_eq!(d1.removed[1].path, "z.md");
    }
}
