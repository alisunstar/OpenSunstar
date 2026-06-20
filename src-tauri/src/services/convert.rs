//! Cross-tool configuration convert wizard (F3 / M2).
//!
//! Supports detecting prompt/MCP/skill/command sources, previewing conversions, and applying
//! writes with backup + rollback on failure.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use toml_edit::{DocumentMut, Item, Table};

use crate::app_config::AppType;
use crate::claude_mcp;
use crate::config::{atomic_write, get_claude_mcp_path, write_text_file};
use crate::error::AppError;
use crate::gemini_mcp;
use crate::prompt_files;
use crate::services::agent_codex::markdown_agent_to_codex_toml;
use crate::services::bridge::{self, BridgePreview};
use crate::services::skill::SkillService;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertSourceItem {
    pub content_type: String,
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertApplyRequest {
    pub source_app: String,
    pub target_app: String,
    pub content_type: String,
    pub content: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertApplyResult {
    pub written_paths: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn detect_convert_sources(source_app: &str) -> Result<Vec<ConvertSourceItem>, AppError> {
    let app = parse_app(source_app)?;
    let mut items = Vec::new();

    // Prompt / rules file
    let prompt_path = prompt_files::prompt_file_path(&app)?;
    let prompt_exists = prompt_path.exists();
    let prompt_content = if prompt_exists {
        Some(fs::read_to_string(&prompt_path).map_err(|e| AppError::io(&prompt_path, e))?)
    } else {
        None
    };
    items.push(ConvertSourceItem {
        content_type: "prompt".into(),
        label: prompt_filename(&app).to_string(),
        path: prompt_path.display().to_string(),
        exists: prompt_exists,
        content: prompt_content,
    });

    // MCP config
    let (mcp_path, mcp_exists, mcp_content) = read_mcp_source(&app)?;
    items.push(ConvertSourceItem {
        content_type: "mcp".into(),
        label: "MCP Servers".into(),
        path: mcp_path.display().to_string(),
        exists: mcp_exists,
        content: mcp_content,
    });

    // Skills
    let (skill_path, skill_exists, skill_content) = read_skills_source(&app)?;
    items.push(ConvertSourceItem {
        content_type: "skill".into(),
        label: "Skills".into(),
        path: skill_path.display().to_string(),
        exists: skill_exists,
        content: skill_content,
    });

    // Commands
    let (cmd_path, cmd_exists, cmd_content) = read_commands_source(&app)?;
    items.push(ConvertSourceItem {
        content_type: "command".into(),
        label: "Commands".into(),
        path: cmd_path.display().to_string(),
        exists: cmd_exists,
        content: cmd_content,
    });

    // Subagents (agents)
    let (agent_path, agent_exists, agent_content) = read_agents_source(&app)?;
    items.push(ConvertSourceItem {
        content_type: "agent".into(),
        label: "Subagents".into(),
        path: agent_path.display().to_string(),
        exists: agent_exists,
        content: agent_content,
    });

    Ok(items)
}

pub fn preview_convert_extended(
    source_app: &str,
    target_app: &str,
    content: &str,
    content_type: &str,
) -> BridgePreview {
    match content_type {
        "prompt" => bridge::preview_bridge(source_app, target_app, content),
        "mcp" => preview_mcp_bridge(source_app, target_app, content),
        "skill" => preview_skill_bridge(source_app, target_app, content),
        "command" => preview_command_bridge(source_app, target_app, content),
        "agent" => preview_agent_bridge(source_app, target_app, content),
        _ => BridgePreview {
            converted_content: content.to_string(),
            unmapped_sections: vec![],
            warnings: vec![format!("Unknown content type: {content_type}")],
        },
    }
}

pub fn apply_convert(req: &ConvertApplyRequest) -> Result<ConvertApplyResult, AppError> {
    let target = parse_app(&req.target_app)?;
    let preview = preview_convert_extended(
        &req.source_app,
        &req.target_app,
        &req.content,
        &req.content_type,
    );

    match req.content_type.as_str() {
        "prompt" => apply_prompt_convert(&target, &preview.converted_content, req.overwrite),
        "mcp" => apply_mcp_convert(&target, &preview.converted_content, req.overwrite),
        "skill" => apply_skill_convert(&target, &preview.converted_content, req.overwrite),
        "command" => apply_command_convert(&target, &preview.converted_content, req.overwrite),
        "agent" => apply_agent_convert(&target, &preview.converted_content, req.overwrite),
        other => Err(AppError::Config(format!("Unsupported content type: {other}"))),
    }
    .map(|written_paths| ConvertApplyResult {
        written_paths,
        warnings: preview.warnings,
    })
}

fn apply_prompt_convert(
    target: &AppType,
    content: &str,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    let path = prompt_files::prompt_file_path(target)?;
    if path.exists() && !overwrite {
        return Err(AppError::Config(format!(
            "目标文件已存在: {}。请确认覆盖后重试。",
            path.display()
        )));
    }
    backup_file(&path)?;
    write_text_file(&path, content)?;
    Ok(vec![path.display().to_string()])
}

fn apply_mcp_convert(
    target: &AppType,
    content: &str,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    match target {
        AppType::Claude => apply_mcp_to_claude(content, overwrite),
        AppType::Gemini => apply_mcp_to_gemini(content, overwrite),
        AppType::Codex => apply_mcp_to_codex(content, overwrite),
        AppType::OpenCode => apply_mcp_to_opencode(content, overwrite),
        AppType::Hermes => apply_mcp_to_hermes(content, overwrite),
        AppType::ClaudeDesktop | AppType::OpenClaw => Err(AppError::Config(
            "该目标工具暂不支持 MCP 转换写入".into(),
        )),
    }
}

fn apply_mcp_to_claude(content: &str, _overwrite: bool) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("MCP JSON 无效: {e}")))?;
    let servers = value
        .get("mcpServers")
        .cloned()
        .unwrap_or(value);
    let mut root = if let Ok(Some(existing)) = claude_mcp::read_mcp_json() {
        serde_json::from_str(&existing).unwrap_or(json!({}))
    } else {
        json!({})
    };
    if let Some(obj) = root.as_object_mut() {
        obj.insert("mcpServers".into(), servers);
    }
    let path = get_claude_mcp_path();
    backup_file(&path)?;
    let text = serde_json::to_string_pretty(&root)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    atomic_write(&path, text.as_bytes())?;
    Ok(vec![path.display().to_string()])
}

