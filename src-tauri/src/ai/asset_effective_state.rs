//! 资产「生效态」诊断：比对 OpenSunstar 库内容与各 CLI 目录实际文件（规范化 hash）
//!
//! P0+：按需 stat/读文件，不做 fs watcher。覆盖 MCP / Prompts / Ignore / Permissions / Hooks /
//! Skills / Commands / Subagents。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::agent::agent_sync_supported;
use crate::ai::agent_readiness::STATUS_MISSING;
use crate::ai::asset_app_support::{asset_support, normalize_target_app, AssetSupport};
use crate::ai::types::AgentReadinessItem;
use crate::app_config::AppType;
use crate::codex_config::get_codex_config_dir;
use crate::database::Database;
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::hermes_config::get_hermes_dir;
use crate::opencode_config::get_opencode_dir;
use crate::services::agent_codex::markdown_agent_to_codex_toml;
use crate::services::claude_settings::ClaudeSettingsMerger;
use crate::services::prompt::PromptService;
use crate::services::skill::SkillService;
use crate::store::AppState;

pub const CONFIGURED: &str = "configured";
pub const UNCONFIGURED: &str = "unconfigured";
pub const EFFECTIVE: &str = "effective";
pub const DRIFTED: &str = "drifted";
pub const UNCHECKED: &str = "unchecked";
pub const NOT_APPLICABLE: &str = "not_applicable";

/// 单项生效态扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveItemState {
    pub check_name: String,
    pub configured_state: String,
    pub effective_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_path: Option<String>,
}

/// 全量生效态扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveScanResult {
    pub scanned_at: i64,
    pub target_app: String,
    pub items: Vec<EffectiveItemState>,
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 规范化文本后 SHA-256（统一换行、去除行尾空白）
pub fn canonical_text_hash(text: &str) -> String {
    let normalized: String = text
        .replace("\r\n", "\n")
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 递归排序 JSON 键后 hash
pub fn canonical_json_hash(value: &Value) -> String {
    canonical_text_hash(&serde_json::to_string(&normalize_json_value(value)).unwrap_or_default())
}

fn normalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                out.insert(k.clone(), normalize_json_value(&map[&k]));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_json_value).collect()),
        other => other.clone(),
    }
}

pub fn derive_configured_state(item: &AgentReadinessItem) -> &'static str {
    match item.status.as_deref() {
        Some(STATUS_MISSING) => UNCONFIGURED,
        Some(_) if item.score > 0 => CONFIGURED,
        _ if item.score > 0 => CONFIGURED,
        _ => UNCONFIGURED,
    }
}

fn compare_text(expected: &str, actual: &str) -> bool {
    canonical_text_hash(expected) == canonical_text_hash(actual)
}

fn compare_json(expected: &Value, actual: &Value) -> bool {
    canonical_json_hash(expected) == canonical_json_hash(actual)
}

fn effective_from_text(
    check_name: &str,
    configured: &str,
    expected: &str,
    live_path: &std::path::Path,
    support: AssetSupport,
) -> EffectiveItemState {
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: NOT_APPLICABLE.to_string(),
            effective_detail: Some("当前目标 CLI 不支持此项文件写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: UNCONFIGURED.to_string(),
            effective_state: NOT_APPLICABLE.to_string(),
            effective_detail: None,
            live_path: None,
        };
    }

    let actual = if live_path.is_file() {
        std::fs::read_to_string(live_path).unwrap_or_default()
    } else {
        String::new()
    };

    let path_str = live_path.display().to_string();
    if compare_text(expected, &actual) {
        EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: EFFECTIVE.to_string(),
            effective_detail: None,
            live_path: Some(path_str),
        }
    } else {
        EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: DRIFTED.to_string(),
            effective_detail: Some("磁盘文件与 OpenSunstar 库内容不一致（可能被外部修改）".into()),
            live_path: Some(path_str),
        }
    }
}

