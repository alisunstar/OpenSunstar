//! Simple Connect 备份目录安全扫描（Phase 3 P1）

use crate::error::AppError;
use crate::services::simple_connect::backup::simple_connect_backup_root;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct BackupAuditItem {
    pub path: String,
    pub suspicious: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupAuditReport {
    pub files_scanned: usize,
    pub suspicious_count: usize,
    pub items: Vec<BackupAuditItem>,
    pub all_clean: bool,
}

fn file_looks_like_secret(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.len() >= 20 && trimmed.contains("sk-") {
            return Some(format!("{} 可能含明文 Key", path.display()));
        }
    }
    None
}

fn walk_audit(dir: &Path, items: &mut Vec<BackupAuditItem>, files: &mut usize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_audit(&path, items, files);
        } else if path.is_file() {
            *files += 1;
            let suspicious = file_looks_like_secret(&path);
            items.push(BackupAuditItem {
                path: path.display().to_string(),
                suspicious: suspicious.is_some(),
                detail: suspicious.unwrap_or_else(|| "OK".into()),
            });
        }
    }
}

pub fn run_backup_audit() -> Result<BackupAuditReport, AppError> {
    let root = simple_connect_backup_root();
    let mut items = Vec::new();
    let mut files_scanned = 0usize;

    if root.exists() {
        walk_audit(&root, &mut items, &mut files_scanned);
    }

    let suspicious_count = items.iter().filter(|i| i.suspicious).count();
    Ok(BackupAuditReport {
        files_scanned,
        suspicious_count,
        all_clean: suspicious_count == 0,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_sk_in_backup_content() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("settings.json");
        fs::write(&file, r#"{"token":"sk-test-secret-key-12345678"}"#).unwrap();
        assert!(file_looks_like_secret(&file).is_some());
    }

    #[test]
    fn clean_file_passes() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("settings.json");
        fs::write(&file, r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sc-local-abc"}}"#).unwrap();
        assert!(file_looks_like_secret(&file).is_none());
    }
}
