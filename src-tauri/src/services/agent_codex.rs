//! Markdown subagent (YAML frontmatter + body) → Codex `.toml` agent file.

use serde_yaml::Value;
use toml_edit::{DocumentMut, Item, Table};

use crate::error::AppError;

const CODEX_SCALAR_KEYS: &[&str] = &["model", "model_reasoning_effort", "sandbox_mode"];

/// Split optional YAML frontmatter from markdown body.
pub fn split_yaml_frontmatter(content: &str) -> (Option<&str>, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content.trim().to_string());
    }

    let after_open = &trimmed[3..];
    let end = after_open.find("\n---");
    let Some(end_idx) = end else {
        return (None, content.trim().to_string());
    };

    let yaml = after_open[..end_idx].trim();
    let body_start = end_idx + 4;
    let body = after_open
        .get(body_start..)
        .map(|s| s.trim_start_matches(['\r', '\n']).trim().to_string())
        .unwrap_or_default();

    if yaml.is_empty() {
        (None, body)
    } else {
        (Some(yaml), body)
    }
}

/// Convert SSOT markdown agent definition to Codex TOML text.
pub fn markdown_agent_to_codex_toml(
    fallback_name: &str,
    fallback_description: Option<&str>,
    markdown: &str,
) -> Result<String, AppError> {
    let (yaml_text, body) = split_yaml_frontmatter(markdown);
    let meta: Value = match yaml_text {
        Some(yaml) => serde_yaml::from_str(yaml)
            .map_err(|e| AppError::Config(format!("YAML frontmatter 无效: {e}")))?,
        None => Value::Null,
    };

    let map = meta.as_mapping();

    let name = string_from_map(map, "name").unwrap_or_else(|| fallback_name.to_string());
    let description = string_from_map(map, "description")
        .or_else(|| fallback_description.map(str::to_string))
        .unwrap_or_default();

    let developer_instructions = if body.is_empty() {
        string_from_map(map, "developer_instructions").unwrap_or_default()
    } else {
        body
    };

    if developer_instructions.trim().is_empty() {
        return Err(AppError::Config(
            "Subagent 缺少正文或 developer_instructions".into(),
        ));
    }

    let mut table = Table::new();
    table.insert("name", Item::Value(name.into()));
    table.insert("description", Item::Value(description.into()));
    table.insert(
        "developer_instructions",
        Item::Value(developer_instructions.into()),
    );

    if let Some(map) = map {
        for key in CODEX_SCALAR_KEYS {
            if let Some(v) = map.get(Value::from(*key)) {
                if let Some(item) = yaml_value_to_toml_item(v) {
                    table.insert(key, item);
                }
            }
        }
        if let Some(mcp) = map.get(Value::from("mcp_servers")) {
            if let Some(mcp_table) = yaml_value_to_toml_table(mcp) {
                table.insert("mcp_servers", Item::Table(mcp_table));
            }
        }
    }

    let mut doc = DocumentMut::new();
    for (k, v) in table.iter() {
        doc[k] = v.clone();
    }
    Ok(doc.to_string())
}

fn string_from_map(map: Option<&serde_yaml::Mapping>, key: &str) -> Option<String> {
    map?.get(Value::from(key))?.as_str().map(str::to_string)
}

fn yaml_value_to_toml_item(value: &Value) -> Option<Item> {
    match value {
        Value::String(s) => Some(Item::Value(s.clone().into())),
        Value::Bool(b) => Some(Item::Value((*b).into())),
        Value::Number(n) => n
            .as_i64()
            .map(|i| Item::Value(i.into()))
            .or_else(|| n.as_f64().map(|f| Item::Value((f as i64).into()))),
        Value::Mapping(map) => {
            yaml_mapping_to_toml_table(&Value::Mapping(map.clone())).map(Item::Table)
        }
        Value::Sequence(seq) => {
            let mut arr = toml_edit::Array::new();
            for v in seq {
                if let Value::String(s) = v {
                    arr.push(s.as_str());
                }
            }
            Some(Item::Value(arr.into()))
        }
        _ => None,
    }
}

fn yaml_value_to_toml_table(value: &Value) -> Option<Table> {
    let map = value.as_mapping()?;
    let mut table = Table::new();
    for (k, v) in map {
        let key = k.as_str()?;
        if let Some(item) = yaml_value_to_toml_item(v) {
            table.insert(key, item);
        }
    }
    Some(table)
}

fn yaml_mapping_to_toml_table(value: &Value) -> Option<Table> {
    yaml_value_to_toml_table(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_frontmatter_and_body_to_toml() {
        let md = r#"---
name: code-reviewer
description: Reviews pull requests
model: gpt-5.5
---
You are a meticulous code reviewer.
"#;
        let toml = markdown_agent_to_codex_toml("fallback", None, md).expect("toml");
        assert!(toml.contains("name = \"code-reviewer\""));
        assert!(toml.contains("description = \"Reviews pull requests\""));
        assert!(toml.contains("developer_instructions"));
        assert!(toml.contains("meticulous code reviewer"));
        assert!(toml.contains("model = \"gpt-5.5\""));
    }

    #[test]
    fn uses_fallback_name_without_frontmatter() {
        let toml =
            markdown_agent_to_codex_toml("explorer", Some("Explore code"), "Find relevant files.")
                .expect("toml");
        assert!(toml.contains("name = \"explorer\""));
        assert!(toml.contains("description = \"Explore code\""));
    }
}
