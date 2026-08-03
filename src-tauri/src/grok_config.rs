//! Native Grok Build configuration support.
//!
//! Grok Build stores its active provider in `~/.grok/config.toml`.  OpenSunstar
//! keeps the TOML document inside the provider record and writes it back to the
//! native path only when the provider is selected.  This keeps the database as
//! the source of truth while preserving user-owned tables such as MCP.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::config::{get_home_dir, write_text_file};
use crate::error::AppError;
use crate::provider::Provider;

pub const DEFAULT_MODEL: &str = "grok-4.5";
pub const DEFAULT_API_BACKEND: &str = "responses";
pub const DEFAULT_CONTEXT_WINDOW: i64 = 500_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokModelConfig {
    pub profile: String,
    pub model: String,
    pub base_url: String,
    pub name: String,
    pub api_key: Option<String>,
    pub env_key: Option<String>,
    pub api_backend: String,
    pub context_window: i64,
}

pub fn get_grok_config_dir() -> PathBuf {
    get_home_dir().join(".grok")
}

pub fn get_grok_config_path() -> PathBuf {
    get_grok_config_dir().join("config.toml")
}

fn required_string<'a>(table: &'a toml::value::Table, key: &str) -> Result<&'a str, AppError> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.field.missing",
                format!("Grok Build 配置缺少有效的 {key} 字段"),
                format!("Grok Build configuration is missing a valid {key} field"),
            )
        })
}

fn optional_string(table: &toml::value::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub fn validate_config_toml_syntax(config_toml: &str) -> Result<(), AppError> {
    if config_toml.trim().is_empty() {
        return Ok(());
    }
    config_toml
        .parse::<toml::Value>()
        .map(|_| ())
        .map_err(|error| {
            AppError::localized(
                "provider.grokbuild.config.invalid_toml",
                format!("Grok Build config.toml 格式错误: {error}"),
                format!("Invalid Grok Build config.toml: {error}"),
            )
        })
}

pub fn is_official_live_config(config_toml: &str) -> bool {
    let Ok(document) = config_toml.parse::<toml::Value>() else {
        return false;
    };
    document
        .as_table()
        .is_some_and(|root| !root.contains_key("models") && !root.contains_key("model"))
}

pub fn validate_config_toml(config_toml: &str) -> Result<(), AppError> {
    let document = config_toml.parse::<toml::Value>().map_err(|error| {
        AppError::localized(
            "provider.grokbuild.config.invalid_toml",
            format!("Grok Build config.toml 格式错误: {error}"),
            format!("Invalid Grok Build config.toml: {error}"),
        )
    })?;
    let root = document.as_table().ok_or_else(|| {
        AppError::localized(
            "provider.grokbuild.config.not_table",
            "Grok Build 配置必须是 TOML 表结构",
            "Grok Build configuration must be a TOML table",
        )
    })?;
    let models = root
        .get("models")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.models.missing",
                "Grok Build 配置缺少 [models]",
                "Grok Build configuration is missing [models]",
            )
        })?;
    let default_model = required_string(models, "default")?;
    let model_entries = root
        .get("model")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.model.missing",
                "Grok Build 配置缺少 [model.<name>]",
                "Grok Build configuration is missing [model.<name>]",
            )
        })?;
    let selected = model_entries
        .get(default_model)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.default_model.missing",
                format!("Grok Build 配置缺少 [model.\"{default_model}\"]"),
                format!("Grok Build configuration is missing [model.\"{default_model}\"]"),
            )
        })?;

    required_string(selected, "model")?;
    required_string(selected, "base_url")?;
    required_string(selected, "name")?;
    required_string(selected, "api_backend")?;
    if optional_string(selected, "api_key").is_none()
        && optional_string(selected, "env_key").is_none()
    {
        return Err(AppError::localized(
            "provider.grokbuild.credentials.missing",
            "Grok Build 配置缺少有效的 api_key 或 env_key 字段",
            "Grok Build configuration is missing a valid api_key or env_key field",
        ));
    }
    if selected
        .get("context_window")
        .and_then(toml::Value::as_integer)
        .filter(|value| *value > 0)
        .is_none()
    {
        return Err(AppError::localized(
            "provider.grokbuild.context_window.invalid",
            "Grok Build context_window 必须是正整数",
            "Grok Build context_window must be a positive integer",
        ));
    }
    Ok(())
}