fn effective_from_json_field(
    check_name: &str,
    configured: &str,
    expected: &Value,
    field_name: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: NOT_APPLICABLE.to_string(),
            effective_detail: Some("当前目标 CLI 不支持此项".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: UNCONFIGURED.to_string(),
            effective_state: NOT_APPLICABLE.to_string(),
            effective_detail: None,
            live_path: None,
        };
    }

    let settings = ClaudeSettingsMerger::read_settings_or_default();
    let actual = settings.get(field_name).cloned().unwrap_or(json!({}));
    let path = crate::config::get_claude_settings_path();
    if compare_json(expected, &actual) {
        EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: EFFECTIVE.to_string(),
            effective_detail: None,
            live_path: Some(path.display().to_string()),
        }
    } else {
        EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: DRIFTED.to_string(),
            effective_detail: Some(format!(
                "settings.json 中 {field_name} 与 OpenSunstar 库不一致"
            )),
            live_path: Some(path.display().to_string()),
        }
    }
}

fn ignore_file_path(app: &AppType) -> Result<std::path::PathBuf, AppError> {
    use crate::codex_config::get_codex_auth_path;
    use crate::config::get_claude_config_dir;
    use crate::gemini_config::get_gemini_dir;
    use crate::hermes_config::get_hermes_dir;
    use crate::opencode_config::get_opencode_dir;

    let path = match app {
        AppType::Claude => get_claude_config_dir().join(".claudeignore"),
        AppType::Codex => get_codex_auth_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".codex"))
            .join(".codexignore"),
        AppType::Gemini => get_gemini_dir().join(".geminiignore"),
        AppType::OpenCode => get_opencode_dir().join(".opencodeignore"),
        AppType::Hermes => get_hermes_dir().join(".hermesignore"),
        AppType::OpenClaw | AppType::ClaudeDesktop => {
            return Err(AppError::Config(format!("{app:?} 不支持 ignore 文件同步")));
        }
    };
    Ok(path)
}

fn expected_ignore_content(db: &Database, app: &AppType) -> Result<String, AppError> {
    let rules = db.get_all_ignore_rules()?;
    let patterns: Vec<&str> = rules
        .iter()
        .filter(|r| r.is_enabled_for(app))
        .map(|r| r.pattern.as_str())
        .collect();
    Ok(if patterns.is_empty() {
        String::new()
    } else {
        patterns.join("\n") + "\n"
    })
}

fn expected_permissions_json(db: &Database) -> Result<Value, AppError> {
    let perms = db.get_all_tool_permissions()?;
    let mut allow: Vec<String> = Vec::new();
    let mut deny: Vec<String> = Vec::new();
    for perm in perms.into_iter().filter(|p| p.enabled_claude) {
        match perm.permission_type.as_str() {
            "allowedTools" | "autoApprove" => allow.push(perm.tool_pattern),
            "deniedTools" => deny.push(perm.tool_pattern),
            _ => {}
        }
    }
    allow.sort();
    allow.dedup();
    deny.sort();
    deny.dedup();
    Ok(json!({
        "allow": allow,
        "deny": deny,
        "additionalDirectories": []
    }))
}

