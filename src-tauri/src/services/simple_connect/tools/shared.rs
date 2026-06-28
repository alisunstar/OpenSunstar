//! Simple Connect CLI 写入共享 helpers（对标 beeapi-switch tools.rs）

use crate::error::AppError;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const MANAGED_MARKER: &str = "opensunstar-simple-connect";
pub const SC_PROVIDER_ID: &str = "simple-connect";

pub struct WriteOutcome {
    pub files: Vec<PathBuf>,
}

pub struct StatusOutcome {
    pub configured: bool,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub key: Option<String>,
    pub wire_api: Option<String>,
}

impl StatusOutcome {
    pub fn empty() -> Self {
        Self {
            configured: false,
            base_url: None,
            model: None,
            key: None,
            wire_api: None,
        }
    }
}

pub fn normalize_base(base: &str) -> (String, String) {
    let trimmed = base.trim().trim_end_matches('/').to_string();
    if trimmed.ends_with("/v1") {
        let root = trimmed.trim_end_matches("/v1").to_string();
        (root, trimmed)
    } else {
        (trimmed.clone(), format!("{trimmed}/v1"))
    }
}

pub fn read_json_or_empty(path: &Path) -> Result<Value, AppError> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    crate::config::read_json_file(path)
}

pub fn ensure_object(v: &mut Value) -> Result<&mut Map<String, Value>, AppError> {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
    v.as_object_mut()
        .ok_or_else(|| AppError::Message("JSON 根节点必须是对象".into()))
}

pub fn write_json_pretty(path: &Path, value: &Value) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    crate::config::write_json_file(path, value)
}

pub fn write_text(path: &Path, text: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    crate::config::write_text_file(path, text)
}

pub fn is_managed_value(value: &str) -> bool {
    value == MANAGED_MARKER
}

/// 判断 CLI 配置是否被 Simple Connect 托管。
///
/// **仅依赖显式托管标记**（MANAGED_MARKER），不再以 base 地址兜底。
/// 此前用 `127.0.0.1`/`localhost` 兜底会导致主代理模块（Expert Proxy 同样
/// 操作 ANTHROPIC_*）被误判为 Simple Connect 托管。
pub fn configured_by_marker(managed: Option<&str>, _base: Option<&str>) -> bool {
    managed.map(is_managed_value).unwrap_or(false)
}