fn apply_mcp_to_gemini(content: &str, _overwrite: bool) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("MCP JSON 无效: {e}")))?;
    let servers_obj = value
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::Config("缺少 mcpServers 对象".into()))?;
    let mut map = std::collections::HashMap::new();
    for (k, v) in servers_obj {
        map.insert(k.clone(), v.clone());
    }
    gemini_mcp::set_mcp_servers_map(&map)?;
    let path = crate::gemini_config::get_gemini_settings_path();
    Ok(vec![path.display().to_string()])
}

fn apply_mcp_to_codex(content: &str, _overwrite: bool) -> Result<Vec<String>, AppError> {
    let path = crate::codex_config::get_codex_config_path();
    backup_file(&path)?;
    let mut doc = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
        if text.trim().is_empty() {
            DocumentMut::default()
        } else {
            text.parse::<DocumentMut>()
                .map_err(|e| AppError::Config(format!("解析 config.toml 失败: {e}")))?
        }
    } else {
        DocumentMut::default()
    };

    if content.trim_start().starts_with('{') {
        let value: Value = serde_json::from_str(content)
            .map_err(|e| AppError::Config(format!("MCP JSON 无效: {e}")))?;
        let servers = value
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .ok_or_else(|| AppError::Config("缺少 mcpServers".into()))?;
        let mut mcp_table = Table::new();
        for (id, spec) in servers {
            if let Ok(tbl) = json_mcp_server_to_toml_table(spec) {
                mcp_table.insert(id, Item::Table(tbl));
            }
        }
        doc["mcp_servers"] = Item::Table(mcp_table);
    } else {
        let parsed: DocumentMut = content
            .parse()
            .map_err(|e| AppError::Config(format!("MCP TOML 无效: {e}")))?;
        if let Some(Item::Table(tbl)) = parsed.get("mcp_servers") {
            doc["mcp_servers"] = Item::Table(tbl.clone());
        }
    }

    atomic_write(&path, doc.to_string().as_bytes())?;
    Ok(vec![path.display().to_string()])
}

