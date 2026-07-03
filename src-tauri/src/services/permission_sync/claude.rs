use std::path::Path;

use crate::error::AppError;
use crate::services::claude_settings::ClaudeSettingsMerger;
use crate::services::permission_sync::PermissionLists;
use serde_json::json;

pub fn sync_permissions(lists: &PermissionLists) -> Result<(), AppError> {
    sync_permissions_at_path(lists, &crate::config::get_claude_settings_path())
}

pub fn sync_permissions_at_path(
    lists: &PermissionLists,
    settings_path: &Path,
) -> Result<(), AppError> {
    let mut allow = lists.allow.clone();
    allow.extend(lists.auto_approve.clone());
    allow.sort();
    allow.dedup();

    if allow.is_empty() && lists.deny.is_empty() {
        ClaudeSettingsMerger::remove_field_at_path(settings_path, "permissions")
    } else {
        let permissions = json!({
            "allow": allow,
            "deny": lists.deny,
            "additionalDirectories": []
        });
        ClaudeSettingsMerger::update_field_at_path(settings_path, "permissions", permissions)
    }
}
