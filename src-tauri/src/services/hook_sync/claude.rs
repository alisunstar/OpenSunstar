use std::path::Path;

use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::hook::Hook;
use crate::services::claude_settings::ClaudeSettingsMerger;

pub fn sync_hooks(hooks: &[Hook]) -> Result<(), AppError> {
    sync_hooks_at_path(hooks, &crate::config::get_claude_settings_path())
}

pub fn sync_hooks_at_path(hooks: &[Hook], settings_path: &Path) -> Result<(), AppError> {
    let mut hooks_map: Map<String, Value> = Map::new();
    for hook in hooks {
        let entry = json!({
            "matcher": hook.tool_pattern,
            "hooks": [{
                "type": "command",
                "command": hook.hook_command,
                "timeout": hook.timeout_seconds
            }]
        });
        hooks_map
            .entry(hook.event_type.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("hooks array")
            .push(entry);
    }

    if hooks_map.is_empty() {
        ClaudeSettingsMerger::remove_field_at_path(settings_path, "hooks")
    } else {
        ClaudeSettingsMerger::update_field_at_path(
            settings_path,
            "hooks",
            Value::Object(hooks_map),
        )
    }
}
