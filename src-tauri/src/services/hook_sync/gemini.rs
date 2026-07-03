use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::{read_json_file, write_json_file};
use crate::error::AppError;
use crate::gemini_config::get_gemini_settings_path;
use crate::hook::Hook;

fn map_gemini_event(event_type: &str) -> Option<&'static str> {
    match event_type {
        "PreToolUse" => Some("BeforeTool"),
        "PostToolUse" => Some("AfterTool"),
        "Stop" => Some("SessionEnd"),
        _ => None,
    }
}

pub fn sync_hooks(hooks: &[Hook]) -> Result<(), AppError> {
    sync_hooks_at_path(hooks, &get_gemini_settings_path())
}

pub fn sync_hooks_at_path(hooks: &[Hook], settings_path: &Path) -> Result<(), AppError> {
    let mut settings: Value = if settings_path.exists() {
        read_json_file(settings_path).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let mut hooks_map: Map<String, Value> = Map::new();
    for hook in hooks {
        let Some(gemini_event) = map_gemini_event(&hook.event_type) else {
            continue;
        };
        let entry = json!({
            "matcher": hook.tool_pattern,
            "hooks": [{
                "type": "command",
                "command": hook.hook_command,
                "timeoutMs": hook.timeout_seconds * 1000
            }]
        });
        hooks_map
            .entry(gemini_event.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("hooks array")
            .push(entry);
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| crate::error::AppError::io(parent, e))?;
    }

    if hooks_map.is_empty() {
        if let Some(obj) = settings.as_object_mut() {
            obj.remove("hooks");
        }
    } else {
        settings["hooks"] = Value::Object(hooks_map);
    }

    write_json_file(settings_path, &settings)
}
