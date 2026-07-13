//! Claude Code CLI 写入

use super::shared::{
    configured_by_marker, ensure_object, read_json_or_empty, write_json_pretty, StatusOutcome,
    WriteOutcome, MANAGED_MARKER,
};
use crate::config::get_claude_settings_path;
use crate::error::AppError;
use serde_json::Value;

pub fn paths() -> Vec<std::path::PathBuf> {
    vec![get_claude_settings_path()]
}

pub fn apply(api_key: &str, model: &str, anthropic_base: &str) -> Result<WriteOutcome, AppError> {
    let key = api_key.trim();
    let model = model.trim();
    let base = anthropic_base.trim().trim_end_matches('/');

    if key.is_empty() {
        return Err(AppError::Message("API Key 不能为空".into()));
    }
    if model.is_empty() {
        return Err(AppError::Message("模型不能为空".into()));
    }
    if base.is_empty() {
        return Err(AppError::Message("Base URL 不能为空".into()));
    }

    let settings_path = get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let mut value = read_json_or_empty(&settings_path)?;
    let root_obj = ensure_object(&mut value)?;
    let env = root_obj
        .entry("env".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let env_obj = env
        .as_object_mut()
        .ok_or_else(|| AppError::Message("settings.json env 必须是对象".into()))?;

    env_obj.insert("ANTHROPIC_BASE_URL".into(), Value::String(base.to_string()));
    env_obj.insert(
        "ANTHROPIC_AUTH_TOKEN".into(),
        Value::String(key.to_string()),
    );
    env_obj.insert("ANTHROPIC_MODEL".into(), Value::String(model.to_string()));
    env_obj.insert(
        "ANTHROPIC_SMALL_FAST_MODEL".into(),
        Value::String(model.to_string()),
    );
    env_obj.insert(
        "OPEN_SUNSTAR_SIMPLE_CONNECT".into(),
        Value::String(MANAGED_MARKER.into()),
    );

    write_json_pretty(&settings_path, &value)?;

    Ok(WriteOutcome {
        files: vec![settings_path],
    })
}

pub fn status() -> Result<StatusOutcome, AppError> {
    let settings_path = get_claude_settings_path();
    if !settings_path.exists() {
        return Ok(StatusOutcome::empty());
    }
    let v: Value = read_json_or_empty(&settings_path)?;
    let env = v.get("env").and_then(|x| x.as_object());
    let base = env
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let model = env
        .and_then(|e| e.get("ANTHROPIC_MODEL"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let key = env
        .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let managed = env
        .and_then(|e| e.get("OPEN_SUNSTAR_SIMPLE_CONNECT"))
        .and_then(|x| x.as_str());
    let configured = configured_by_marker(managed, base.as_deref());
    Ok(StatusOutcome {
        configured,
        base_url: base,
        model,
        key,
        ..StatusOutcome::empty()
    })
}

pub fn clear() -> Result<(), AppError> {
    let settings_path = get_claude_settings_path();
    if !settings_path.exists() {
        return Ok(());
    }
    let mut value = read_json_or_empty(&settings_path)?;
    if let Some(obj) = value.as_object_mut() {
        if let Some(env) = obj.get_mut("env").and_then(|x| x.as_object_mut()) {
            for k in [
                "ANTHROPIC_BASE_URL",
                "ANTHROPIC_AUTH_TOKEN",
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_MODEL",
                "ANTHROPIC_SMALL_FAST_MODEL",
                "OPEN_SUNSTAR_SIMPLE_CONNECT",
            ] {
                env.remove(k);
            }
        }
    }
    write_json_pretty(&settings_path, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_marker_matches_shared() {
        assert_eq!(MANAGED_MARKER, "opensunstar-simple-connect");
    }
}
