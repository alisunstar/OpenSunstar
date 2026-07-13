//! Hermes CLI 写入（custom_providers + model 路由）

use super::shared::{
    configured_by_marker, normalize_base, StatusOutcome, WriteOutcome, MANAGED_MARKER,
    SC_PROVIDER_ID,
};
use crate::error::AppError;
use crate::hermes_config::{
    get_hermes_config_path, get_model_config, read_hermes_config, remove_provider,
    set_model_config, set_provider, HermesModelConfig,
};
use serde_json::json;

pub fn paths() -> Vec<std::path::PathBuf> {
    vec![get_hermes_config_path()]
}

pub fn apply(api_key: &str, model: &str, openai_base: &str) -> Result<WriteOutcome, AppError> {
    let (_, openai) = normalize_base(openai_base);
    let model = model.trim();

    let provider_config = json!({
        "base_url": openai,
        "api_key": api_key.trim(),
        "api_mode": "chat_completions",
        "simple_connect_managed": MANAGED_MARKER,
        "models": {
            model: { "context_length": 128000 }
        }
    });

    set_provider(SC_PROVIDER_ID, provider_config.clone())?;

    let current = get_model_config()?.unwrap_or_default();
    set_model_config(&HermesModelConfig {
        default: Some(model.to_string()),
        provider: Some(SC_PROVIDER_ID.into()),
        ..current
    })?;

    Ok(WriteOutcome {
        files: vec![get_hermes_config_path()],
    })
}

pub fn status() -> Result<StatusOutcome, AppError> {
    let path = get_hermes_config_path();
    if !path.exists() {
        return Ok(StatusOutcome::empty());
    }
    let config = read_hermes_config()?;
    let providers = config.get("custom_providers").and_then(|v| v.as_sequence());
    let entry = providers.and_then(|seq| {
        seq.iter()
            .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(SC_PROVIDER_ID))
    });
    let base_url = entry
        .and_then(|p| p.get("base_url"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let key = entry
        .and_then(|p| p.get("api_key"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let managed = entry
        .and_then(|p| p.get("simple_connect_managed"))
        .and_then(|x| x.as_str());
    let model_cfg = get_model_config().ok().flatten();
    let model = model_cfg.as_ref().and_then(|m| m.default.clone());
    let configured = configured_by_marker(managed, base_url.as_deref())
        || model_cfg.as_ref().and_then(|m| m.provider.as_deref()) == Some(SC_PROVIDER_ID);
    Ok(StatusOutcome {
        configured,
        base_url,
        model,
        key,
        ..StatusOutcome::empty()
    })
}

pub fn clear() -> Result<(), AppError> {
    let path = get_hermes_config_path();
    if !path.exists() {
        return Ok(());
    }
    let config = read_hermes_config()?;
    let managed = config
        .get("custom_providers")
        .and_then(|v| v.as_sequence())
        .and_then(|seq| {
            seq.iter()
                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(SC_PROVIDER_ID))
        })
        .and_then(|p| p.get("simple_connect_managed"))
        .and_then(|x| x.as_str())
        .map(super::shared::is_managed_value)
        .unwrap_or(false);
    if !managed {
        return Ok(());
    }
    remove_provider(SC_PROVIDER_ID)?;
    Ok(())
}
