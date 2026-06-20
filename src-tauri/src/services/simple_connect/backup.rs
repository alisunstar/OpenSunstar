//! 写入 CLI 配置前备份（Simple Connect 专用目录）

use crate::error::AppError;
use chrono::Local;
use std::path::{Path, PathBuf};

pub fn simple_connect_backup_root() -> PathBuf {
    crate::config::get_app_config_dir()
        .join("simple-connect")
        .join("backups")
}

pub fn backup_file(tool: &str, src: &Path) -> Result<Option<PathBuf>, AppError> {
    if !src.exists() {
        return Ok(None);
    }
    let root = simple_connect_backup_root().join(tool);
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let dir = root.join(stamp);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::io(&dir, e))?;
    let name = src
        .file_name()
        .ok_or_else(|| AppError::Message("无法读取备份文件名".into()))?;
    let dst = dir.join(name);
    std::fs::copy(src, &dst).map_err(|e| AppError::io(&dst, e))?;
    Ok(Some(dst))
}