fn expected_hooks_json(db: &Database) -> Result<Value, AppError> {
    let hooks = db
        .get_all_hooks()?
        .into_iter()
        .filter(|h| h.enabled_claude)
        .collect::<Vec<_>>();
    let mut hooks_map: Map<String, Value> = Map::new();
    for hook in hooks {
        let entry = json!({
            "matcher": hook.tool_pattern,
            "hooks": [{
                "type": "command",
                "command": hook.hook_command,
                "timeout": hook.timeout_seconds
            }]
        });
        hooks_map
            .entry(hook.event_type.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("hooks array")
            .push(entry);
    }
    Ok(Value::Object(hooks_map))
}

fn expected_mcp_map(db: &Database, app: &AppType) -> Result<HashMap<String, Value>, AppError> {
    let servers = db.get_all_mcp_servers()?;
    let mut map = HashMap::new();
    for (id, server) in servers {
        if server.apps.is_enabled_for(app) {
            map.insert(id, server.server.clone());
        }
    }
    Ok(map)
}

fn read_live_mcp_map(app: &AppType) -> Result<HashMap<String, Value>, AppError> {
    match app {
        AppType::Claude => crate::claude_mcp::read_mcp_servers_map(),
        AppType::Gemini => crate::gemini_mcp::read_mcp_servers_map(),
        AppType::OpenCode => {
            let map = crate::opencode_config::get_mcp_servers()?;
            Ok(map.into_iter().collect())
        }
        AppType::Codex | AppType::Hermes | AppType::OpenClaw | AppType::ClaudeDesktop => {
            Ok(HashMap::new())
        }
    }
}

fn mcp_map_to_json(map: &HashMap<String, Value>) -> Value {
    let mut keys: Vec<_> = map.keys().cloned().collect();
    keys.sort();
    let mut obj = Map::new();
    for k in keys {
        obj.insert(k.clone(), normalize_json_value(&map[&k]));
    }
    Value::Object(obj)
}

fn scan_mcp(
    db: &Database,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    let check_name = "mcp_enabled";
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 MCP 文件写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: None,
            live_path: None,
        };
    }
    if !matches!(app, AppType::Claude | AppType::Gemini | AppType::OpenCode) {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: UNCHECKED.into(),
            effective_detail: Some("该 CLI 的 MCP 生效态比对暂未实现（TOML/YAML）".into()),
            live_path: None,
        };
    }

    let expected = expected_mcp_map(db, app).unwrap_or_default();
    let actual = read_live_mcp_map(app).unwrap_or_default();
    let live_path = match app {
        AppType::Claude => Some(crate::config::get_claude_mcp_path()),
        AppType::Gemini => Some(crate::gemini_config::get_gemini_settings_path()),
        AppType::OpenCode => Some(crate::opencode_config::get_opencode_dir().join("opencode.json")),
        _ => None,
    };

    let exp_json = mcp_map_to_json(&expected);
    let act_json = mcp_map_to_json(&actual);
    let path_str = live_path.as_ref().map(|p| p.display().to_string());

    if compare_json(&exp_json, &act_json) {
        EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: EFFECTIVE.into(),
            effective_detail: None,
            live_path: path_str,
        }
    } else {
        EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: DRIFTED.into(),
            effective_detail: Some("MCP 配置文件与 OpenSunstar 库不一致".into()),
            live_path: path_str,
        }
    }
}

fn expected_prompt_content(state: &AppState, app: &AppType) -> Result<String, AppError> {
    let prompts = state.db.get_prompts(app.as_str())?;
    if let Some(prompt) = prompts.values().find(|p| p.enabled && !p.is_fragment) {
        return PromptService::resolve_effective_content(state, app, prompt);
    }
    Ok(String::new())
}

fn scan_prompt(
    state: &AppState,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    let check_name = "prompt_files";
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 Prompt 文件写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: None,
            live_path: None,
        };
    }

    let path = match crate::prompt_files::prompt_file_path(app) {
        Ok(p) => p,
        Err(e) => {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: None,
            };
        }
    };

    let expected = expected_prompt_content(state, app).unwrap_or_default();
    effective_from_text(check_name, configured, &expected, &path, AssetSupport::Supported)
}

fn scan_ignore(
    db: &Database,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: "ignore_rules".into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 Ignore 写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: "ignore_rules".into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: None,
            live_path: None,
        };
    }

    let path = match ignore_file_path(app) {
        Ok(p) => p,
        Err(e) => {
            return EffectiveItemState {
                check_name: "ignore_rules".into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: None,
            };
        }
    };
    let expected = expected_ignore_content(db, app).unwrap_or_default();
    effective_from_text("ignore_rules", configured, &expected, &path, AssetSupport::Supported)
}

fn scan_permissions(db: &Database, configured: &str, support: AssetSupport) -> EffectiveItemState {
    let expected = expected_permissions_json(db).unwrap_or(json!({}));
    effective_from_json_field("permissions", configured, &expected, "permissions", support)
}

fn scan_hooks(db: &Database, configured: &str, support: AssetSupport) -> EffectiveItemState {
    let expected = expected_hooks_json(db).unwrap_or(json!({}));
    effective_from_json_field("hooks_configured", configured, &expected, "hooks", support)
}

