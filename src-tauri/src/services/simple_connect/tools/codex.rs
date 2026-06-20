//! Codex CLI 写入（OpenAI-compatible，provider slug = supplier_id）

use super::shared::{
    normalize_base, write_json_pretty, write_text, WriteOutcome, MANAGED_MARKER, StatusOutcome,
};
use crate::codex_config::{get_codex_auth_path, get_codex_config_dir};
use crate::error::AppError;
use serde_json::{json, Value};
use toml_edit::{value, DocumentMut, Item, Table};

fn provider_slug(supplier_id: &str) -> String {
    supplier_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn paths() -> Vec<std::path::PathBuf> {
    vec![
        get_codex_config_dir().join("config.toml"),
        get_codex_auth_path(),
    ]
}

pub fn apply(
    supplier_id: &str,
    api_key: &str,
    model: &str,
    openai_base: &str,
) -> Result<WriteOutcome, AppError> {
    let slug = provider_slug(supplier_id);
    let (_, openai) = normalize_base(openai_base);
    let cfg_path = get_codex_config_dir().join("config.toml");
    let auth_path = get_codex_auth_path();

    if let Some(parent) = cfg_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let existing = if cfg_path.exists() {
        std::fs::read_to_string(&cfg_path).unwrap_or_default()
    } else {
        String::new()
    };
    let mut doc: DocumentMut = existing.parse().unwrap_or_else(|_| DocumentMut::new());

    doc["model"] = value(model.trim());
    doc["model_provider"] = value(slug.as_str());
    doc["preferred_auth_method"] = value("apikey");
    doc["disable_response_storage"] = value(true);
    doc["simple_connect_managed"] = value(MANAGED_MARKER);

    if !matches!(doc.get("model_providers"), Some(Item::Table(_))) {
        doc["model_providers"] = Item::Table(Table::new());
    }
    let providers = doc["model_providers"]
        .as_table_mut()
        .ok_or_else(|| AppError::Message("config.toml model_providers 不是表".into()))?;
    providers.set_implicit(true);

    let mut provider_table = Table::new();
    provider_table["name"] = value(supplier_id);
    provider_table["base_url"] = value(openai.as_str());
    provider_table["wire_api"] = value("responses");
    providers.insert(slug.as_str(), Item::Table(provider_table));

    write_text(&cfg_path, &doc.to_string())?;

    let auth = json!({
        "OPENAI_API_KEY": api_key.trim(),
        "simple_connect_managed": MANAGED_MARKER
    });
    write_json_pretty(&auth_path, &auth)?;

    Ok(WriteOutcome {
        files: vec![cfg_path, auth_path],
    })
}

pub fn status() -> Result<StatusOutcome, AppError> {
    let cfg_path = get_codex_config_dir().join("config.toml");
    if !cfg_path.exists() {
        return Ok(StatusOutcome::empty());
    }
    let text = std::fs::read_to_string(&cfg_path).unwrap_or_default();
    let configured = text.contains("simple_connect_managed")
        || text.contains(MANAGED_MARKER)
        || text.contains("127.0.0.1");
    let doc: DocumentMut = text.parse().unwrap_or_default();
    let model = doc.get("model").and_then(|v| v.as_str()).map(String::from);
    let provider = doc.get("model_provider").and_then(|v| v.as_str());
    let provider_table = doc
        .get("model_providers")
        .and_then(|mp| mp.as_table())
        .and_then(|providers| provider.and_then(|id| providers.get(id)))
        .and_then(|b| b.as_table());
    let base_url = provider_table
        .and_then(|t| t.get("base_url"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let wire_api = provider_table
        .and_then(|t| t.get("wire_api"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let auth_path = get_codex_auth_path();
    let key = if auth_path.exists() {
        crate::config::read_json_file::<Value>(&auth_path)
            .ok()
            .and_then(|v| {
                v.get("OPENAI_API_KEY")
                    .and_then(|x| x.as_str())
                    .map(String::from)
            })
    } else {
        None
    };
    Ok(StatusOutcome {
        configured,
        base_url,
        model,
        key,
        wire_api,
    })
}

pub fn clear() -> Result<(), AppError> {
    let cfg_path = get_codex_config_dir().join("config.toml");
    if cfg_path.exists() {
        let text = std::fs::read_to_string(&cfg_path).unwrap_or_default();
        if text.contains(MANAGED_MARKER) || text.contains("simple_connect_managed") {
            let mut doc: DocumentMut = text.parse().unwrap_or_default();
            doc.as_table_mut().remove("simple_connect_managed");
            if let Some(mp) = doc.get_mut("model_providers").and_then(|x| x.as_table_mut()) {
                let keys: Vec<String> = mp
                    .iter()
                    .filter_map(|(k, v)| {
                        v.as_table()
                            .and_then(|t| t.get("base_url"))
                            .and_then(|u| u.as_str())
                            .filter(|b| b.contains("127.0.0.1"))
                            .map(|_| k.to_string())
                    })
                    .collect();
                for k in keys {
                    mp.remove(&k);
                }
            }
            write_text(&cfg_path, &doc.to_string())?;
        }
    }
    let auth_path = get_codex_auth_path();
    if auth_path.exists() {
        if let Ok(v) = crate::config::read_json_file::<Value>(&auth_path) {
            if v.get("simple_connect_managed")
                .and_then(|x| x.as_str())
                .map(super::shared::is_managed_value)
                .unwrap_or(false)
            {
                let _ = std::fs::remove_file(&auth_path);
            }
        }
    }
    Ok(())
}