fn apply_mcp_to_opencode(content: &str, _overwrite: bool) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("MCP JSON 无效: {e}")))?;
    let path = crate::opencode_config::get_opencode_config_path();
    backup_file(&path)?;
    let mut root = if path.exists() {
        crate::opencode_config::read_opencode_config()?
    } else {
        json!({})
    };
    let servers = value.get("mcpServers").cloned().unwrap_or(value);
    if let Some(obj) = root.as_object_mut() {
        obj.insert("mcp".into(), json!({ "servers": servers }));
    }
    crate::opencode_config::write_opencode_config(&root)?;
    Ok(vec![path.display().to_string()])
}

fn apply_mcp_to_hermes(content: &str, _overwrite: bool) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("MCP JSON 无效: {e}")))?;
    let path = crate::hermes_config::get_hermes_config_path();
    backup_file(&path)?;
    let servers = value.get("mcpServers").cloned().unwrap_or(value);
    let yaml = serde_yaml::to_string(&json!({ "mcp_servers": servers }))
        .map_err(|e| AppError::Config(format!("YAML 序列化失败: {e}")))?;
    // Merge into existing YAML is complex; for M2 append mcp_servers section via full replace of section
    let existing = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    let merged = if existing.trim().is_empty() {
        yaml
    } else {
        let section = mcp_servers_section_from_json(&servers)?;
        format!("{existing}\n\n# OpenSunstar convert\n{section}")
    };
    atomic_write(&path, merged.as_bytes())?;
    Ok(vec![path.display().to_string()])
}

fn mcp_servers_section_from_json(servers: &Value) -> Result<String, AppError> {
    serde_yaml::to_string(servers)
        .map_err(|e| AppError::Config(format!("YAML 失败: {e}")))
}

fn preview_mcp_bridge(source_app: &str, target_app: &str, content: &str) -> BridgePreview {
    let mut warnings = Vec::new();
    let converted = match (source_app, target_app) {
        (src, tgt) if src == "codex" || tgt == "codex" => {
            if src == "codex" && tgt != "codex" {
                match codex_toml_to_mcp_json(content) {
                    Ok(json) => json,
                    Err(e) => {
                        warnings.push(e.to_string());
                        content.to_string()
                    }
                }
            } else if tgt == "codex" && src != "codex" {
                match mcp_json_to_codex_toml(content) {
                    Ok(toml) => toml,
                    Err(e) => {
                        warnings.push(e.to_string());
                        content.to_string()
                    }
                }
            } else {
                content.to_string()
            }
        }
        _ => {
            // JSON reformat for readability
            match serde_json::from_str::<Value>(content) {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.to_string()),
                Err(_) => content.to_string(),
            }
        }
    };

    BridgePreview {
        converted_content: converted,
        unmapped_sections: vec![],
        warnings,
    }
}

fn codex_toml_to_mcp_json(content: &str) -> Result<String, AppError> {
    let doc: DocumentMut = content
        .parse()
        .map_err(|e| AppError::Config(format!("TOML 解析失败: {e}")))?;
    let mut servers = Map::new();
    if let Some(Item::Table(tbl)) = doc.get("mcp_servers") {
        for (id, item) in tbl.iter() {
            if let Item::Table(server) = item {
                let mut spec = Map::new();
                for (k, v) in server.iter() {
                    if let Some(json_v) = toml_item_to_json(v) {
                        spec.insert(k.to_string(), json_v);
                    }
                }
                servers.insert(id.to_string(), Value::Object(spec));
            }
        }
    }
    let root = json!({ "mcpServers": Value::Object(servers) });
    serde_json::to_string_pretty(&root).map_err(|e| AppError::JsonSerialize { source: e })
}

