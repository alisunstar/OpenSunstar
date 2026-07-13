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
use crate::services::marker_merge::{
    extract_markdown_section, strip_managed_ignore_marker, strip_managed_subagent_marker,
    PROMPT_SECTION_ID,
};
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

/// 单项漂移修复结果（写回 + 复扫验证）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairAssetDriftResult {
    pub check_name: String,
    pub before_state: String,
    pub after_state: String,
    pub repaired: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_path: Option<String>,
    pub scanned_at: i64,
}

/// 项目级批量漂移修复结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairProjectDriftResult {
    pub repaired_count: u32,
    pub still_drifted_count: u32,
    pub items: Vec<RepairAssetDriftResult>,
    pub scanned_at: i64,
}

/// 漂移修复预览条目（展示当前磁盘内容供用户确认）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPreviewItem {
    pub check_name: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_detail: Option<String>,
    /// 当前磁盘文件内容摘要（截断至 800 字符）
    pub current_content: String,
    pub is_safety_critical: bool,
}

/// 漂移修复预览结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPreviewResult {
    pub items: Vec<RepairPreviewItem>,
    pub total_drifted: u32,
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

#[allow(dead_code)]
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
    ctx: EffectiveScanContext<'_>,
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

    let use_project = matches!(app, AppType::Claude)
        && ctx
            .project_id
            .is_some_and(|id| db.count_enabled_project_mcp(id).unwrap_or(0) > 0)
        && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let raw_expected = expected_project_mcp_map(db, project_id).unwrap_or_default();
        let live_path = crate::prompt_files::project_mcp_json_path(project_root);
        let expected = crate::claude_mcp::sanitized_mcp_servers_map(&raw_expected, &live_path)
            .unwrap_or(raw_expected);
        let actual =
            crate::claude_mcp::read_project_mcp_servers_map(project_root).unwrap_or_default();
        let exp_json = mcp_map_to_json(&expected);
        let act_json = mcp_map_to_json(&actual);
        let path_str = live_path.display().to_string();
        if compare_json(&exp_json, &act_json) {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: EFFECTIVE.into(),
                effective_detail: None,
                live_path: Some(path_str),
            };
        }
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: DRIFTED.into(),
            effective_detail: Some("项目 .mcp.json 与 OpenSunstar 库不一致".into()),
            live_path: Some(path_str),
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
    ctx: EffectiveScanContext<'_>,
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

    let use_project = ctx.project_id.is_some_and(|id| {
        state
            .db
            .get_project_prompts(id)
            .map(|links| {
                links
                    .iter()
                    .any(|l| l.enabled && l.prompt_app_type == app.as_str())
            })
            .unwrap_or(false)
    }) && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let path = match crate::prompt_files::project_prompt_file_path(project_root, app) {
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
        let expected = expected_project_prompt_body(state, project_id, app).unwrap_or_default();
        return effective_from_managed_text(check_name, configured, &expected, &path, support);
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
    effective_from_managed_text(check_name, configured, &expected, &path, support)
}

