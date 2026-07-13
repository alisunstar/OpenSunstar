//! Gemini CLI 写入

use super::shared::{
    ensure_object, normalize_base, read_json_or_empty, write_json_pretty, write_text,
    StatusOutcome, WriteOutcome, MANAGED_MARKER,
};
use crate::error::AppError;
use crate::gemini_config::{get_gemini_dir, get_gemini_env_path};
use serde_json::Value;

pub fn paths() -> Vec<std::path::PathBuf> {
    vec![
        get_gemini_dir().join("settings.json"),
        get_gemini_env_path(),
    ]
}

pub fn apply(api_key: &str, model: &str, openai_base: &str) -> Result<WriteOutcome, AppError> {
    let (root, openai) = normalize_base(openai_base);
    let settings = get_gemini_dir().join("settings.json");
    let env_path = get_gemini_env_path();

    if let Some(parent) = settings.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let mut value = read_json_or_empty(&settings)?;
    let obj = ensure_object(&mut value)?;
    obj.insert(
        "selectedAuthType".into(),
        Value::String("gemini-api-key".into()),
    );
    obj.insert("model".into(), Value::String(model.trim().to_string()));
    obj.insert(
        "simpleConnectManaged".into(),
        Value::String(MANAGED_MARKER.into()),
    );
    write_json_pretty(&settings, &value)?;

    let env_contents = format!(
        "# Managed by OpenSunstar Simple Connect - {MANAGED_MARKER}\n\
         GEMINI_API_KEY={key}\n\
         GOOGLE_API_KEY={key}\n\
         GOOGLE_GEMINI_BASE_URL={root}\n\
         OPENAI_API_KEY={key}\n\
         OPENAI_BASE_URL={openai}\n",
        key = api_key.trim(),
    );
    write_text(&env_path, &env_contents)?;

    Ok(WriteOutcome {
        files: vec![settings, env_path],
    })
}

pub fn status() -> Result<StatusOutcome, AppError> {
    let env_path = get_gemini_env_path();
    if !env_path.exists() {
        return Ok(StatusOutcome::empty());
    }
    let text = std::fs::read_to_string(&env_path).unwrap_or_default();
    let configured = text.contains(MANAGED_MARKER) || text.contains("127.0.0.1");
    let base_url = text
        .lines()
        .find_map(|l| l.strip_prefix("GOOGLE_GEMINI_BASE_URL="))
        .map(|s| s.trim().to_string());
    let key = text
        .lines()
        .find_map(|l| l.strip_prefix("GEMINI_API_KEY="))
        .map(|s| s.trim().to_string());
    let settings = get_gemini_dir().join("settings.json");
    let model = if settings.exists() {
        read_json_or_empty(&settings)
            .ok()
            .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
    } else {
        None
    };
    let managed = settings
        .exists()
        .then(|| read_json_or_empty(&settings).ok())
        .flatten()
        .and_then(|v| {
            v.get("simpleConnectManaged")
                .and_then(|x| x.as_str())
                .map(String::from)
        });
    Ok(StatusOutcome {
        configured: configured
            || managed
                .as_deref()
                .map(super::shared::is_managed_value)
                .unwrap_or(false),
        base_url,
        model,
        key,
        ..StatusOutcome::empty()
    })
}

pub fn clear() -> Result<(), AppError> {
    let env_path = get_gemini_env_path();
    if env_path.exists() {
        if let Ok(t) = std::fs::read_to_string(&env_path) {
            if t.contains(MANAGED_MARKER) {
                let _ = std::fs::remove_file(&env_path);
            }
        }
    }
    let settings = get_gemini_dir().join("settings.json");
    if settings.exists() {
        let mut value = read_json_or_empty(&settings)?;
        if let Some(obj) = value.as_object_mut() {
            obj.remove("simpleConnectManaged");
        }
        write_json_pretty(&settings, &value)?;
    }
    Ok(())
}
