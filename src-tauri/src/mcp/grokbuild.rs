//! Grok Build MCP projection in `~/.grok/config.toml`.

use serde_json::{json, Value};

use crate::app_config::{McpApps, McpServer, MultiAppConfig};
use crate::error::AppError;

use super::codex::json_server_to_toml_table;
use super::validation::validate_server_spec;

fn should_sync() -> bool {
    crate::grok_config::get_grok_config_dir().exists()
}

fn read_config_text() -> Result<String, AppError> {
    let path = crate::grok_config::get_grok_config_path();
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))
}

fn server_to_toml(server_spec: &Value) -> Result<toml_edit::Table, AppError> {
    let mut table = json_server_to_toml_table(server_spec)?;
    table.remove("type");
    if let Some(headers) = table.remove("http_headers") {
        table.insert("headers", headers);
    }
    Ok(table)
}

fn toml_to_server(entry: &toml::value::Table) -> Value {
    fn convert(value: &toml::Value) -> Option<Value> {
        match value {
            toml::Value::String(value) => Some(json!(value)),
            toml::Value::Integer(value) => Some(json!(value)),
            toml::Value::Float(value) => Some(json!(value)),
            toml::Value::Boolean(value) => Some(json!(value)),
            toml::Value::Datetime(value) => Some(json!(value.to_string())),
            toml::Value::Array(values) => {
                Some(Value::Array(values.iter().filter_map(convert).collect()))
            }
            toml::Value::Table(values) => Some(Value::Object(
                values
                    .iter()
                    .filter_map(|(key, value)| convert(value).map(|value| (key.clone(), value)))
                    .collect(),
            )),
        }
    }

    let mut spec = serde_json::Map::new();
    for (key, value) in entry {
        let output_key = if key == "http_headers" {
            "headers"
        } else {
            key
        };
        if let Some(value) = convert(value) {
            spec.insert(output_key.to_string(), value);
        }
    }
    let default_type = if spec.contains_key("url") {
        "http"
    } else {
        "stdio"
    };
    spec.entry("type".to_string())
        .or_insert_with(|| json!(default_type));
    Value::Object(spec)
}

pub fn read_grokbuild_servers_map() -> Result<std::collections::HashMap<String, Value>, AppError> {
    let text = read_config_text()?;
    if text.trim().is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let root: toml::Table = toml::from_str(&text)
        .map_err(|e| AppError::McpValidation(format!("解析 ~/.grok/config.toml 失败: {e}")))?;
    let Some(entries) = root.get("mcp_servers").and_then(toml::Value::as_table) else {
        return Ok(std::collections::HashMap::new());
    };
    Ok(entries
        .iter()
        .filter_map(|(id, entry)| {
            entry
                .as_table()
                .map(|entry| (id.clone(), toml_to_server(entry)))
        })
        .collect())
}

pub fn import_from_grokbuild(config: &mut MultiAppConfig) -> Result<usize, AppError> {
    let text = read_config_text()?;
    if text.trim().is_empty() {
        return Ok(0);
    }
    let root: toml::Table = toml::from_str(&text)
        .map_err(|e| AppError::McpValidation(format!("解析 ~/.grok/config.toml 失败: {e}")))?;
    let Some(entries) = root.get("mcp_servers").and_then(toml::Value::as_table) else {
        return Ok(0);
    };
    let servers = config
        .mcp
        .servers
        .get_or_insert_with(std::collections::HashMap::new);
    let mut changed = 0;
    for (id, entry) in entries {
        let Some(entry) = entry.as_table() else {
            continue;
        };
        let spec = toml_to_server(entry);
        if let Err(error) = validate_server_spec(&spec) {
            log::warn!("跳过无效 Grok Build MCP 项 '{id}': {error}");
            continue;
        }
        if let Some(existing) = servers.get_mut(id) {
            if !existing.apps.grokbuild {
                existing.apps.grokbuild = true;
                changed += 1;
            }
        } else {
            servers.insert(
                id.clone(),
                McpServer {
                    id: id.clone(),
                    name: id.clone(),
                    server: spec,
                    apps: McpApps {
                        grokbuild: true,
                        ..Default::default()
                    },
                    description: None,
                    homepage: None,
                    docs: None,
                    tags: Vec::new(),
                },
            );
            changed += 1;
        }
    }
    Ok(changed)
}

pub fn sync_single_server_to_grokbuild(
    _config: &MultiAppConfig,
    id: &str,
    server_spec: &Value,
) -> Result<(), AppError> {
    if !should_sync() {
        return Ok(());
    }
    let path = crate::grok_config::get_grok_config_path();
    let text = read_config_text()?;
    let mut document = if text.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        text.parse::<toml_edit::DocumentMut>().map_err(|e| {
            AppError::McpValidation(format!("解析 Grok Build config.toml 失败: {e}"))
        })?
    };
    if document
        .get("mcp_servers")
        .is_none_or(|item| item.as_table_like().is_none())
    {
        document["mcp_servers"] = toml_edit::table();
    }
    let servers = document
        .get_mut("mcp_servers")
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| AppError::McpValidation("Grok Build mcp_servers 不是表".to_string()))?;
    servers.insert(id, toml_edit::Item::Table(server_to_toml(server_spec)?));
    crate::config::write_text_file(&path, &document.to_string())
}

pub fn remove_server_from_grokbuild(id: &str) -> Result<(), AppError> {
    if !should_sync() {
        return Ok(());
    }
    let path = crate::grok_config::get_grok_config_path();
    if !path.exists() {
        return Ok(());
    }
    let text = read_config_text()?;
    let mut document = match text.parse::<toml_edit::DocumentMut>() {
        Ok(document) => document,
        Err(error) => {
            log::warn!("解析 Grok Build config.toml 失败: {error}，跳过删除操作");
            return Ok(());
        }
    };
    if let Some(item) = document.get_mut("mcp_servers") {
        if let Some(servers) = item.as_table_like_mut() {
            servers.remove(id);
        }
    }
    crate::config::write_text_file(&path, &document.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_grok_remote_server_without_codex_transport_fields() {
        let table = server_to_toml(&json!({
            "type": "http",
            "url": "https://example.com/mcp",
            "headers": {"Authorization": "Bearer token"}
        }))
        .expect("convert");
        assert!(!table.contains_key("type"));
        assert!(!table.contains_key("http_headers"));
        assert!(table.contains_key("headers"));
    }
}