pub fn extract_model_config(config_toml: &str) -> Option<GrokModelConfig> {
    let document = config_toml.parse::<toml::Value>().ok()?;
    let root = document.as_table()?;
    let profile = root
        .get("models")?
        .as_table()?
        .get("default")?
        .as_str()?
        .trim();
    let selected = root.get("model")?.as_table()?.get(profile)?.as_table()?;
    Some(GrokModelConfig {
        profile: profile.to_string(),
        model: selected.get("model")?.as_str()?.trim().to_string(),
        base_url: selected
            .get("base_url")?
            .as_str()?
            .trim_end_matches('/')
            .to_string(),
        name: selected.get("name")?.as_str()?.trim().to_string(),
        api_key: optional_string(selected, "api_key"),
        env_key: optional_string(selected, "env_key"),
        api_backend: optional_string(selected, "api_backend")
            .unwrap_or_else(|| DEFAULT_API_BACKEND.to_string()),
        context_window: selected
            .get("context_window")
            .and_then(toml::Value::as_integer)
            .unwrap_or(DEFAULT_CONTEXT_WINDOW),
    })
}

pub fn extract_credentials(config_toml: &str) -> Option<(String, String)> {
    let config = extract_model_config(config_toml)?;
    let api_key = config.api_key.or_else(|| {
        config
            .env_key
            .as_deref()
            .and_then(|key| std::env::var(key).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })?;
    Some((extract_base_url(config_toml)?, api_key))
}

pub fn extract_base_url(config_toml: &str) -> Option<String> {
    extract_model_config(config_toml).map(|config| config.base_url)
}

#[allow(dead_code)]
fn update_selected_string(config_toml: &str, field: &str, value: &str) -> Result<String, AppError> {
    let mut document = config_toml
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| {
            AppError::localized(
                "provider.grokbuild.config.invalid_toml",
                format!("Grok Build config.toml 格式错误: {error}"),
                format!("Invalid Grok Build config.toml: {error}"),
            )
        })?;
    let profile = document
        .get("models")
        .and_then(|item| item.get("default"))
        .and_then(toml_edit::Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.default_model.missing",
                "Grok Build 配置缺少 models.default",
                "Grok Build configuration is missing models.default",
            )
        })?
        .to_string();
    let selected = document
        .get_mut("model")
        .and_then(|item| item.get_mut(&profile))
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.default_model.missing",
                format!("Grok Build 配置缺少 [model.\"{profile}\"]"),
                format!("Grok Build configuration is missing [model.\"{profile}\"]"),
            )
        })?;
    selected.insert(field, toml_edit::value(value));
    Ok(document.to_string())
}

#[allow(dead_code)]
pub fn apply_proxy_takeover(
    config_toml: &str,
    proxy_base_url: &str,
    token_placeholder: &str,
) -> Result<String, AppError> {
    let updated = update_selected_string(config_toml, "base_url", proxy_base_url)?;
    update_selected_string(&updated, "api_key", token_placeholder)
}

#[allow(dead_code)]
pub fn update_api_key(config_toml: &str, api_key: &str) -> Result<String, AppError> {
    update_selected_string(config_toml, "api_key", api_key)
}

#[allow(dead_code)]
pub fn has_proxy_placeholder(config_toml: &str, token_placeholder: &str) -> bool {
    extract_model_config(config_toml)
        .and_then(|config| config.api_key)
        .is_some_and(|api_key| api_key == token_placeholder)
}

