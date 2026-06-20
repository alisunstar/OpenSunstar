//! OpenCode CLI 写入（OpenAI-compatible provider）

use super::shared::{
    configured_by_marker, normalize_base, read_json_or_empty, WriteOutcome, MANAGED_MARKER,
    SC_PROVIDER_ID, StatusOutcome,
};
use crate::error::AppError;
use crate::opencode_config::{
    get_opencode_config_path, read_opencode_config, remove_provider, set_typed_provider,
    write_opencode_config,
};
use crate::provider::{OpenCodeModel, OpenCodeProviderConfig, OpenCodeProviderOptions};
use serde_json::Value;
use std::collections::HashMap;

pub fn paths() -> Vec<std::path::PathBuf> {
    vec![get_opencode_config_path()]
}

pub fn apply(api_key: &str, model: &str, openai_base: &str) -> Result<WriteOutcome, AppError> {
    let (_, openai) = normalize_base(openai_base);
    let model = model.trim();
    let mut models = HashMap::new();
    models.insert(
        model.to_string(),
        OpenCodeModel {
            name: model.to_string(),
            limit: None,
            options: None,
            extra: HashMap::new(),
        },
    );

    let config = OpenCodeProviderConfig {
        npm: "@ai-sdk/openai-compatible".into(),
        name: Some("Simple Connect".into()),
        options: OpenCodeProviderOptions {
            base_url: Some(openai),
            api_key: Some(api_key.trim().to_string()),
            ..Default::default()
        },
        models,
    };

    set_typed_provider(SC_PROVIDER_ID, &config)?;

    let mut full = read_opencode_config()?;
    if let Some(obj) = full.as_object_mut() {
        obj.insert(
            "model".into(),
            Value::String(format!("{SC_PROVIDER_ID}/{model}")),
        );
        obj.insert(
            "simpleConnectManaged".into(),
            Value::String(MANAGED_MARKER.into()),
        );
    }
    write_opencode_config(&full)?;

    Ok(WriteOutcome {
        files: vec![get_opencode_config_path()],
    })
}

pub fn status() -> Result<StatusOutcome, AppError> {
    let path = get_opencode_config_path();
    if !path.exists() {
        return Ok(StatusOutcome::empty());
    }
    let v = read_json_or_empty(&path)?;
    let managed = v
        .get("simpleConnectManaged")
        .and_then(|x| x.as_str());
    let beeapi = v.get("provider").and_then(|p| p.get(SC_PROVIDER_ID));
    let base_url = beeapi
        .and_then(|b| b.get("options"))
        .and_then(|o| o.get("baseURL"))
        .and_then(|u| u.as_str())
        .map(String::from);
    let key = beeapi
        .and_then(|b| b.get("options"))
        .and_then(|o| o.get("apiKey"))
        .and_then(|k| k.as_str())
        .map(String::from);
    let model = v.get("model").and_then(|m| m.as_str()).map(String::from);
    let configured = configured_by_marker(managed, base_url.as_deref()) || beeapi.is_some();
    Ok(StatusOutcome {
        configured,
        base_url,
        model,
        key,
        ..StatusOutcome::empty()
    })
}

pub fn clear() -> Result<(), AppError> {
    let path = get_opencode_config_path();
    if !path.exists() {
        return Ok(());
    }
    let v = read_json_or_empty(&path)?;
    let managed = v
        .get("simpleConnectManaged")
        .and_then(|x| x.as_str())
        .map(super::shared::is_managed_value)
        .unwrap_or(false);
    if !managed {
        return Ok(());
    }
    remove_provider(SC_PROVIDER_ID)?;
    let mut full = read_opencode_config()?;
    if let Some(obj) = full.as_object_mut() {
        obj.remove("simpleConnectManaged");
        if obj
            .get("model")
            .and_then(|m| m.as_str())
            .map(|m| m.starts_with(&format!("{SC_PROVIDER_ID}/")))
            .unwrap_or(false)
        {
            obj.remove("model");
        }
    }
    write_opencode_config(&full)
}