fn mcp_json_to_codex_toml(content: &str) -> Result<String, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("JSON 解析失败: {e}")))?;
    let servers = value
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::Config("缺少 mcpServers".into()))?;
    let mut doc = DocumentMut::new();
    let mut mcp_table = Table::new();
    for (id, spec) in servers {
        let tbl = json_mcp_server_to_toml_table(spec)?;
        mcp_table.insert(id, Item::Table(tbl));
    }
    doc["mcp_servers"] = Item::Table(mcp_table);
    Ok(doc.to_string())
}

fn json_mcp_server_to_toml_table(spec: &Value) -> Result<Table, AppError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| AppError::Config("MCP server 必须是对象".into()))?;
    let mut table = Table::new();
    for (k, v) in obj {
        if let Some(item) = json_value_to_toml_item(v) {
            table.insert(k, item);
        }
    }
    Ok(table)
}

fn json_value_to_toml_item(value: &Value) -> Option<Item> {
    match value {
        Value::String(s) => Some(Item::Value(s.clone().into())),
        Value::Bool(b) => Some(Item::Value((*b).into())),
        Value::Number(n) => n
            .as_i64()
            .map(|i| Item::Value(i.into()))
            .or_else(|| n.as_f64().map(|f| Item::Value((f as i64).into()))),
        Value::Array(arr) => {
            let mut array = toml_edit::Array::new();
            for v in arr {
                if let Value::String(s) = v {
                    array.push(s.as_str());
                }
            }
            Some(Item::Value(array.into()))
        }
        Value::Object(map) => {
            let mut tbl = Table::new();
            for (k, v) in map {
                if let Some(item) = json_value_to_toml_item(v) {
                    tbl.insert(k, item);
                }
            }
            Some(Item::Table(tbl))
        }
        _ => None,
    }
}

fn toml_item_to_json(item: &Item) -> Option<Value> {
    match item {
        Item::Value(v) => {
            if let Some(s) = v.as_str() {
                Some(Value::String(s.to_string()))
            } else if let Some(b) = v.as_bool() {
                Some(Value::Bool(b))
            } else if let Some(i) = v.as_integer() {
                Some(json!(i))
            } else if let Some(arr) = v.as_array() {
                let vals: Vec<Value> = arr
                    .iter()
                    .filter_map(|x| x.as_str().map(|s| Value::String(s.to_string())))
                    .collect();
                Some(Value::Array(vals))
            } else {
                None
            }
        }
        Item::Table(tbl) => {
            let mut map = Map::new();
            for (k, v) in tbl.iter() {
                if let Some(j) = toml_item_to_json(v) {
                    map.insert(k.to_string(), j);
                }
            }
            Some(Value::Object(map))
        }
        _ => None,
    }
}

fn read_mcp_source(app: &AppType) -> Result<(PathBuf, bool, Option<String>), AppError> {
    match app {
        AppType::Claude => {
            let path = get_claude_mcp_path();
            if let Ok(Some(text)) = claude_mcp::read_mcp_json() {
                Ok((path, true, Some(text)))
            } else {
                Ok((path, false, None))
            }
        }
        AppType::Gemini => {
            let path = crate::gemini_config::get_gemini_settings_path();
            if !path.exists() {
                return Ok((path, false, None));
            }
            let map = gemini_mcp::read_mcp_servers_map()?;
            let json = json!({ "mcpServers": map });
            let text = serde_json::to_string_pretty(&json)
                .map_err(|e| AppError::JsonSerialize { source: e })?;
            Ok((path, true, Some(text)))
        }
        AppType::Codex => {
            let path = crate::codex_config::get_codex_config_path();
            if !path.exists() {
                return Ok((path, false, None));
            }
            let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
            Ok((path, true, Some(text)))
        }
        AppType::OpenCode => {
            let path = crate::opencode_config::get_opencode_config_path();
            if !path.exists() {
                return Ok((path, false, None));
            }
            let cfg = crate::opencode_config::read_opencode_config()?;
            let mcp = cfg.get("mcp").cloned().unwrap_or(json!({}));
            let text = serde_json::to_string_pretty(&mcp)
                .map_err(|e| AppError::JsonSerialize { source: e })?;
            Ok((path, true, Some(text)))
        }
        AppType::Hermes => {
            let path = crate::hermes_config::get_hermes_config_path();
            if !path.exists() {
                return Ok((path, false, None));
            }
            let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
            Ok((path, true, Some(text)))
        }
        AppType::ClaudeDesktop | AppType::OpenClaw => Err(AppError::Config(
            "该源工具暂不支持 MCP 检测".into(),
        )),
    }
}