pub fn strip_grok_mcp_servers_from_settings(settings: &mut Value) -> Result<(), AppError> {
    let Some(config_text) = settings.get("config").and_then(Value::as_str) else {
        return Ok(());
    };
    let mut document = config_text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| {
            AppError::localized(
                "provider.grokbuild.config.invalid_toml",
                format!("Grok Build config.toml 格式错误: {error}"),
                format!("Invalid Grok Build config.toml: {error}"),
            )
        })?;
    let mut changed = document.as_table_mut().remove("mcp_servers").is_some();
    if let Some(mcp_table) = document
        .get_mut("mcp")
        .and_then(toml_edit::Item::as_table_like_mut)
    {
        if mcp_table.remove("servers").is_some() {
            changed = true;
        }
        if mcp_table.is_empty() {
            document.as_table_mut().remove("mcp");
        }
    }
    if changed {
        if let Some(object) = settings.as_object_mut() {
            object.insert("config".to_string(), Value::String(document.to_string()));
        }
    }
    Ok(())
}

pub fn read_grok_live_settings() -> Result<Value, AppError> {
    let path = get_grok_config_path();
    if !path.exists() {
        return Err(AppError::localized(
            "grokbuild.config.missing",
            "Grok Build 配置文件不存在",
            "Grok Build configuration file not found",
        ));
    }
    let config = fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?;
    validate_config_toml_syntax(&config)?;
    Ok(json!({ "config": config }))
}

pub fn write_grok_provider_live(provider: &Provider) -> Result<(), AppError> {
    let config = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.config.missing",
                "Grok Build 配置缺少 config 字段",
                "Grok Build configuration is missing the config field",
            )
        })?;
    if provider.category.as_deref() != Some("official") && !is_official_live_config(config) {
        validate_config_toml(config)?;
    }
    let mut settings = json!({ "config": config });
    strip_grok_mcp_servers_from_settings(&mut settings)?;
    write_grok_live_settings(&settings)
}

pub fn write_grok_live_settings(settings: &Value) -> Result<(), AppError> {
    let config = settings
        .get("config")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::localized(
                "provider.grokbuild.config.missing",
                "Grok Build 配置缺少 config 字段",
                "Grok Build configuration is missing the config field",
            )
        })?;
    validate_config_toml_syntax(config)?;
    write_text_file(&get_grok_config_path(), config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_config() -> &'static str {
        r#"[models]
default = "grok-4.5"

[model."grok-4.5"]
model = "grok-4.5"
base_url = "https://example.com/v1"
name = "Example"
api_key = "secret"
api_backend = "responses"
context_window = 500000
"#
    }

    #[test]
    fn validates_and_extracts_native_config() {
        validate_config_toml(valid_config()).expect("valid Grok config");
        let model = extract_model_config(valid_config()).expect("selected model");
        assert_eq!(model.profile, "grok-4.5");
        assert_eq!(model.base_url, "https://example.com/v1");
        assert_eq!(
            extract_credentials(valid_config()),
            Some(("https://example.com/v1".into(), "secret".into()))
        );
    }

    #[test]
    fn accepts_official_syntax_only_snapshot_and_detects_custom_tables() {
        validate_config_toml_syntax("[mcp_servers.echo]\ncommand = \"echo\"\n").expect("syntax");
        assert!(is_official_live_config(
            "[mcp_servers.echo]\ncommand = \"echo\"\n"
        ));
        assert!(!is_official_live_config(valid_config()));
        assert!(validate_config_toml("[models]\ndefault = \"grok-4.5\"\n").is_err());
    }

    #[test]
    fn proxy_takeover_updates_only_selected_model() {
        let updated = apply_proxy_takeover(
            valid_config(),
            "http://127.0.0.1:15721/grokbuild/v1",
            "PROXY_MANAGED",
        )
        .expect("takeover");
        let model = extract_model_config(&updated).expect("selected model");
        assert_eq!(model.base_url, "http://127.0.0.1:15721/grokbuild/v1");
        assert!(has_proxy_placeholder(&updated, "PROXY_MANAGED"));
    }

    #[test]
    fn strips_mcp_projection_without_touching_other_tables() {
        let mut settings = json!({"config": "[models]\ndefault = \"grok-4.5\"\n\n[mcp_servers.echo]\ncommand = \"echo\"\n"});
        strip_grok_mcp_servers_from_settings(&mut settings).expect("strip");
        let config = settings["config"].as_str().unwrap();
        assert!(config.contains("[models]"));
        assert!(!config.contains("mcp_servers"));
    }
}
