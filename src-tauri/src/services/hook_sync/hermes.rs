use std::path::Path;

use serde_yaml::{Mapping, Value as YamlValue};

use crate::error::AppError;
use crate::hermes_config::{get_hermes_config_path, write_config_section_at_path};
use crate::hook::Hook;

fn map_hermes_event(event_type: &str) -> Option<&'static str> {
    match event_type {
        "PreToolUse" => Some("pre_tool_call"),
        "PostToolUse" => Some("post_tool_call"),
        "Stop" => Some("agent_end"),
        _ => None,
    }
}

fn build_hermes_hooks_section(hooks: &[Hook]) -> Mapping {
    let mut section = Mapping::new();

    for hook in hooks {
        let Some(event_key) = map_hermes_event(&hook.event_type) else {
            continue;
        };
        let mut entry = Mapping::new();
        entry.insert(
            YamlValue::String("command".into()),
            YamlValue::String(hook.hook_command.clone()),
        );
        let mut when = Mapping::new();
        when.insert(
            YamlValue::String("tools".into()),
            YamlValue::Sequence(vec![YamlValue::String(hook.tool_pattern.clone())]),
        );
        entry.insert(YamlValue::String("when".into()), YamlValue::Mapping(when));

        let key = YamlValue::String(event_key.to_string());
        let seq = match section.get(&key) {
            Some(YamlValue::Sequence(items)) => {
                let mut items = items.clone();
                items.push(YamlValue::Mapping(entry));
                items
            }
            _ => vec![YamlValue::Mapping(entry)],
        };
        section.insert(key, YamlValue::Sequence(seq));
    }

    section
}

pub fn sync_hooks(hooks: &[Hook]) -> Result<(), AppError> {
    sync_hooks_at_path(hooks, &get_hermes_config_path())
}

pub fn sync_hooks_at_path(hooks: &[Hook], config_path: &Path) -> Result<(), AppError> {
    let section = build_hermes_hooks_section(hooks);
    let value = if section.is_empty() {
        YamlValue::Null
    } else {
        YamlValue::Mapping(section)
    };
    write_config_section_at_path(config_path, "hooks", &value)
}