fn read_skills_source(app: &AppType) -> Result<(PathBuf, bool, Option<String>), AppError> {
    let path = SkillService::get_app_skills_dir(app)
        .map_err(|e| AppError::Config(e.to_string()))?;
    let skills = scan_skills_in_dir(&path)?;
    let exists = !skills.is_empty();
    let content = if exists {
        Some(skills_to_bundle(&skills)?)
    } else {
        None
    };
    Ok((path, exists, content))
}

fn read_commands_source(app: &AppType) -> Result<(PathBuf, bool, Option<String>), AppError> {
    let path = match commands_dir_for_app(app) {
        Ok(p) => p,
        Err(_) => {
            let placeholder = match app {
                AppType::Codex => dirs::home_dir()
                    .map(|h| h.join(".codex").join("commands"))
                    .unwrap_or_else(|| PathBuf::from("~/.codex/commands")),
                AppType::ClaudeDesktop => PathBuf::from("~/.claude-desktop/commands"),
                AppType::OpenClaw => PathBuf::from("~/.openclaw/commands"),
                _ => PathBuf::from("N/A"),
            };
            return Ok((placeholder, false, None));
        }
    };
    let commands = scan_commands_in_dir(&path)?;
    let exists = !commands.is_empty();
    let content = if exists {
        Some(commands_to_bundle(&commands)?)
    } else {
        None
    };
    Ok((path, exists, content))
}

fn read_agents_source(app: &AppType) -> Result<(PathBuf, bool, Option<String>), AppError> {
    let path = agents_dir_for_app(app)?;
    let agents = if matches!(app, AppType::Codex) {
        scan_agents_toml_in_dir(&path)?
    } else {
        scan_agents_md_in_dir(&path)?
    };
    let exists = !agents.is_empty();
    let content = if exists {
        Some(agents_to_bundle(&agents)?)
    } else {
        None
    };
    Ok((path, exists, content))
}

fn agents_dir_for_app(app: &AppType) -> Result<PathBuf, AppError> {
    match app {
        AppType::Claude => Ok(crate::config::get_claude_config_dir().join("agents")),
        AppType::Gemini => Ok(crate::gemini_config::get_gemini_dir().join("agents")),
        AppType::OpenCode => Ok(crate::opencode_config::get_opencode_dir().join("agents")),
        AppType::Codex => Ok(crate::codex_config::get_codex_config_dir().join("agents")),
        AppType::Hermes | AppType::ClaudeDesktop | AppType::OpenClaw => {
            Err(AppError::Config(format!("{app:?} 不支持 Subagent 检测")))
        }
    }
}

fn scan_agents_md_in_dir(agents_root: &Path) -> Result<Vec<(String, String)>, AppError> {
    scan_markdown_files_in_dir(agents_root)
}