fn aggregate_effective_state(
    check_name: &str,
    configured: &str,
    live_path: Option<String>,
    drifted: Vec<String>,
    empty_ok_detail: &str,
) -> EffectiveItemState {
    if drifted.is_empty() {
        EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: EFFECTIVE.to_string(),
            effective_detail: if live_path.is_some() {
                None
            } else {
                Some(empty_ok_detail.to_string())
            },
            live_path,
        }
    } else {
        let count = drifted.len();
        let preview: String = drifted.into_iter().take(3).collect::<Vec<_>>().join("、");
        let suffix = if count > 3 { "…" } else { "" };
        EffectiveItemState {
            check_name: check_name.to_string(),
            configured_state: configured.to_string(),
            effective_state: DRIFTED.to_string(),
            effective_detail: Some(format!(
                "以下项与 OpenSunstar 库不一致或未同步：{preview}{suffix}"
            )),
            live_path,
        }
    }
}

fn command_file_path(name: &str, app: &AppType) -> Result<std::path::PathBuf, AppError> {
    let safe_name = format!("{name}.md");
    Ok(match app {
        AppType::Claude => crate::config::get_claude_config_dir()
            .join("commands")
            .join(&safe_name),
        AppType::Gemini => get_gemini_dir().join("commands").join(&safe_name),
        AppType::OpenCode => get_opencode_dir().join("commands").join(&safe_name),
        AppType::Hermes => get_hermes_dir().join("commands").join(&safe_name),
        AppType::Codex | AppType::OpenClaw | AppType::ClaudeDesktop => {
            return Err(AppError::Config(format!("{app:?} 不支持 slash 命令文件路径")));
        }
    })
}

fn commands_live_root(app: &AppType) -> Result<std::path::PathBuf, AppError> {
    command_file_path("_probe", app).map(|p| {
        p.parent()
            .map(|d| d.to_path_buf())
            .unwrap_or_else(|| p.clone())
    })
}

fn agent_file_path(name: &str, app: &AppType) -> Result<std::path::PathBuf, AppError> {
    if !agent_sync_supported(app) {
        return Err(AppError::Config(format!("{app:?} 不支持 Subagent 文件路径")));
    }
    Ok(match app {
        AppType::Claude => crate::config::get_claude_config_dir()
            .join("agents")
            .join(format!("{name}.md")),
        AppType::Gemini => get_gemini_dir().join("agents").join(format!("{name}.md")),
        AppType::OpenCode => get_opencode_dir()
            .join("agents")
            .join(format!("{name}.md")),
        AppType::Codex => get_codex_config_dir()
            .join("agents")
            .join(format!("{name}.toml")),
        _ => return Err(AppError::Config(format!("{app:?} 不支持 Subagent 文件路径"))),
    })
}

fn agents_live_root(app: &AppType) -> Result<std::path::PathBuf, AppError> {
    agent_file_path("_probe", app).map(|p| {
        p.parent()
            .map(|d| d.to_path_buf())
            .unwrap_or_else(|| p.clone())
    })
}

fn expected_agent_payload(
    name: &str,
    description: Option<&str>,
    content: &str,
    app: &AppType,
) -> Result<String, AppError> {
    if matches!(app, AppType::Codex) {
        markdown_agent_to_codex_toml(name, description, content)
    } else {
        Ok(content.to_string())
    }
}

fn scan_skills(
    db: &Database,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    let check_name = "skills_configured";
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 Skills 写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: None,
            live_path: None,
        };
    }

    let live_root = match SkillService::get_app_skills_dir(app) {
        Ok(p) => p,
        Err(e) => {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: None,
            };
        }
    };
    let ssot_dir = match SkillService::get_ssot_dir() {
        Ok(p) => p,
        Err(e) => {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: None,
            };
        }
    };

    let enabled: Vec<_> = db
        .get_all_installed_skills()
        .unwrap_or_default()
        .into_values()
        .filter(|s| s.apps.is_enabled_for(app))
        .collect();

    let mut drifted = Vec::new();
    for skill in &enabled {
        let source = ssot_dir.join(&skill.directory);
        let dest = live_root.join(&skill.directory);
        if !dest.exists() {
            drifted.push(skill.name.clone());
            continue;
        }
        let source_hash = SkillService::compute_dir_hash(&source).ok();
        let dest_hash = SkillService::compute_dir_hash(&dest).ok();
        if source_hash != dest_hash {
            drifted.push(skill.name.clone());
        }
    }

    aggregate_effective_state(
        check_name,
        configured,
        Some(live_root.display().to_string()),
        drifted,
        "无已启用的 Skills，CLI 目录为空或未同步",
    )
}

