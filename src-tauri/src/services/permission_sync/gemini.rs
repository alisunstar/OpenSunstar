use std::path::Path;

use serde_json::{json, Value};

use crate::config::{read_json_file, write_json_file};
use crate::error::AppError;
use crate::gemini_config::get_gemini_settings_path;
use crate::services::permission_sync::PermissionLists;

pub fn sync_permissions(lists: &PermissionLists) -> Result<(), AppError> {
    sync_permissions_at_path(lists, &get_gemini_settings_path())
}

pub fn sync_permissions_at_path(
    lists: &PermissionLists,
    settings_path: &Path,
) -> Result<(), AppError> {
    let mut settings: Value = if settings_path.exists() {
        read_json_file(settings_path).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let tools = json!({
        "core": lists.allow,
        "allowed": lists.auto_approve,
        "exclude": lists.deny
    });

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    if lists.allow.is_empty() && lists.auto_approve.is_empty() && lists.deny.is_empty() {
        if let Some(obj) = settings.as_object_mut() {
            obj.remove("tools");
        }
    } else {
        settings["tools"] = tools;
    }

    write_json_file(settings_path, &settings)
}