fn scan_agents_toml_in_dir(agents_root: &Path) -> Result<Vec<(String, String)>, AppError> {
    let mut out = Vec::new();
    if !agents_root.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(agents_root).map_err(|e| AppError::io(agents_root, e))? {
        let entry = entry.map_err(|e| AppError::io(agents_root, e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
        out.push((name, content));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn scan_markdown_files_in_dir(root: &Path) -> Result<Vec<(String, String)>, AppError> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| AppError::io(root, e))? {
        let entry = entry.map_err(|e| AppError::io(root, e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
        out.push((name, content));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn agents_to_bundle(agents: &[(String, String)]) -> Result<String, AppError> {
    let arr: Vec<Value> = agents
        .iter()
        .map(|(name, content)| json!({ "name": name, "content": content }))
        .collect();
    serde_json::to_string_pretty(&json!({ "agents": arr }))
        .map_err(|e| AppError::JsonSerialize { source: e })
}

fn preview_agent_bridge(source_app: &str, target_app: &str, content: &str) -> BridgePreview {
    let mut warnings = Vec::new();
    let converted = if target_app == "codex" {
        warnings.push("Markdown Subagent 将转换为 Codex TOML（~/.codex/agents/*.toml）".into());
        match convert_agents_bundle_to_codex(content) {
            Ok(toml_bundle) => toml_bundle,
            Err(e) => {
                warnings.push(e.to_string());
                content.to_string()
            }
        }
    } else if source_app == "codex" {
        warnings.push("Codex TOML 源暂不支持自动转回 Markdown，请手动编辑".into());
        match serde_json::from_str::<Value>(content) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.to_string()),
            Err(_) => content.to_string(),
        }
    } else {
        warnings.push("Subagent 将按名称写入目标 agents/{name}.md".into());
        match serde_json::from_str::<Value>(content) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.to_string()),
            Err(_) => content.to_string(),
        }
    };
    BridgePreview {
        converted_content: converted,
        unmapped_sections: vec![],
        warnings,
    }
}

fn convert_agents_bundle_to_codex(content: &str) -> Result<String, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("Agent JSON 无效: {e}")))?;
    let agents = value
        .get("agents")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Config("缺少 agents 数组".into()))?;
    let mut out = Vec::new();
    for item in agents {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config("Agent 项缺少 name".into()))?;
        if item.get("format").and_then(|v| v.as_str()) == Some("toml") {
            let toml = item
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config(format!("Agent {name} 缺少 content")))?;
            out.push(json!({ "name": name, "format": "toml", "content": toml }));
            continue;
        }
        let md = item
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config(format!("Agent {name} 缺少 content")))?;
        let toml = markdown_agent_to_codex_toml(name, None, md)?;
        out.push(json!({ "name": name, "format": "toml", "content": toml }));
    }
    serde_json::to_string_pretty(&json!({ "agents": out }))
        .map_err(|e| AppError::JsonSerialize { source: e })
}

fn apply_agent_convert(
    target: &AppType,
    content: &str,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("Agent JSON 无效: {e}")))?;
    let agents = value
        .get("agents")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Config("缺少 agents 数组".into()))?;
    if agents.is_empty() {
        return Err(AppError::Config("没有可写入的 Subagent".into()));
    }

    let base = agents_dir_for_app(target)?;
    fs::create_dir_all(&base).map_err(|e| AppError::io(&base, e))?;
    let mut written = Vec::new();

    for item in agents {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config("Agent 项缺少 name".into()))?;
        crate::agent::validate_agent_name(name).map_err(AppError::Config)?;

        let path = if matches!(target, AppType::Codex) {
            base.join(format!("{name}.toml"))
        } else {
            base.join(format!("{name}.md"))
        };

        let payload = if matches!(target, AppType::Codex) {
            if item.get("format").and_then(|v| v.as_str()) == Some("toml") {
                item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Config(format!("Agent {name} 缺少 content")))?
                    .to_string()
            } else {
                let md = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Config(format!("Agent {name} 缺少 content")))?;
                markdown_agent_to_codex_toml(name, None, md)?
            }
        } else {
            item.get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config(format!("Agent {name} 缺少 content")))?
                .to_string()
        };

        if path.exists() && !overwrite {
            return Err(AppError::Config(format!(
                "目标文件已存在: {}。请确认覆盖后重试。",
                path.display()
            )));
        }
        backup_file(&path)?;
        write_text_file(&path, &payload)?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