fn scan_commands(
    db: &Database,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    let check_name = "commands_configured";
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 Commands 写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: None,
            live_path: None,
        };
    }

    let live_root = match commands_live_root(app) {
        Ok(p) => p,
        Err(e) => {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: None,
            };
        }
    };

    let enabled: Vec<_> = db
        .get_all_commands()
        .unwrap_or_default()
        .into_values()
        .filter(|c| c.is_enabled_for(app))
        .collect();

    let mut drifted = Vec::new();
    for cmd in &enabled {
        let path = match command_file_path(&cmd.name, app) {
            Ok(p) => p,
            Err(_) => {
                drifted.push(cmd.name.clone());
                continue;
            }
        };
        let actual = if path.is_file() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        if !compare_text(&cmd.content, &actual) {
            drifted.push(cmd.name.clone());
        }
    }

    aggregate_effective_state(
        check_name,
        configured,
        Some(live_root.display().to_string()),
        drifted,
        "无已启用的 Commands",
    )
}

fn scan_subagents(
    db: &Database,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
) -> EffectiveItemState {
    let check_name = "subagents_configured";
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 Subagent 文件写回".into()),
            live_path: None,
        };
    }
    if configured == UNCONFIGURED {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: None,
            live_path: None,
        };
    }
    if !agent_sync_supported(app) {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持 Subagent 文件写回".into()),
            live_path: None,
        };
    }

    let live_root = match agents_live_root(app) {
        Ok(p) => p,
        Err(e) => {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: None,
            };
        }
    };

    let enabled: Vec<_> = db
        .get_all_agents()
        .unwrap_or_default()
        .into_values()
        .filter(|a| a.enabled_apps().contains(app))
        .collect();

    let mut drifted = Vec::new();
    for agent in &enabled {
        let path = match agent_file_path(&agent.name, app) {
            Ok(p) => p,
            Err(_) => {
                drifted.push(agent.name.clone());
                continue;
            }
        };
        let expected = expected_agent_payload(
            &agent.name,
            agent.description.as_deref(),
            &agent.content,
            app,
        )
        .unwrap_or_default();
        let actual = if path.is_file() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        if !compare_text(&expected, &actual) {
            drifted.push(agent.name.clone());
        }
    }

    let empty_detail = if matches!(support, AssetSupport::Partial) {
        "无已启用的 Subagents（Codex 写入 TOML 格式）"
    } else {
        "无已启用的 Subagents"
    };

    aggregate_effective_state(
        check_name,
        configured,
        Some(live_root.display().to_string()),
        drifted,
        empty_detail,
    )
}

fn unchecked_item(check_name: &str, configured: &str, reason: &str) -> EffectiveItemState {
    EffectiveItemState {
        check_name: check_name.into(),
        configured_state: configured.into(),
        effective_state: if configured == UNCONFIGURED {
            NOT_APPLICABLE.into()
        } else {
            UNCHECKED.into()
        },
        effective_detail: if configured == UNCONFIGURED {
            None
        } else {
            Some(reason.into())
        },
        live_path: None,
    }
}

fn target_app_to_type(target_app: &str) -> AppType {
    match normalize_target_app(Some(target_app)) {
        "codex" => AppType::Codex,
        "gemini" => AppType::Gemini,
        "opencode" => AppType::OpenCode,
        "openclaw" => AppType::OpenClaw,
        "hermes" => AppType::Hermes,
        _ => AppType::Claude,
    }
}

