use std::path::Path;

use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::opencode_config::{
    get_opencode_config_path, read_opencode_config_at, write_opencode_config_at,
};
use crate::services::permission_sync::PermissionLists;

pub fn parse_opencode_tool_pattern(pattern: &str) -> Option<(String, String)> {
    let trimmed = pattern.trim();
    if let Some(inner) = trimmed
        .strip_prefix("Bash(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return Some(("bash".into(), inner.trim().into()));
    }
    if let Some((tool, rest)) = trimmed.split_once('(') {
        if let Some(inner) = rest.strip_suffix(')') {
            return Some((tool.to_lowercase(), inner.trim().into()));
        }
    }
    Some((trimmed.to_lowercase(), "*".into()))
}

pub fn sync_permissions(lists: &PermissionLists) -> Result<(), AppError> {
    sync_permissions_at_path(lists, &get_opencode_config_path())
}

pub fn sync_permissions_at_path(
    lists: &PermissionLists,
    config_path: &Path,
) -> Result<(), AppError> {
    let mut config = read_opencode_config_at(config_path)?;
    let mut permission = Map::new();

    for pattern in &lists.allow {
        apply_pattern(&mut permission, pattern, "allow");
    }
    for pattern in &lists.auto_approve {
        apply_pattern(&mut permission, pattern, "allow");
    }
    for pattern in &lists.deny {
        apply_pattern(&mut permission, pattern, "deny");
    }

    if permission.is_empty() {
        if let Some(obj) = config.as_object_mut() {
            obj.remove("permission");
        }
    } else {
        config["permission"] = Value::Object(permission);
    }

    write_opencode_config_at(config_path, &config)
}

fn apply_pattern(permission: &mut Map<String, Value>, pattern: &str, decision: &str) {
    let Some((tool, sub)) = parse_opencode_tool_pattern(pattern) else {
        return;
    };
    let entry = permission
        .entry(tool.clone())
        .or_insert_with(|| json!({}));
    if let Some(obj) = entry.as_object_mut() {
        obj.insert(sub, json!(decision));
    }
}
