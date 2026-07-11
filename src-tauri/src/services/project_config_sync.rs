//! Project-level config write-back (L2): MCP, prompts, commands, hooks, permissions, skills, subagents.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::agent_sync_supported;
use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::error::AppError;
use crate::hook::Hook;
use crate::prompt_files::{
    project_agent_file_path, project_agents_dir, project_claude_settings_path,
    project_codex_config_path, project_command_file_path, project_command_manifest_path,
    project_commands_dir, project_gemini_settings_path, project_hermes_config_path,
    project_mcp_json_path, project_opencode_config_path, project_prompt_file_path,
    project_skill_manifest_path, project_skills_dir, project_subagent_manifest_path,
};
use crate::services::agent_codex::markdown_agent_to_codex_toml;
use crate::services::marker_merge::{
    inject_markdown_section, is_managed_command_file, is_managed_subagent_file,
    wrap_managed_command, wrap_managed_subagent, wrap_managed_subagent_codex, AGENTS_BRIDGE_LINE,
    AGENTS_BRIDGE_SECTION_ID, PROMPT_SECTION_ID,
};
use crate::services::permission_sync::collect_permission_lists;
use crate::services::prompt::PromptService;
use crate::services::skill::SkillService;
use crate::store::AppState;
use crate::tool_permission::ToolPermission;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CommandManifest {
    #[serde(default)]
    entries: Vec<CommandManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct CommandManifestEntry {
    app: String,
    command_id: String,
    file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SubagentManifest {
    #[serde(default)]
    entries: Vec<SubagentManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SubagentManifestEntry {
    app: String,
    agent_id: String,
    file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SkillManifest {
    #[serde(default)]
    entries: Vec<SkillManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SkillManifestEntry {
    app: String,
    skill_id: String,
    directory: String,
}

/// Sync all project-level artifacts after junction table changes.
pub fn sync_all_for_project_id(state: &AppState, project_id: &str) -> Result<(), AppError> {
    let project = state
        .db
        .get_project(project_id)?
        .ok_or_else(|| AppError::Message(format!("项目不存在: {project_id}")))?;
    sync_all_for_project_path(state, &project.path)
}

pub fn sync_all_for_project_path(state: &AppState, project_path: &str) -> Result<(), AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }
    sync_project_mcp_json(state, &root)?;
    sync_project_prompts_for_all_apps(state, &root)?;
    sync_project_commands_for_all_apps(state, &root)?;
    sync_project_hooks_for_all_apps(state, &root)?;
    sync_project_permissions_for_all_apps(state, &root)?;
    sync_project_skills_for_all_apps(state, &root)?;
    sync_project_subagents_for_all_apps(state, &root)?;
    sync_project_ignore_for_all_apps(state, &root)?;
    Ok(())
}

/// 按就绪度检查项写回单类项目级资产（漂移一键修复 P0-B）
pub fn sync_asset_for_project_path(
    state: &AppState,
    project_path: &str,
    check_name: &str,
) -> Result<(), AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }
    match check_name {
        "mcp_enabled" => sync_project_mcp_json(state, &root),
        "prompt_files" => sync_project_prompts_for_all_apps(state, &root),
        "commands_configured" => sync_project_commands_for_all_apps(state, &root),
        "hooks_configured" => sync_project_hooks_for_all_apps(state, &root),
        "permissions" => sync_project_permissions_for_all_apps(state, &root),
        "skills_configured" => sync_project_skills_for_all_apps(state, &root),
        "subagents_configured" => sync_project_subagents_for_all_apps(state, &root),
        "ignore_rules" => sync_project_ignore_for_all_apps(state, &root),
        "recent_updates" => Err(AppError::Message(
            "维护度指标不支持磁盘写回修复".into(),
        )),
        other => Err(AppError::Message(format!("未知检查项: {other}"))),
    }
}

fn sync_project_mcp_json(state: &AppState, project_root: &Path) -> Result<(), AppError> {
    use crate::services::marker_merge::{create_companion_marker, has_companion_marker};

    let db = &state.db;
    let project_id = db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    let links = db.get_project_mcp_servers(&project_id)?;
    let all_servers = db.get_all_mcp_servers()?;
    let mut enabled: HashMap<String, Value> = HashMap::new();

    for link in links.into_iter().filter(|l| l.enabled) {
        if let Some(server) = all_servers.get(&link.config_id) {
            if server.apps.claude {
                enabled.insert(link.config_id.clone(), server.server.clone());
            }
        }
    }

    let mcp_path = project_mcp_json_path(project_root);

    // 标记保护：文件存在但没有 OpenSunstar 伴生标记 → 用户自建，跳过覆盖
    if mcp_path.is_file() && !has_companion_marker(&mcp_path) {
        log::warn!(
            "跳过覆盖非 OpenSunstar 管理的 MCP 文件: {}",
            mcp_path.display()
        );
        return Ok(());
    }

    if enabled.is_empty() {
        if mcp_path.is_file() {
            crate::claude_mcp::write_project_mcp_servers_map(project_root, &HashMap::new())?;
        }
        return Ok(());
    }

    crate::claude_mcp::write_project_mcp_servers_map(project_root, &enabled)?;

    // 写入成功后创建伴生标记文件
    create_companion_marker(&mcp_path);

    Ok(())
}

fn sync_project_prompts_for_all_apps(state: &AppState, project_root: &Path) -> Result<(), AppError> {
    let db = &state.db;
    let project_id = db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    let links = db.get_project_prompts(&project_id)?;
    let apps: std::collections::HashSet<String> = links
        .iter()
        .filter(|l| l.enabled)
        .map(|l| l.prompt_app_type.clone())
        .collect();

    let has_claude = apps.contains("claude");

    for app_str in apps.clone() {
        if let Ok(app) = app_str.parse::<AppType>() {
            sync_project_prompts_for_app(state, project_root, &project_id, &app)?;
        }
    }

    if has_claude {
        ensure_agents_bridge_on_claude(state, project_root, &project_id)?;
    }

    Ok(())
}

fn sync_project_prompts_for_app(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
    app: &AppType,
) -> Result<(), AppError> {
    let db = &state.db;
    let links = db
        .get_project_prompts(project_id)?
        .into_iter()
        .filter(|l| l.enabled && l.prompt_app_type == app.as_str())
        .collect::<Vec<_>>();

    let path = project_prompt_file_path(project_root, app)?;
    if links.is_empty() {
        if path.is_file() {
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            let cleared = inject_markdown_section(&existing, PROMPT_SECTION_ID, "");
            if cleared.trim().is_empty() {
                let _ = std::fs::remove_file(&path);
            } else {
                write_text_file(&path, &cleared)?;
            }
        }
        return Ok(());
    }

    let prompts = db.get_prompts(app.as_str())?;
    let mut parts: Vec<String> = Vec::new();
    for link in &links {
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

    let body = parts.join("\n\n");
    let existing = if path.is_file() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    let merged = inject_markdown_section(&existing, PROMPT_SECTION_ID, &body);
    write_text_file(&path, &merged)?;
    Ok(())
}

/// L2-03: inject `@AGENTS.md` via managed bridge section when cross-tool AGENTS family is in use.
fn ensure_agents_bridge_on_claude(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
) -> Result<(), AppError> {
    let agents_path = project_root.join("AGENTS.md");
    let claude_path = project_prompt_file_path(project_root, &AppType::Claude)?;

    let needs_bridge = agents_path.is_file()
        || state
            .db
            .get_project_prompts(project_id)?
            .iter()
            .any(|l| {
                l.enabled
                    && matches!(
                        l.prompt_app_type.as_str(),
                        "codex" | "opencode" | "openclaw" | "hermes"
                    )
            });

    if !needs_bridge || !claude_path.is_file() {
        return Ok(());
    }

    let existing = std::fs::read_to_string(&claude_path).unwrap_or_default();
    if existing.contains(AGENTS_BRIDGE_LINE) {
        return Ok(());
    }

    let merged = inject_markdown_section(&existing, AGENTS_BRIDGE_SECTION_ID, AGENTS_BRIDGE_LINE);
    write_text_file(&claude_path, &merged)?;
    Ok(())
}

fn read_command_manifest(project_root: &Path) -> CommandManifest {
    let path = project_command_manifest_path(project_root);
    if !path.is_file() {
        return CommandManifest::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_command_manifest(project_root: &Path, manifest: &CommandManifest) -> Result<(), AppError> {
    let path = project_command_manifest_path(project_root);
    if manifest.entries.is_empty() {
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| AppError::io(&path, e))?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    write_text_file(&path, &text)
}

fn sync_project_commands_for_all_apps(state: &AppState, project_root: &Path) -> Result<(), AppError> {
    let db = &state.db;
    let project_id = db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    let links = db
        .get_project_asset_links(&project_id, Some("command"))
        .unwrap_or_default();
    let manifest = read_command_manifest(project_root);

    let mut apps: HashSet<String> = links
        .iter()
        .filter(|l| l.enabled)
        .map(|l| l.asset_app_type.clone())
        .collect();
    for entry in &manifest.entries {
        apps.insert(entry.app.clone());
    }

    for app_str in apps {
        if let Ok(app) = app_str.parse::<AppType>() {
            sync_project_commands_for_app(state, project_root, &project_id, &app)?;
        }
    }
    Ok(())
}

fn sync_project_commands_for_app(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
    app: &AppType,
) -> Result<(), AppError> {
    let db = &state.db;
    let app_str = app.as_str();
    let links: Vec<_> = db
        .get_project_asset_links(project_id, Some("command"))?
        .into_iter()
        .filter(|l| l.enabled && l.asset_app_type == app_str)
        .collect();

    let commands = db.get_all_commands()?;
    let commands_dir = project_commands_dir(project_root, app)?;
    if !links.is_empty() {
        std::fs::create_dir_all(&commands_dir).map_err(|e| AppError::io(&commands_dir, e))?;
    }

    let mut manifest = read_command_manifest(project_root);
    let previous: Vec<CommandManifestEntry> = manifest
        .entries
        .iter()
        .filter(|e| e.app == app_str)
        .cloned()
        .collect();

    let mut next_entries: Vec<CommandManifestEntry> = Vec::new();

    for link in &links {
        let Some(command) = commands.get(&link.asset_id) else {
            log::warn!(
                "项目 Command 关联缺失全局库记录: project={project_id} id={}",
                link.asset_id
            );
            continue;
        };
        let path = project_command_file_path(project_root, app, &command.name)?;
        let file_name = format!("{}.md", command.name);

        if path.is_file() {
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            if !existing.is_empty() && !is_managed_command_file(&existing) {
                log::warn!(
                    "跳过覆盖非 OpenSunstar 管理的项目 Command: {}",
                    path.display()
                );
                continue;
            }
        }

        let payload = wrap_managed_command(&command.id, &command.content);
        write_text_file(&path, &payload)?;
        next_entries.push(CommandManifestEntry {
            app: app_str.to_string(),
            command_id: command.id.clone(),
            file_name,
        });
    }

    let next_names: HashSet<_> = next_entries.iter().map(|e| e.file_name.as_str()).collect();
    for old in previous {
        if next_names.contains(old.file_name.as_str()) {
            continue;
        }
        let path = commands_dir.join(&old.file_name);
        if path.is_file() {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if is_managed_command_file(&text) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    manifest
        .entries
        .retain(|e| e.app != app_str);
    manifest.entries.extend(next_entries);
    write_command_manifest(project_root, &manifest)?;
    Ok(())
}

fn project_hooks_for_app(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Vec<Hook>, AppError> {
    let app_str = app.as_str();
    let links = db
        .get_project_asset_links(project_id, Some("hook"))?
        .into_iter()
        .filter(|l| l.enabled && l.asset_app_type == app_str)
        .collect::<Vec<_>>();
    let all = db.get_all_hooks()?;
    let by_id: HashMap<_, _> = all.iter().map(|h| (h.id.clone(), h.clone())).collect();
    Ok(links
        .iter()
        .filter_map(|l| by_id.get(&l.asset_id).cloned())
        .collect())
}

fn project_permissions_for_app(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Vec<ToolPermission>, AppError> {
    let app_str = app.as_str();
    let links = db
        .get_project_asset_links(project_id, Some("permission"))?
        .into_iter()
        .filter(|l| l.enabled && l.asset_app_type == app_str)
        .collect::<Vec<_>>();
    let all = db.get_all_tool_permissions()?;
    let by_id: HashMap<_, _> = all.iter().map(|p| (p.id.clone(), p.clone())).collect();
    Ok(links
        .iter()
        .filter_map(|l| by_id.get(&l.asset_id).cloned())
        .collect())
}

fn sync_project_hooks_for_all_apps(state: &AppState, project_root: &Path) -> Result<(), AppError> {
    let project_id = state
        .db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    for app in [
        AppType::Claude,
        AppType::Codex,
        AppType::Gemini,
        AppType::Hermes,
    ] {
        let has_links = state
            .db
            .get_project_asset_links(&project_id, Some("hook"))?
            .iter()
            .any(|l| l.enabled && l.asset_app_type == app.as_str());
        let config_path = project_hook_config_path(project_root, &app);
        if has_links || config_path.is_file() {
            sync_project_hooks_for_app(state, project_root, &project_id, &app)?;
        }
    }
    Ok(())
}

fn sync_project_hooks_for_app(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
    app: &AppType,
) -> Result<(), AppError> {
    use crate::services::marker_merge::{create_companion_marker, has_companion_marker};

    let hooks = project_hooks_for_app(&state.db, project_id, app)?;
    let config_path = project_hook_config_path(project_root, app);

    // 标记保护：配置文件存在但没有伴生标记 → 用户自建，跳过覆盖
    if config_path.is_file() && !has_companion_marker(&config_path) {
        log::warn!(
            "跳过覆盖非 OpenSunstar 管理的项目 Hooks 配置: {}",
            config_path.display()
        );
        return Ok(());
    }

    crate::services::hook_sync::sync_hooks_at_path(&hooks, app, &config_path)?;

    // 写入成功后创建伴生标记
    if config_path.is_file() {
        create_companion_marker(&config_path);
    }

    Ok(())
}

fn sync_project_permissions_for_all_apps(
    state: &AppState,
    project_root: &Path,
) -> Result<(), AppError> {
    let project_id = state
        .db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    for app in [
        AppType::Claude,
        AppType::Codex,
        AppType::Gemini,
        AppType::OpenCode,
        AppType::Hermes,
    ] {
        let has_links = state
            .db
            .get_project_asset_links(&project_id, Some("permission"))?
            .iter()
            .any(|l| l.enabled && l.asset_app_type == app.as_str());
        let config_path = project_permission_config_path(project_root, &app);
        if has_links || config_path.is_file() {
            sync_project_permissions_for_app(state, project_root, &project_id, &app)?;
        }
    }
    Ok(())
}

fn sync_project_permissions_for_app(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
    app: &AppType,
) -> Result<(), AppError> {
    use crate::services::marker_merge::{create_companion_marker, has_companion_marker};

    let perms = project_permissions_for_app(&state.db, project_id, app)?;
    let lists = collect_permission_lists(&perms);
    let config_path = project_permission_config_path(project_root, app);

    // 标记保护：配置文件存在但没有伴生标记 → 用户自建，跳过覆盖
    if config_path.is_file() && !has_companion_marker(&config_path) {
        log::warn!(
            "跳过覆盖非 OpenSunstar 管理的项目 Permissions 配置: {}",
            config_path.display()
        );
        return Ok(());
    }

    crate::services::permission_sync::sync_permissions_at_path(&lists, app, &config_path)?;

    // 写入成功后创建伴生标记
    if config_path.is_file() {
        create_companion_marker(&config_path);
    }

    Ok(())
}

fn project_hook_config_path(project_root: &Path, app: &AppType) -> PathBuf {
    match app {
        AppType::Claude => project_claude_settings_path(project_root),
        AppType::Codex => project_codex_config_path(project_root),
        AppType::Gemini => project_gemini_settings_path(project_root),
        AppType::Hermes => project_hermes_config_path(project_root),
        _ => PathBuf::new(),
    }
}

fn project_permission_config_path(project_root: &Path, app: &AppType) -> PathBuf {
    match app {
        AppType::Claude => project_claude_settings_path(project_root),
        AppType::Codex => project_codex_config_path(project_root),
        AppType::Gemini => project_gemini_settings_path(project_root),
        AppType::OpenCode => project_opencode_config_path(project_root),
        AppType::Hermes => project_hermes_config_path(project_root),
        _ => PathBuf::new(),
    }
}

fn read_subagent_manifest(project_root: &Path) -> SubagentManifest {
    let path = project_subagent_manifest_path(project_root);
    if !path.is_file() {
        return SubagentManifest::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_subagent_manifest(project_root: &Path, manifest: &SubagentManifest) -> Result<(), AppError> {
    let path = project_subagent_manifest_path(project_root);
    if manifest.entries.is_empty() {
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| AppError::io(&path, e))?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    write_text_file(&path, &text)
}

fn sync_project_subagents_for_all_apps(
    state: &AppState,
    project_root: &Path,
) -> Result<(), AppError> {
    let project_id = state
        .db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    let links = state
        .db
        .get_project_asset_links(&project_id, Some("subagent"))
        .unwrap_or_default();
    let manifest = read_subagent_manifest(project_root);

    let mut apps: HashSet<String> = links
        .iter()
        .filter(|l| l.enabled)
        .map(|l| l.asset_app_type.clone())
        .collect();
    for entry in &manifest.entries {
        apps.insert(entry.app.clone());
    }

    for app_str in apps {
        if let Ok(app) = app_str.parse::<AppType>() {
            sync_project_subagents_for_app(state, project_root, &project_id, &app)?;
        }
    }
    Ok(())
}

fn sync_project_subagents_for_app(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
    app: &AppType,
) -> Result<(), AppError> {
    if !agent_sync_supported(app) {
        return Ok(());
    }

    let app_str = app.as_str();
    let links: Vec<_> = state
        .db
        .get_project_asset_links(project_id, Some("subagent"))?
        .into_iter()
        .filter(|l| l.enabled && l.asset_app_type == app_str)
        .collect();

    let agents = state.db.get_all_agents()?;
    let agents_dir = project_agents_dir(project_root, app)?;
    if !links.is_empty() {
        std::fs::create_dir_all(&agents_dir).map_err(|e| AppError::io(&agents_dir, e))?;
    }

    let mut manifest = read_subagent_manifest(project_root);
    let previous: Vec<SubagentManifestEntry> = manifest
        .entries
        .iter()
        .filter(|e| e.app == app_str)
        .cloned()
        .collect();

    let mut next_entries = Vec::new();
    for link in &links {
        let Some(agent) = agents.get(&link.asset_id) else {
            continue;
        };
        let ext = if matches!(app, AppType::Codex) {
            "toml"
        } else {
            "md"
        };
        let file_name = format!("{}.{}", agent.name, ext);
        let path = project_agent_file_path(project_root, app, &agent.name)?;

        if path.is_file() {
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            if !existing.is_empty() && !is_managed_subagent_file(&existing) {
                log::warn!(
                    "跳过覆盖非 OpenSunstar 管理的项目 Subagent: {}",
                    path.display()
                );
                continue;
            }
        }

        let body = if matches!(app, AppType::Codex) {
            markdown_agent_to_codex_toml(
                &agent.name,
                agent.description.as_deref(),
                &agent.content,
            )?
        } else {
            agent.content.clone()
        };
        let payload = if matches!(app, AppType::Codex) {
            wrap_managed_subagent_codex(&agent.id, &body)
        } else {
            wrap_managed_subagent(&agent.id, &body)
        };
        write_text_file(&path, &payload)?;
        next_entries.push(SubagentManifestEntry {
            app: app_str.to_string(),
            agent_id: agent.id.clone(),
            file_name,
        });
    }

    let next_names: HashSet<_> = next_entries.iter().map(|e| e.file_name.as_str()).collect();
    for old in previous {
        if next_names.contains(old.file_name.as_str()) {
            continue;
        }
        let path = agents_dir.join(&old.file_name);
        if path.is_file() {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if is_managed_subagent_file(&text) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    manifest.entries.retain(|e| e.app != app_str);
    manifest.entries.extend(next_entries);
    write_subagent_manifest(project_root, &manifest)?;
    Ok(())
}

fn read_skill_manifest(project_root: &Path) -> SkillManifest {
    let path = project_skill_manifest_path(project_root);
    if !path.is_file() {
        return SkillManifest::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_skill_manifest(project_root: &Path, manifest: &SkillManifest) -> Result<(), AppError> {
    let path = project_skill_manifest_path(project_root);
    if manifest.entries.is_empty() {
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| AppError::io(&path, e))?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    write_text_file(&path, &text)
}

fn sync_project_skills_for_all_apps(state: &AppState, project_root: &Path) -> Result<(), AppError> {
    let project_id = state
        .db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;

    let skill_links = state.db.get_project_skills(&project_id)?;
    if skill_links.is_empty() && read_skill_manifest(project_root).entries.is_empty() {
        return Ok(());
    }

    let installed = state.db.get_all_installed_skills()?;
    let mut apps: HashSet<String> = HashSet::new();
    for link in &skill_links {
        if !link.enabled {
            continue;
        }
        if let Some(skill) = installed.get(&link.config_id) {
            for app in [
                AppType::Claude,
                AppType::Codex,
                AppType::Gemini,
                AppType::OpenCode,
                AppType::Hermes,
            ] {
                if skill.apps.is_enabled_for(&app) {
                    apps.insert(app.as_str().to_string());
                }
            }
        }
    }
    for entry in read_skill_manifest(project_root).entries {
        apps.insert(entry.app);
    }

    for app_str in apps {
        if let Ok(app) = app_str.parse::<AppType>() {
            sync_project_skills_for_app(state, project_root, &project_id, &app)?;
        }
    }
    Ok(())
}

fn sync_project_skills_for_app(
    state: &AppState,
    project_root: &Path,
    project_id: &str,
    app: &AppType,
) -> Result<(), AppError> {
    let skill_links = state
        .db
        .get_project_skills(project_id)?
        .into_iter()
        .filter(|l| l.enabled)
        .collect::<Vec<_>>();
    let installed = state.db.get_all_installed_skills()?;
    let skills_root = project_skills_dir(project_root, app)?;

    let mut manifest = read_skill_manifest(project_root);
    let app_str = app.as_str();
    let previous: Vec<SkillManifestEntry> = manifest
        .entries
        .iter()
        .filter(|e| e.app == app_str)
        .cloned()
        .collect();

    let mut next_entries = Vec::new();
    for link in &skill_links {
        let Some(skill) = installed.get(&link.config_id) else {
            continue;
        };
        if !skill.apps.is_enabled_for(app) {
            continue;
        }
        SkillService::sync_to_skills_root(&skill.directory, &skills_root)
            .map_err(|e| AppError::Message(e.to_string()))?;
        next_entries.push(SkillManifestEntry {
            app: app_str.to_string(),
            skill_id: skill.id.clone(),
            directory: skill.directory.clone(),
        });
    }

    let next_dirs: HashSet<_> = next_entries.iter().map(|e| e.directory.as_str()).collect();
    for old in previous {
        if next_dirs.contains(old.directory.as_str()) {
            continue;
        }
        let dest = skills_root.join(&old.directory);
        if dest.exists() {
            SkillService::remove_from_skills_root(&old.directory, &skills_root)
                .map_err(|e| AppError::Message(e.to_string()))?;
        }
    }

    manifest.entries.retain(|e| e.app != app_str);
    manifest.entries.extend(next_entries);
    write_skill_manifest(project_root, &manifest)?;
    Ok(())
}

fn sync_project_ignore_for_all_apps(
    state: &AppState,
    project_root: &Path,
) -> Result<(), AppError> {
    let project_id = state
        .db
        .get_project_id_by_path(project_root.to_string_lossy().as_ref())?
        .ok_or_else(|| AppError::Message("项目未注册到 OpenSunstar".into()))?;
    crate::services::ignore::IgnoreService::sync_project_ignore(state, project_root, &project_id)
}

/// 供生效态扫描：项目启用的 Command 名称与期望正文（不含 marker 行）
pub fn expected_project_commands(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Vec<(String, String)>, AppError> {
    let app_str = app.as_str();
    let links = db
        .get_project_asset_links(project_id, Some("command"))?
        .into_iter()
        .filter(|l| l.enabled && l.asset_app_type == app_str)
        .collect::<Vec<_>>();
    let commands = db.get_all_commands()?;
    let mut out = Vec::new();
    for link in links {
        if let Some(command) = commands.get(&link.asset_id) {
            out.push((command.name.clone(), command.content.clone()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// 项目是否有某类资产关联（指定 CLI）
pub fn project_has_asset_links(
    db: &crate::database::Database,
    project_id: &str,
    asset_type: &str,
    app: &AppType,
) -> bool {
    db.get_project_asset_links(project_id, Some(asset_type))
        .map(|links| {
            links
                .iter()
                .any(|l| l.enabled && l.asset_app_type == app.as_str())
        })
        .unwrap_or(false)
}

/// 项目是否有关联 Skill
pub fn project_has_skills(db: &crate::database::Database, project_id: &str) -> bool {
    db.get_project_skills(project_id)
        .map(|links| links.iter().any(|l| l.enabled))
        .unwrap_or(false)
}

/// 供生效态：项目级 Claude hooks JSON
pub fn expected_project_hooks_json(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Value, AppError> {
    use serde_json::{json, Map};
    let hooks = project_hooks_for_app(db, project_id, app)?;
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

pub fn expected_project_hooks(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Vec<Hook>, AppError> {
    project_hooks_for_app(db, project_id, app)
}

/// 供生效态：项目级 Claude permissions JSON
pub fn expected_project_permissions_json(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Value, AppError> {
    use serde_json::json;
    let perms = project_permissions_for_app(db, project_id, app)?;
    let lists = collect_permission_lists(&perms);
    let mut allow = lists.allow.clone();
    allow.extend(lists.auto_approve.clone());
    allow.sort();
    allow.dedup();
    Ok(json!({
        "allow": allow,
        "deny": lists.deny,
        "additionalDirectories": []
    }))
}

pub fn expected_project_permission_lists(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<crate::services::permission_sync::PermissionLists, AppError> {
    let perms = project_permissions_for_app(db, project_id, app)?;
    Ok(collect_permission_lists(&perms))
}

/// 供生效态：项目启用的 Subagent 名称与期望正文（不含 marker）
pub fn expected_project_subagents(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Vec<(String, String)>, AppError> {
    let app_str = app.as_str();
    let links = db
        .get_project_asset_links(project_id, Some("subagent"))?
        .into_iter()
        .filter(|l| l.enabled && l.asset_app_type == app_str)
        .collect::<Vec<_>>();
    let agents = db.get_all_agents()?;
    let mut out = Vec::new();
    for link in links {
        if let Some(agent) = agents.get(&link.asset_id) {
            let body = if matches!(app, AppType::Codex) {
                markdown_agent_to_codex_toml(
                    &agent.name,
                    agent.description.as_deref(),
                    &agent.content,
                )?
            } else {
                agent.content.clone()
            };
            out.push((agent.name.clone(), body));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// 供生效态：项目启用的 Skill 目录名
pub fn expected_project_skill_directories(
    db: &crate::database::Database,
    project_id: &str,
    app: &AppType,
) -> Result<Vec<String>, AppError> {
    let installed = db.get_all_installed_skills()?;
    let mut out = Vec::new();
    for link in db.get_project_skills(project_id)? {
        if !link.enabled {
            continue;
        }
        if let Some(skill) = installed.get(&link.config_id) {
            if skill.apps.is_enabled_for(app) {
                out.push(skill.directory.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::{McpApps, McpServer};
    use crate::database::{Database, Project};
    use crate::hook::Hook;
    use crate::tool_permission::ToolPermission;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn test_state() -> AppState {
        AppState::new(Arc::new(Database::memory().unwrap()))
    }

    fn seed_project(db: &Database, path: &Path) -> String {
        let id = "proj-test".to_string();
        let now = 1_700_000_000_i64;
        db.upsert_project(&Project {
            id: id.clone(),
            name: "test".into(),
            path: path.to_string_lossy().into(),
            git_remote_url: None,
            created_at: now,
            updated_at: now,
            target_app: None,
            blueprint_id: None,
            stage: "mvp".into(),
            mvp_progress: None,
        })
        .unwrap();
        id
    }

    #[test]
    fn writes_project_mcp_json() {
        let dir = tempdir().unwrap();
        let state = test_state();
        let pid = seed_project(&state.db, dir.path());

        state
            .db
            .save_mcp_server(&McpServer {
                id: "ctx7".into(),
                name: "Context7".into(),
                server: json!({"command": "npx", "args": ["-y", "pkg"]}),
                apps: McpApps {
                    claude: true,
                    ..Default::default()
                },
                description: None,
                homepage: None,
                docs: None,
                tags: vec![],
            })
            .unwrap();
        state.db.link_project_mcp_server(&pid, "ctx7", true).unwrap();

        sync_project_mcp_json(&state, dir.path()).unwrap();
        let text = fs::read_to_string(dir.path().join(".mcp.json")).unwrap();
        assert!(text.contains("mcpServers"));
        assert!(text.contains("ctx7"));
    }

    #[test]
    fn writes_project_prompt_with_marker() {
        use crate::prompt::Prompt;

        let dir = tempdir().unwrap();
        let state = test_state();
        let pid = seed_project(&state.db, dir.path());

        let prompt = Prompt {
            id: "p1".into(),
            name: "Rules".into(),
            content: "Use TypeScript strictly.".into(),
            enabled: false,
            ..Default::default()
        };
        state.db.save_prompt("claude", &prompt).unwrap();
        state
            .db
            .link_project_prompt(&pid, "p1", "claude", true)
            .unwrap();

        sync_project_prompts_for_app(&state, dir.path(), &pid, &AppType::Claude).unwrap();
        let text = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(text.contains("opensunstar:managed-prompt"));
        assert!(text.contains("TypeScript"));
    }

    #[test]
    fn sync_asset_rejects_unknown_check() {
        let dir = tempdir().unwrap();
        let state = test_state();
        let _pid = seed_project(&state.db, dir.path());
        let err = sync_asset_for_project_path(
            &state,
            dir.path().to_str().unwrap(),
            "recent_updates",
        )
        .unwrap_err();
        assert!(err.to_string().contains("不支持"));
    }

    #[test]
    fn repair_loop_closes_mcp_drift() {
        use crate::ai::agent_readiness::{compute_readiness_items, ReadinessCheckInput};
        use crate::ai::asset_effective_state::{
            scan_effective_states, EffectiveScanContext, DRIFTED, EFFECTIVE,
        };
        use crate::app_config::AppType;

        let dir = tempdir().unwrap();
        let state = test_state();
        let pid = seed_project(&state.db, dir.path());
        let path = dir.path().to_string_lossy().to_string();

        state
            .db
            .save_mcp_server(&McpServer {
                id: "ctx7".into(),
                name: "Context7".into(),
                server: json!({"command": "npx", "args": ["-y", "pkg"]}),
                apps: McpApps {
                    claude: true,
                    ..Default::default()
                },
                description: None,
                homepage: None,
                docs: None,
                tags: vec![],
            })
            .unwrap();
        state.db.link_project_mcp_server(&pid, "ctx7", true).unwrap();
        sync_project_mcp_json(&state, dir.path()).unwrap();

        fs::write(
            dir.path().join(".mcp.json"),
            r#"{"mcpServers":{"stale":{"command":"echo"}}}"#,
        )
        .unwrap();

        let (_, details) = compute_readiness_items(&ReadinessCheckInput {
            mcp_project_count: 1,
            has_repo_mcp: false,
            skills_count: 0,
            prompt_db_count: 0,
            prompt_files: vec![],
            commands_count: 0,
            hooks_count: 0,
            ignore_project_count: 0,
            ignore_global_count: 0,
            permissions_project_count: 0,
            permissions_global_count: 0,
            subagents_count: 0,
            recent_update_within_90d: false,
            target_app: Some("claude".into()),
        });

        let before = scan_effective_states(
            &state,
            &details,
            Some("claude"),
            EffectiveScanContext {
                project_path: Some(&path),
                project_id: Some(&pid),
            },
        );
        let mcp_before = before
            .items
            .iter()
            .find(|i| i.check_name == "mcp_enabled")
            .expect("mcp item");
        assert_eq!(mcp_before.effective_state, DRIFTED);

        sync_asset_for_project_path(&state, &path, "mcp_enabled").unwrap();

        let after = scan_effective_states(
            &state,
            &details,
            Some(AppType::Claude.as_str()),
            EffectiveScanContext {
                project_path: Some(&path),
                project_id: Some(&pid),
            },
        );
        let mcp_after = after
            .items
            .iter()
            .find(|i| i.check_name == "mcp_enabled")
            .expect("mcp item");
        assert_eq!(mcp_after.effective_state, EFFECTIVE);
    }

    #[test]
    fn writes_project_command_with_marker() {
        use crate::command::Command;

        let dir = tempdir().unwrap();
        let state = test_state();
        let pid = seed_project(&state.db, dir.path());

        let command = Command {
            id: "c1".into(),
            name: "review".into(),
            description: None,
            content: "Review the diff carefully.".into(),
            arguments: "[]".into(),
            enabled_claude: true,
            enabled_codex: false,
            enabled_gemini: false,
            enabled_opencode: false,
            enabled_hermes: false,
            created_at: None,
            updated_at: None,
        };
        state.db.save_command(&command).unwrap();
        state
            .db
            .link_project_asset(&pid, "command", "c1", "claude", true)
            .unwrap();

        sync_project_commands_for_app(&state, dir.path(), &pid, &AppType::Claude).unwrap();
        let path = dir.path().join(".claude").join("commands").join("review.md");
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("opensunstar:managed-command"));
        assert!(text.contains("Review the diff"));
    }

    #[test]
    fn clears_project_hooks_when_unlinked() {
        let dir = tempdir().unwrap();
        let state = test_state();
        let pid = seed_project(&state.db, dir.path());

        let hook = Hook {
            id: "h1".into(),
            event_type: "PreToolUse".into(),
            tool_pattern: "*".into(),
            hook_command: "echo hi".into(),
            timeout_seconds: 30,
            enabled_claude: true,
            enabled_codex: false,
            enabled_gemini: false,
            enabled_opencode: false,
            enabled_hermes: false,
            description: None,
            sort_index: 0,
            created_at: None,
        };
        state.db.save_hook(&hook).unwrap();
        state
            .db
            .link_project_asset(&pid, "hook", "h1", "claude", true)
            .unwrap();

        sync_project_hooks_for_all_apps(&state, dir.path()).unwrap();
        let settings_path = dir.path().join(".claude").join("settings.json");
        let text = fs::read_to_string(&settings_path).unwrap();
        assert!(text.contains("\"hooks\""));

        state
            .db
            .link_project_asset(&pid, "hook", "h1", "claude", false)
            .unwrap();
        sync_project_hooks_for_all_apps(&state, dir.path()).unwrap();
        let text = fs::read_to_string(&settings_path).unwrap();
        assert!(!text.contains("\"hooks\""));
    }

    #[test]
    fn clears_project_permissions_when_unlinked() {
        let dir = tempdir().unwrap();
        let state = test_state();
        let pid = seed_project(&state.db, dir.path());

        let perm = ToolPermission {
            id: "p1".into(),
            permission_type: "allowedTools".into(),
            tool_pattern: "Read".into(),
            enabled_claude: true,
            enabled_codex: false,
            enabled_gemini: false,
            enabled_opencode: false,
            enabled_hermes: false,
            enabled_openclaw: false,
            description: None,
            sort_index: 0,
            created_at: None,
        };
        state.db.save_tool_permission(&perm).unwrap();
        state
            .db
            .link_project_asset(&pid, "permission", "p1", "claude", true)
            .unwrap();

        sync_project_permissions_for_all_apps(&state, dir.path()).unwrap();
        let settings_path = dir.path().join(".claude").join("settings.json");
        let text = fs::read_to_string(&settings_path).unwrap();
        assert!(text.contains("\"permissions\""));

        state
            .db
            .link_project_asset(&pid, "permission", "p1", "claude", false)
            .unwrap();
        sync_project_permissions_for_all_apps(&state, dir.path()).unwrap();
        let text = fs::read_to_string(&settings_path).unwrap();
        assert!(!text.contains("\"permissions\""));
    }
}
