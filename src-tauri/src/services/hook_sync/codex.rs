use std::path::Path;

use crate::codex_config::{get_codex_config_path, read_codex_config_text, write_codex_live_config_atomic};
use crate::config::write_text_file;
use crate::error::AppError;
use crate::hook::Hook;

pub fn sync_hooks(hooks: &[Hook]) -> Result<(), AppError> {
    sync_hooks_at_path(hooks, &get_codex_config_path())
}

pub fn sync_hooks_at_path(hooks: &[Hook], config_path: &Path) -> Result<(), AppError> {
    let existing = if config_path.is_file() {
        std::fs::read_to_string(config_path).unwrap_or_default()
    } else {
        read_codex_config_text().unwrap_or_default()
    };
    let stripped = strip_codex_hooks_sections(&existing);
    let block = build_codex_hooks_toml(hooks);
    let merged = merge_codex_config(&stripped, &block);
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    if merged.trim().is_empty() {
        if config_path.exists() {
            write_text_file(config_path, "")?;
        }
        return Ok(());
    }
    if config_path == get_codex_config_path() {
        write_codex_live_config_atomic(Some(&merged))
    } else {
        write_text_file(config_path, &merged)
    }
}

pub(crate) fn build_codex_hooks_toml(hooks: &[Hook]) -> String {
    let mut out = String::new();
    for hook in hooks {
        out.push_str(&format!(
            "\n[[hooks.{}]]\nmatcher = {:?}\ncommand = {:?}\ntimeout = {}\n",
            hook.event_type, hook.tool_pattern, hook.hook_command, hook.timeout_seconds
        ));
    }
    out
}

fn strip_codex_hooks_sections(toml: &str) -> String {
    let mut result = String::new();
    let mut skip = false;
    for line in toml.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[hooks.") {
            skip = true;
            continue;
        }
        if skip {
            if trimmed.starts_with("[[") || (trimmed.starts_with('[') && !trimmed.starts_with("[[")) {
                skip = false;
            } else {
                continue;
            }
        }
        if !result.is_empty() || !line.is_empty() {
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim_end().to_string()
}

fn merge_codex_config(base: &str, hooks_block: &str) -> String {
    let base = base.trim();
    let block = hooks_block.trim();
    if base.is_empty() {
        return block.to_string();
    }
    if block.is_empty() {
        return base.to_string();
    }
    let mut merged = base.to_string();
    if !merged.ends_with('\n') {
        merged.push('\n');
    }
    merged.push_str(block);
    if !merged.ends_with('\n') {
        merged.push('\n');
    }
    merged
}