fn commands_dir_for_app(app: &AppType) -> Result<PathBuf, AppError> {
    match app {
        AppType::Claude => Ok(crate::config::get_claude_config_dir().join("commands")),
        AppType::Gemini => Ok(crate::gemini_config::get_gemini_dir().join("commands")),
        AppType::OpenCode => Ok(crate::opencode_config::get_opencode_dir().join("commands")),
        AppType::Hermes => Ok(crate::hermes_config::get_hermes_dir().join("commands")),
        AppType::Codex => Err(AppError::Config(
            "Codex 不支持独立 slash 命令文件".into(),
        )),
        AppType::OpenClaw | AppType::ClaudeDesktop => Err(AppError::Config(format!(
            "{app:?} 不支持 slash 命令检测"
        ))),
    }
}

fn scan_skills_in_dir(skills_root: &Path) -> Result<Vec<(String, String)>, AppError> {
    let mut out = Vec::new();
    if !skills_root.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(skills_root).map_err(|e| AppError::io(skills_root, e))? {
        let entry = entry.map_err(|e| AppError::io(skills_root, e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            let directory = entry.file_name().to_string_lossy().to_string();
            let content =
                fs::read_to_string(&skill_md).map_err(|e| AppError::io(&skill_md, e))?;
            out.push((directory, content));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn scan_commands_in_dir(commands_root: &Path) -> Result<Vec<(String, String)>, AppError> {
    let mut out = Vec::new();
    if !commands_root.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(commands_root).map_err(|e| AppError::io(commands_root, e))? {
        let entry = entry.map_err(|e| AppError::io(commands_root, e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
        out.push((name, content));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn skills_to_bundle(skills: &[(String, String)]) -> Result<String, AppError> {
    let arr: Vec<Value> = skills
        .iter()
        .map(|(directory, content)| json!({ "directory": directory, "content": content }))
        .collect();
    serde_json::to_string_pretty(&json!({ "skills": arr }))
        .map_err(|e| AppError::JsonSerialize { source: e })
}

fn commands_to_bundle(commands: &[(String, String)]) -> Result<String, AppError> {
    let arr: Vec<Value> = commands
        .iter()
        .map(|(name, content)| json!({ "name": name, "content": content }))
        .collect();
    serde_json::to_string_pretty(&json!({ "commands": arr }))
        .map_err(|e| AppError::JsonSerialize { source: e })
}

fn preview_skill_bridge(_source_app: &str, _target_app: &str, content: &str) -> BridgePreview {
    let converted = match serde_json::from_str::<Value>(content) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.to_string()),
        Err(_) => content.to_string(),
    };
    BridgePreview {
        converted_content: converted,
        unmapped_sections: vec![],
        warnings: vec![
            "Skill 将按目录名写入目标 skills/{directory}/SKILL.md".into(),
        ],
    }
}

fn preview_command_bridge(source_app: &str, target_app: &str, content: &str) -> BridgePreview {
    let mut warnings = Vec::new();
    if target_app == "codex" {
        warnings.push("Codex 不支持独立 slash 命令文件，无法写入".into());
    }
    if source_app == "codex" {
        warnings.push("Codex 不支持作为命令源".into());
    }
    let converted = match serde_json::from_str::<Value>(content) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.to_string()),
        Err(_) => content.to_string(),
    };
    BridgePreview {
        converted_content: converted,
        unmapped_sections: vec![],
        warnings,
    }
}

fn apply_skill_convert(
    target: &AppType,
    content: &str,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("Skill JSON 无效: {e}")))?;
    let skills = value
        .get("skills")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Config("缺少 skills 数组".into()))?;
    if skills.is_empty() {
        return Err(AppError::Config("没有可写入的 Skill".into()));
    }

    let base = SkillService::get_app_skills_dir(target)
        .map_err(|e| AppError::Config(e.to_string()))?;
    let mut written = Vec::new();
    for item in skills {
        let directory = item
            .get("directory")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config("Skill 项缺少 directory".into()))?;
        if directory.contains('/') || directory.contains('\\') || directory.contains("..") {
            return Err(AppError::Config(format!(
                "非法 Skill 目录名: {directory}"
            )));
        }
        let skill_content = item
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config(format!("Skill {directory} 缺少 content")))?;

        let skill_dir = base.join(directory);
        let skill_md = skill_dir.join("SKILL.md");
        if skill_md.exists() && !overwrite {
            return Err(AppError::Config(format!(
                "目标文件已存在: {}。请确认覆盖后重试。",
                skill_md.display()
            )));
        }
        backup_file(&skill_md)?;
        fs::create_dir_all(&skill_dir).map_err(|e| AppError::io(&skill_dir, e))?;
        write_text_file(&skill_md, skill_content)?;
        written.push(skill_md.display().to_string());
    }
    Ok(written)
}