fn scan_ignore(
    db: &Database,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
    ctx: EffectiveScanContext<'_>,
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

    // 项目级 ignore：当存在项目上下文时，额外检查项目根目录的 ignore 文件
    let project_result =
        if let (Some(project_id), Some(project_path)) = (ctx.project_id, ctx.project_path) {
            let project_root = std::path::Path::new(project_path);
            match crate::prompt_files::project_ignore_file_path(project_root, app) {
                Ok(proj_path) => {
                    let proj_expected =
                        expected_project_ignore_content(db, project_id, app).unwrap_or_default();
                    if proj_expected.is_empty() && !proj_path.is_file() {
                        None
                    } else {
                        // 读取磁盘文件并剥离管理标记后再比较
                        let proj_actual = if proj_path.is_file() {
                            strip_managed_ignore_marker(
                                &std::fs::read_to_string(&proj_path).unwrap_or_default(),
                            )
                        } else {
                            String::new()
                        };
                        let proj_effective = if compare_text(&proj_expected, &proj_actual) {
                            EFFECTIVE.to_string()
                        } else {
                            DRIFTED.to_string()
                        };
                        let proj_detail = if proj_effective == DRIFTED {
                            Some("磁盘文件与 OpenSunstar 库内容不一致（可能被外部修改）".into())
                        } else {
                            None
                        };
                        Some(EffectiveItemState {
                            check_name: "ignore_rules".into(),
                            configured_state: configured.into(),
                            effective_state: proj_effective,
                            effective_detail: proj_detail,
                            live_path: Some(proj_path.display().to_string()),
                        })
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

    // 全局 ignore：读取磁盘文件并剥离管理标记后再比较
    let actual_global = if path.is_file() {
        strip_managed_ignore_marker(&std::fs::read_to_string(&path).unwrap_or_default())
    } else {
        String::new()
    };
    let global_effective = if compare_text(&expected, &actual_global) {
        EFFECTIVE.to_string()
    } else {
        DRIFTED.to_string()
    };
    let global_detail = if global_effective == DRIFTED {
        Some("磁盘文件与 OpenSunstar 库内容不一致（可能被外部修改）".into())
    } else {
        None
    };
    let mut global = EffectiveItemState {
        check_name: "ignore_rules".into(),
        configured_state: configured.into(),
        effective_state: global_effective,
        effective_detail: global_detail,
        live_path: Some(path.display().to_string()),
    };

    // 项目级 ignore 结果合并：drifted > effective，并附加项目路径信息
    if let Some(proj) = project_result {
        let proj_drifted = proj.effective_state == DRIFTED;
        let proj_live = proj.live_path.clone();
        if proj_drifted {
            global = proj;
        }
        if let Some(ref proj_live_path) = proj_live {
            global.live_path = Some(format!(
                "全局: {} | 项目: {}",
                path.display(),
                proj_live_path
            ));
        }
    }

    global
}

/// 项目级期望 ignore 内容（仅包含关联到该项目的规则）
fn expected_project_ignore_content(
    db: &Database,
    project_id: &str,
    app: &AppType,
) -> Result<String, AppError> {
    let rules = db.get_all_ignore_rules()?;
    let links = db
        .get_project_asset_links(project_id, Some(crate::database::ASSET_IGNORE))
        .unwrap_or_default();
    let linked_ids: std::collections::HashSet<&str> = links
        .iter()
        .filter(|l| l.enabled)
        .map(|l| l.asset_id.as_str())
        .collect();

    if linked_ids.is_empty() {
        return Ok(String::new());
    }

    let patterns: Vec<&str> = rules
        .iter()
        .filter(|r| linked_ids.contains(r.id.as_str()) && r.is_enabled_for(app))
        .map(|r| r.pattern.as_str())
        .collect();
    Ok(if patterns.is_empty() {
        String::new()
    } else {
        patterns.join("\n") + "\n"
    })
}

fn scan_permissions(
    state: &AppState,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
    ctx: EffectiveScanContext<'_>,
) -> EffectiveItemState {
    let use_project = ctx.project_id.is_some_and(|id| {
        crate::services::project_config_sync::project_has_asset_links(
            &state.db,
            id,
            "permission",
            app,
        )
    }) && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let config_path = match app {
            AppType::Claude => crate::prompt_files::project_claude_settings_path(project_root),
            AppType::Codex => crate::prompt_files::project_codex_config_path(project_root),
            AppType::Gemini => crate::prompt_files::project_gemini_settings_path(project_root),
            AppType::OpenCode => crate::prompt_files::project_opencode_config_path(project_root),
            AppType::Hermes => crate::prompt_files::project_hermes_config_path(project_root),
            _ => {
                return EffectiveItemState {
                    check_name: "permissions".into(),
                    configured_state: configured.into(),
                    effective_state: NOT_APPLICABLE.into(),
                    effective_detail: Some(
                        "当前目标 CLI 不支持项目级 Permissions 生效态扫描".into(),
                    ),
                    live_path: None,
                };
            }
        };
        let lists = crate::services::project_config_sync::expected_project_permission_lists(
            &state.db, project_id, app,
        )
        .unwrap_or_else(|_| crate::services::permission_sync::PermissionLists {
            allow: vec![],
            deny: vec![],
            auto_approve: vec![],
        });
        return effective_from_project_resync(
            "permissions",
            configured,
            support,
            &config_path,
            |tmp_path| {
                crate::services::permission_sync::sync_permissions_at_path(&lists, app, tmp_path)
            },
        );
    }

    let expected = expected_permissions_json(&state.db).unwrap_or(json!({}));
    effective_from_json_field("permissions", configured, &expected, "permissions", support)
}

fn scan_hooks(
    state: &AppState,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
    ctx: EffectiveScanContext<'_>,
) -> EffectiveItemState {
    let use_project = ctx.project_id.is_some_and(|id| {
        crate::services::project_config_sync::project_has_asset_links(&state.db, id, "hook", app)
    }) && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let config_path = match app {
            AppType::Claude => crate::prompt_files::project_claude_settings_path(project_root),
            AppType::Codex => crate::prompt_files::project_codex_config_path(project_root),
            AppType::Gemini => crate::prompt_files::project_gemini_settings_path(project_root),
            AppType::Hermes => crate::prompt_files::project_hermes_config_path(project_root),
            _ => {
                return EffectiveItemState {
                    check_name: "hooks_configured".into(),
                    configured_state: configured.into(),
                    effective_state: NOT_APPLICABLE.into(),
                    effective_detail: Some("当前目标 CLI 不支持项目级 Hooks 生效态扫描".into()),
                    live_path: None,
                };
            }
        };
        let hooks = crate::services::project_config_sync::expected_project_hooks(
            &state.db, project_id, app,
        )
        .unwrap_or_default();
        return effective_from_project_resync(
            "hooks_configured",
            configured,
            support,
            &config_path,
            |tmp_path| crate::services::hook_sync::sync_hooks_at_path(&hooks, app, tmp_path),
        );
    }

    let expected = expected_hooks_json(&state.db).unwrap_or(json!({}));
    effective_from_json_field("hooks_configured", configured, &expected, "hooks", support)
}

fn effective_from_project_resync<F>(
    check_name: &str,
    configured: &str,
    support: AssetSupport,
    live_path: &std::path::Path,
    sync_fn: F,
) -> EffectiveItemState
where
    F: FnOnce(&std::path::Path) -> Result<(), AppError>,
{
    if support == AssetSupport::Unsupported {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: NOT_APPLICABLE.into(),
            effective_detail: Some("当前目标 CLI 不支持此项".into()),
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

    let actual = if live_path.is_file() {
        std::fs::read_to_string(live_path).unwrap_or_default()
    } else {
        String::new()
    };

    let tmp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return EffectiveItemState {
                check_name: check_name.into(),
                configured_state: configured.into(),
                effective_state: UNCHECKED.into(),
                effective_detail: Some(e.to_string()),
                live_path: Some(live_path.display().to_string()),
            };
        }
    };
    let tmp_path = tmp_dir.path().join(
        live_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "config.tmp".to_string()),
    );
    if !actual.is_empty() {
        let _ = std::fs::write(&tmp_path, &actual);
    }
    if let Err(e) = sync_fn(&tmp_path) {
        return EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: UNCHECKED.into(),
            effective_detail: Some(e.to_string()),
            live_path: Some(live_path.display().to_string()),
        };
    }
    let expected = if tmp_path.is_file() {
        std::fs::read_to_string(&tmp_path).unwrap_or_default()
    } else {
        String::new()
    };

    if compare_text(&expected, &actual) {
        EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: EFFECTIVE.into(),
            effective_detail: None,
            live_path: Some(live_path.display().to_string()),
        }
    } else {
        EffectiveItemState {
            check_name: check_name.into(),
            configured_state: configured.into(),
            effective_state: DRIFTED.into(),
            effective_detail: Some("项目配置与 OpenSunstar 库不一致".into()),
            live_path: Some(live_path.display().to_string()),
        }
    }
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
            return Err(AppError::Config(format!(
                "{app:?} 不支持 slash 命令文件路径"
            )));
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
        return Err(AppError::Config(format!(
            "{app:?} 不支持 Subagent 文件路径"
        )));
    }
    Ok(match app {
        AppType::Claude => crate::config::get_claude_config_dir()
            .join("agents")
            .join(format!("{name}.md")),
        AppType::Gemini => get_gemini_dir().join("agents").join(format!("{name}.md")),
        AppType::OpenCode => get_opencode_dir().join("agents").join(format!("{name}.md")),
        AppType::Codex => get_codex_config_dir()
            .join("agents")
            .join(format!("{name}.toml")),
        _ => {
            return Err(AppError::Config(format!(
                "{app:?} 不支持 Subagent 文件路径"
            )))
        }
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
    state: &AppState,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
    ctx: EffectiveScanContext<'_>,
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

    let use_project = ctx
        .project_id
        .is_some_and(|id| crate::services::project_config_sync::project_has_skills(&state.db, id))
        && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let live_root = match crate::prompt_files::project_skills_dir(project_root, app) {
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
        let expected_dirs =
            crate::services::project_config_sync::expected_project_skill_directories(
                &state.db, project_id, app,
            )
            .unwrap_or_default();

        let mut drifted = Vec::new();
        for directory in &expected_dirs {
            let source = ssot_dir.join(directory);
            let dest = live_root.join(directory);
            if !dest.exists() {
                drifted.push(directory.clone());
                continue;
            }
            let source_hash = SkillService::compute_dir_hash(&source).ok();
            let dest_hash = SkillService::compute_dir_hash(&dest).ok();
            if source_hash != dest_hash {
                drifted.push(directory.clone());
            }
        }

        return aggregate_effective_state(
            check_name,
            configured,
            Some(live_root.display().to_string()),
            drifted,
            "无项目启用的 Skills",
        );
    }

    let db = &state.db;
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
    state: &AppState,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
    ctx: EffectiveScanContext<'_>,
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

    let use_project = ctx.project_id.is_some_and(|id| {
        state
            .db
            .get_project_asset_links(id, Some("command"))
            .map(|links| {
                links
                    .iter()
                    .any(|l| l.enabled && l.asset_app_type == app.as_str())
            })
            .unwrap_or(false)
    }) && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let live_root = match crate::prompt_files::project_commands_dir(project_root, app) {
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

        let expected = crate::services::project_config_sync::expected_project_commands(
            &state.db, project_id, app,
        )
        .unwrap_or_default();

        let mut drifted = Vec::new();
        for (name, content) in &expected {
            let path = match crate::prompt_files::project_command_file_path(project_root, app, name)
            {
                Ok(p) => p,
                Err(_) => {
                    drifted.push(name.clone());
                    continue;
                }
            };
            let actual = if path.is_file() {
                crate::services::marker_merge::strip_managed_command_marker(
                    &std::fs::read_to_string(&path).unwrap_or_default(),
                )
            } else {
                String::new()
            };
            if !compare_text(content, &actual) {
                drifted.push(name.clone());
            }
        }

        return aggregate_effective_state(
            check_name,
            configured,
            Some(live_root.display().to_string()),
            drifted,
            "无项目启用的 Commands",
        );
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

    let enabled: Vec<_> = state
        .db
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
    state: &AppState,
    app: &AppType,
    configured: &str,
    support: AssetSupport,
    ctx: EffectiveScanContext<'_>,
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

    let use_project = ctx.project_id.is_some_and(|id| {
        crate::services::project_config_sync::project_has_asset_links(
            &state.db, id, "subagent", app,
        )
    }) && ctx.project_path.is_some();

    if use_project {
        let project_id = ctx.project_id.unwrap();
        let project_root = std::path::Path::new(ctx.project_path.unwrap());
        let live_root = match crate::prompt_files::project_agents_dir(project_root, app) {
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
        let expected = crate::services::project_config_sync::expected_project_subagents(
            &state.db, project_id, app,
        )
        .unwrap_or_default();

        let mut drifted = Vec::new();
        for (name, content) in &expected {
            let path = match crate::prompt_files::project_agent_file_path(project_root, app, name) {
                Ok(p) => p,
                Err(_) => {
                    drifted.push(name.clone());
                    continue;
                }
            };
            let actual = if path.is_file() {
                strip_managed_subagent_marker(&std::fs::read_to_string(&path).unwrap_or_default())
            } else {
                String::new()
            };
            if !compare_text(content, &actual) {
                drifted.push(name.clone());
            }
        }

        return aggregate_effective_state(
            check_name,
            configured,
            Some(live_root.display().to_string()),
            drifted,
            "无项目启用的 Subagents",
        );
    }

    let db = &state.db;
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

/// Optional project context for project-level file comparisons (L2).
#[derive(Debug, Clone, Copy, Default)]
pub struct EffectiveScanContext<'a> {
    pub project_path: Option<&'a str>,
    pub project_id: Option<&'a str>,
}

fn effective_from_managed_text(
    check_name: &str,
    configured: &str,
    expected_body: &str,
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

    let actual_full = if live_path.is_file() {
        std::fs::read_to_string(live_path).unwrap_or_default()
    } else {
        String::new()
    };
    let actual_body = extract_markdown_section(&actual_full, PROMPT_SECTION_ID)
        .filter(|s| !s.is_empty())
        .unwrap_or(actual_full);

    let path_str = live_path.display().to_string();
    if compare_text(expected_body, &actual_body) {
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
            effective_detail: Some("项目级 Prompt 文件的 OpenSunstar 管理段与库内容不一致".into()),
            live_path: Some(path_str),
        }
    }
}

fn expected_project_mcp_map(
    db: &Database,
    project_id: &str,
) -> Result<HashMap<String, Value>, AppError> {
    let links = db.get_project_mcp_servers(project_id)?;
    let all = db.get_all_mcp_servers()?;
    let mut map = HashMap::new();
    for link in links.into_iter().filter(|l| l.enabled) {
        if let Some(server) = all.get(&link.config_id) {
            if server.apps.claude {
                map.insert(link.config_id.clone(), server.server.clone());
            }
        }
    }
    Ok(map)
}

fn expected_project_prompt_body(
    state: &AppState,
    project_id: &str,
    app: &AppType,
) -> Result<String, AppError> {
    let links = state
        .db
        .get_project_prompts(project_id)?
        .into_iter()
        .filter(|l| l.enabled && l.prompt_app_type == app.as_str())
        .collect::<Vec<_>>();
    let prompts = state.db.get_prompts(app.as_str())?;
    let mut parts = Vec::new();
    for link in links {
        if let Some(prompt) = prompts.get(&link.prompt_id) {
            if prompt.is_fragment {
                continue;
            }
            let content = PromptService::resolve_effective_content(state, app, prompt)?;
            if !content.trim().is_empty() {
                parts.push(content);
            }
        }
    }
    Ok(parts.join("\n\n"))
}

/// 扫描全部 readiness 检查项的生效态，并与 readiness 明细合并
pub fn scan_effective_states(
    state: &AppState,
    readiness_details: &[AgentReadinessItem],
    target_app: Option<&str>,
    ctx: EffectiveScanContext<'_>,
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
            "mcp_enabled" => scan_mcp(&state.db, &app, configured, support, ctx),
            "prompt_files" => scan_prompt(state, &app, configured, support, ctx),
            "ignore_rules" => scan_ignore(&state.db, &app, configured, support, ctx),
            "permissions" => scan_permissions(state, &app, configured, support, ctx),
            "hooks_configured" => scan_hooks(state, &app, configured, support, ctx),
            "recent_updates" => EffectiveItemState {
                check_name: detail.check_name.clone(),
                configured_state: configured.to_string(),
                effective_state: NOT_APPLICABLE.to_string(),
                effective_detail: Some("维护度指标无磁盘生效态".into()),
                live_path: None,
            },
            "skills_configured" => scan_skills(state, &app, configured, support, ctx),
            "commands_configured" => scan_commands(state, &app, configured, support, ctx),
            "subagents_configured" => scan_subagents(state, &app, configured, support, ctx),
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
