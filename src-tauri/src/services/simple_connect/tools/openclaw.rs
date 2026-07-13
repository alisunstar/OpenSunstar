//! OpenClaw CLI 写入（models.providers + agents.defaults）

use super::shared::{
    configured_by_marker, normalize_base, StatusOutcome, WriteOutcome, MANAGED_MARKER,
    SC_PROVIDER_ID,
};
use crate::error::AppError;
use crate::openclaw_config::{
    get_openclaw_config_path, read_openclaw_config, remove_provider, set_default_model,
    set_typed_provider, OpenClawDefaultModel, OpenClawModelEntry, OpenClawProviderConfig,
};
use std::collections::HashMap;

pub fn paths() -> Vec<std::path::PathBuf> {
    vec![get_openclaw_config_path()]
}

pub fn apply(api_key: &str, model: &str, anthropic_base: &str) -> Result<WriteOutcome, AppError> {
    let (root, _) = normalize_base(anthropic_base);
    let model = model.trim();

    let mut extra = HashMap::new();
    extra.insert(
        "simpleConnectManaged".into(),
        serde_json::json!(MANAGED_MARKER),
    );

    let config = OpenClawProviderConfig {
        base_url: Some(root),
        api_key: Some(api_key.trim().to_string()),
        api: Some("anthropic-messages".into()),
        models: vec![OpenClawModelEntry {
            id: model.to_string(),
            name: Some(model.to_string()),
            alias: None,
            cost: None,
            context_window: None,
            extra: HashMap::new(),
        }],
        headers: HashMap::new(),
        extra,
    };

    set_typed_provider(SC_PROVIDER_ID, &config)?;
    set_default_model(&OpenClawDefaultModel {
        primary: format!("{SC_PROVIDER_ID}/{model}"),
        fallbacks: vec![],
        extra: HashMap::new(),
    })?;

    Ok(WriteOutcome {
        files: vec![get_openclaw_config_path()],
    })
}

pub fn status() -> Result<StatusOutcome, AppError> {
    let path = get_openclaw_config_path();
    if !path.exists() {
        return Ok(StatusOutcome::empty());
    }
    let v = read_openclaw_config()?;
    let provider = v
        .pointer("/models/providers/simple-connect")
        .cloned()
        .or_else(|| v.pointer("/models/providers/simple_connect").cloned());
    let base_url = provider
        .as_ref()
        .and_then(|p| p.get("baseUrl"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let key = provider
        .as_ref()
        .and_then(|p| p.get("apiKey"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let managed = provider
        .as_ref()
        .and_then(|p| p.get("simpleConnectManaged"))
        .and_then(|x| x.as_str());
    let model = v
        .pointer("/agents/defaults/model/primary")
        .and_then(|x| x.as_str())
        .map(String::from);
    let configured = configured_by_marker(managed, base_url.as_deref()) || provider.is_some();
    Ok(StatusOutcome {
        configured,
        base_url,
        model,
        key,
        ..StatusOutcome::empty()
    })
}

pub fn clear() -> Result<(), AppError> {
    let path = get_openclaw_config_path();
    if !path.exists() {
        return Ok(());
    }
    let v = read_openclaw_config()?;
    let managed = v
        .pointer("/models/providers/simple-connect/simpleConnectManaged")
        .or_else(|| v.pointer("/models/providers/simple_connect/simpleConnectManaged"))
        .and_then(|x| x.as_str())
        .map(super::shared::is_managed_value)
        .unwrap_or(false);
    if !managed {
        return Ok(());
    }
    remove_provider(SC_PROVIDER_ID)?;
    Ok(())
}