fn apply_command_convert(
    target: &AppType,
    content: &str,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    if matches!(target, AppType::Codex) {
        return Err(AppError::Config(
            "Codex 不支持独立 slash 命令文件".into(),
        ));
    }

    let value: Value = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("Command JSON 无效: {e}")))?;
    let commands = value
        .get("commands")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Config("缺少 commands 数组".into()))?;
    if commands.is_empty() {
        return Err(AppError::Config("没有可写入的 Command".into()));
    }

    let base = commands_dir_for_app(target)?;
    fs::create_dir_all(&base).map_err(|e| AppError::io(&base, e))?;
    let mut written = Vec::new();
    for item in commands {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config("Command 项缺少 name".into()))?;
        crate::command::validate_command_name(name).map_err(AppError::Config)?;
        let cmd_content = item
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Config(format!("Command {name} 缺少 content")))?;

        let path = base.join(format!("{name}.md"));
        if path.exists() && !overwrite {
            return Err(AppError::Config(format!(
                "目标文件已存在: {}。请确认覆盖后重试。",
                path.display()
            )));
        }
        backup_file(&path)?;
        write_text_file(&path, cmd_content)?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

fn backup_file(path: &Path) -> Result<Option<PathBuf>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let backup = path.with_extension("bak.opensunstar");
    fs::copy(path, &backup).map_err(|e| AppError::io(path, e))?;
    Ok(Some(backup))
}

fn parse_app(app: &str) -> Result<AppType, AppError> {
    app.parse::<AppType>()
        .map_err(|e| AppError::Config(e.to_string()))
}

fn prompt_filename(app: &AppType) -> &'static str {
    match app {
        AppType::Claude => "CLAUDE.md",
        AppType::Gemini => "GEMINI.md",
        AppType::Codex | AppType::OpenCode | AppType::OpenClaw | AppType::Hermes => "AGENTS.md",
        AppType::ClaudeDesktop => "N/A",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_json_to_toml_roundtrip_shape() {
        let json = r#"{"mcpServers":{"fs":{"command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","/tmp"]}}}"#;
        let toml = mcp_json_to_codex_toml(json).expect("toml");
        assert!(toml.contains("mcp_servers"));
        assert!(toml.contains("fs"));
    }

    #[test]
    fn skill_bundle_roundtrip() {
        let bundle = r#"{"skills":[{"directory":"demo","content":"Demo Skill"}]}"#;
        let preview = preview_skill_bridge("claude", "gemini", bundle);
        assert!(preview.converted_content.contains("demo"));
        assert!(!preview.warnings.is_empty());
    }

    #[test]
    fn command_preview_warns_codex_target() {
        let bundle = r#"{"commands":[{"name":"review-pr","content":"Review this PR"}]}"#;
        let preview = preview_command_bridge("claude", "codex", bundle);
        assert!(preview
            .warnings
            .iter()
            .any(|w| w.contains("Codex")));
    }

    #[test]
    fn agent_bundle_converts_to_codex_toml() {
        let bundle = r#"{"agents":[{"name":"reviewer","content":"---\nname: reviewer\ndescription: Review\n---\nReview code."}]}"#;
        let preview = preview_agent_bridge("claude", "codex", bundle);
        assert!(preview.converted_content.contains("\"format\": \"toml\""));
        assert!(preview.converted_content.contains("developer_instructions"));
    }
}
