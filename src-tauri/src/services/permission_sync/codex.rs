use std::path::Path;

use toml_edit::{Array, DocumentMut, Item, Value};

use crate::codex_config::{
    get_codex_config_path, read_codex_config_text, write_codex_live_config_atomic,
};
use crate::config::write_text_file;
use crate::error::AppError;
use crate::services::permission_sync::PermissionLists;

pub fn sync_permissions(lists: &PermissionLists) -> Result<(), AppError> {
    sync_permissions_at_path(lists, &get_codex_config_path())
}

pub fn sync_permissions_at_path(
    lists: &PermissionLists,
    config_path: &Path,
) -> Result<(), AppError> {
    let existing = if config_path.is_file() {
        std::fs::read_to_string(config_path).unwrap_or_default()
    } else {
        read_codex_config_text().unwrap_or_default()
    };
    let mut allow = lists.allow.clone();
    allow.extend(lists.auto_approve.clone());
    allow.sort();
    allow.dedup();

    let updated = set_codex_permission_arrays(&existing, &allow, &lists.deny)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    if config_path == get_codex_config_path() {
        write_codex_live_config_atomic(Some(&updated))
    } else {
        write_text_file(config_path, &updated)
    }
}

fn set_codex_permission_arrays(
    toml_str: &str,
    allow: &[String],
    deny: &[String],
) -> Result<String, AppError> {
    let mut doc = if toml_str.trim().is_empty() {
        DocumentMut::new()
    } else {
        toml_str
            .parse::<DocumentMut>()
            .map_err(|e| AppError::Config(format!("Codex TOML 解析失败: {e}")))?
    };

    set_string_array(&mut doc, "tool_allowlist", allow);
    set_string_array(&mut doc, "tool_denylist", deny);

    Ok(doc.to_string())
}

fn set_string_array(doc: &mut DocumentMut, key: &str, values: &[String]) {
    if values.is_empty() {
        doc.as_table_mut().remove(key);
        return;
    }
    let mut arr = Array::new();
    for v in values {
        arr.push(v.as_str());
    }
    doc[key] = Item::Value(Value::Array(arr));
}