/// 扫描全部 readiness 检查项的生效态，并与 readiness 明细合并
pub fn scan_effective_states(
    state: &AppState,
    readiness_details: &[AgentReadinessItem],
    target_app: Option<&str>,
) -> EffectiveScanResult {
    let app_id = normalize_target_app(target_app);
    let app = target_app_to_type(app_id);
    let mut items = Vec::with_capacity(readiness_details.len());

    for detail in readiness_details {
        let configured = derive_configured_state(detail);
        let asset_type = crate::ai::asset_app_support::check_name_to_asset_type(&detail.check_name);
        let support = asset_type
            .map(|t| asset_support(t, app_id))
            .unwrap_or(AssetSupport::Supported);

        let item = match detail.check_name.as_str() {
            "mcp_enabled" => scan_mcp(&state.db, &app, configured, support),
            "prompt_files" => scan_prompt(state, &app, configured, support),
            "ignore_rules" => scan_ignore(&state.db, &app, configured, support),
            "permissions" => scan_permissions(&state.db, configured, support),
            "hooks_configured" => scan_hooks(&state.db, configured, support),
            "recent_updates" => EffectiveItemState {
                check_name: detail.check_name.clone(),
                configured_state: configured.to_string(),
                effective_state: NOT_APPLICABLE.to_string(),
                effective_detail: Some("维护度指标无磁盘生效态".into()),
                live_path: None,
            },
            "skills_configured" => scan_skills(&state.db, &app, configured, support),
            "commands_configured" => scan_commands(&state.db, &app, configured, support),
            "subagents_configured" => scan_subagents(&state.db, &app, configured, support),
            other => unchecked_item(other, configured, "暂未实现生效态扫描"),
        };
        items.push(item);
    }

    EffectiveScanResult {
        scanned_at: now_ts(),
        target_app: app_id.to_string(),
        items,
    }
}

/// 将生效态扫描结果合并进 readiness 明细
pub fn merge_effective_into_details(
    details: &mut [AgentReadinessItem],
    scan: &EffectiveScanResult,
) {
    let map: HashMap<&str, &EffectiveItemState> = scan
        .items
        .iter()
        .map(|i| (i.check_name.as_str(), i))
        .collect();
    for item in details.iter_mut() {
        if let Some(eff) = map.get(item.check_name.as_str()) {
            item.configured_state = Some(eff.configured_state.clone());
            item.effective_state = Some(eff.effective_state.clone());
            item.effective_detail = eff.effective_detail.clone();
            item.effective_scanned_at = Some(scan.scanned_at);
            item.live_path = eff.live_path.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_text_ignores_trailing_whitespace() {
        assert_eq!(
            canonical_text_hash("hello\nworld  \n"),
            canonical_text_hash("hello\nworld\n")
        );
    }

    #[test]
    fn canonical_json_sorts_keys() {
        let a = json!({"b": 1, "a": 2});
        let b = json!({"a": 2, "b": 1});
        assert_eq!(canonical_json_hash(&a), canonical_json_hash(&b));
    }

    #[test]
    fn derive_configured_from_missing() {
        let item = AgentReadinessItem {
            check_name: "mcp_enabled".into(),
            label: "MCP".into(),
            weight: 15,
            score: 0,
            detail: String::new(),
            status: Some(STATUS_MISSING.into()),
            configured_state: None,
            effective_state: None,
            effective_detail: None,
            effective_scanned_at: None,
            live_path: None,
        };
        assert_eq!(derive_configured_state(&item), UNCONFIGURED);
    }

    #[test]
    fn aggregate_effective_when_no_drift() {
        let state = aggregate_effective_state(
            "skills_configured",
            CONFIGURED,
            Some("/tmp/skills".into()),
            vec![],
            "empty",
        );
        assert_eq!(state.effective_state, EFFECTIVE);
    }

    #[test]
    fn aggregate_drifted_lists_names() {
        let state = aggregate_effective_state(
            "commands_configured",
            CONFIGURED,
            None,
            vec!["a".into(), "b".into()],
            "empty",
        );
        assert_eq!(state.effective_state, DRIFTED);
        assert!(state.effective_detail.unwrap().contains("a、b"));
    }
}
