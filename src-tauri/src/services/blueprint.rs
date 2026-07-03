//! Project baseline Blueprint catalog and apply (S2-10 / S2-11).

use serde::{Deserialize, Serialize};

use crate::app_config::{AppType, InstalledSkill, McpServer};
use crate::command::Command;
use crate::database::Database;
use crate::error::AppError;
use crate::prompt::Prompt;
use crate::store::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blueprint {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project_type: String,
    pub target_app: String,
    #[serde(default)]
    pub link_all_mcp_for_target: bool,
    #[serde(default)]
    pub link_all_skills_for_target: bool,
    #[serde(default)]
    pub link_all_prompts_for_target: bool,
    #[serde(default)]
    pub link_all_commands_for_target: bool,
    #[serde(default)]
    pub link_all_hooks_for_target: bool,
    #[serde(default)]
    pub link_all_ignore_for_target: bool,
    #[serde(default)]
    pub link_all_permissions_for_target: bool,
    #[serde(default)]
    pub link_all_subagents_for_target: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintLinkAction {
    pub asset_type: String,
    pub asset_id: String,
    pub app_type: Option<String>,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintApplyPreview {
    pub blueprint_id: String,
    pub blueprint_name: String,
    pub target_app: String,
    pub to_link: Vec<BlueprintLinkAction>,
    pub warnings: Vec<String>,
}

const BUILTIN: &[&str] = &[
    include_str!("../../assets/blueprints/fullstack-web.json"),
    include_str!("../../assets/blueprints/cli-tool.json"),
    include_str!("../../assets/blueprints/agent-plugin.json"),
];

pub fn list_blueprints() -> Result<Vec<Blueprint>, AppError> {
    let mut out = Vec::new();
    for raw in BUILTIN {
        let bp: Blueprint = serde_json::from_str(raw)
            .map_err(|e| AppError::Message(format!("解析 Blueprint 失败: {e}")))?;
        out.push(bp);
    }
    Ok(out)
}

pub fn get_blueprint(id: &str) -> Result<Blueprint, AppError> {
    list_blueprints()?
        .into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| AppError::InvalidInput(format!("Blueprint 不存在: {id}")))
}

fn parse_target_app(bp: &Blueprint) -> Result<AppType, AppError> {
    bp.target_app
        .parse::<AppType>()
        .map_err(|_| AppError::InvalidInput(format!("无效 target_app: {}", bp.target_app)))
}

pub fn preview_apply_blueprint(
    state: &AppState,
    project_id: &str,
    blueprint_id: &str,
) -> Result<BlueprintApplyPreview, AppError> {
    let bp = get_blueprint(blueprint_id)?;
    let app = parse_target_app(&bp)?;
    let db = &state.db;

    let mut to_link = Vec::new();
    let mut warnings = Vec::new();

    if bp.link_all_mcp_for_target {
        let servers = db.get_all_mcp_servers()?;
        let linked: std::collections::HashSet<_> = db
            .get_project_mcp_servers(project_id)?
            .into_iter()
            .filter(|l| l.enabled)
            .map(|l| l.config_id)
            .collect();
        collect_mcp_actions(&servers, &app, &linked, &mut to_link);
        if to_link.iter().filter(|a| a.asset_type == "mcp").count() == 0 {
            warnings.push("全局库中无适用于目标 CLI 的 MCP 服务器".into());
        }
    }

    if bp.link_all_skills_for_target {
        let skills = db.get_all_installed_skills()?;
        let linked: std::collections::HashSet<_> = db
            .get_project_skills(project_id)?
            .into_iter()
            .filter(|l| l.enabled)
            .map(|l| l.config_id)
            .collect();
        collect_skill_actions(&skills, &app, &linked, &mut to_link);
    }

    if bp.link_all_prompts_for_target {
        let prompts = db.get_prompts(app.as_str())?;
        let linked: std::collections::HashSet<_> = db
            .get_project_prompts(project_id)?
            .into_iter()
            .filter(|l| l.enabled)
            .map(|l| format!("{}:{}", l.prompt_id, l.prompt_app_type))
            .collect();
        collect_prompt_actions(&prompts, &app, &linked, &mut to_link);
    }

    if bp.link_all_commands_for_target {
        let commands = db.get_all_commands()?;
        let linked: std::collections::HashSet<_> = db
            .get_project_asset_links(project_id, Some("command"))?
            .into_iter()
            .filter(|l| l.enabled)
            .map(|l| format!("{}:{}", l.asset_id, l.asset_app_type))
            .collect();
        collect_command_actions(&commands, &app, &linked, &mut to_link);
    }

    for (flag, asset_type) in [
        (bp.link_all_hooks_for_target, "hook"),
        (bp.link_all_ignore_for_target, "ignore"),
        (bp.link_all_permissions_for_target, "permission"),
        (bp.link_all_subagents_for_target, "subagent"),
    ] {
        if flag {
            collect_extended_actions(db, project_id, asset_type, &app, &mut to_link, &mut warnings);
        }
    }

    Ok(BlueprintApplyPreview {
        blueprint_id: bp.id,
        blueprint_name: bp.name,
        target_app: bp.target_app,
        to_link,
        warnings,
    })
}

fn collect_mcp_actions(
    servers: &indexmap::IndexMap<String, McpServer>,
    app: &AppType,
    linked: &std::collections::HashSet<String>,
    out: &mut Vec<BlueprintLinkAction>,
) {
    for (id, server) in servers {
        if server.apps.is_enabled_for(app) && !linked.contains(id) {
            out.push(BlueprintLinkAction {
                asset_type: "mcp".into(),
                asset_id: id.clone(),
                app_type: None,
                action: "link".into(),
            });
        }
    }
}

fn collect_skill_actions(
    skills: &indexmap::IndexMap<String, InstalledSkill>,
    app: &AppType,
    linked: &std::collections::HashSet<String>,
    out: &mut Vec<BlueprintLinkAction>,
) {
    for (id, skill) in skills {
        if skill.apps.is_enabled_for(app) && !linked.contains(id) {
            out.push(BlueprintLinkAction {
                asset_type: "skill".into(),
                asset_id: id.clone(),
                app_type: None,
                action: "link".into(),
            });
        }
    }
}

fn collect_prompt_actions(
    prompts: &indexmap::IndexMap<String, Prompt>,
    app: &AppType,
    linked: &std::collections::HashSet<String>,
    out: &mut Vec<BlueprintLinkAction>,
) {
    let app_str = app.as_str();
    for (id, prompt) in prompts {
        if prompt.is_fragment {
            continue;
        }
        let key = format!("{id}:{app_str}");
        if !linked.contains(&key) {
            out.push(BlueprintLinkAction {
                asset_type: "prompt".into(),
                asset_id: id.clone(),
                app_type: Some(app_str.to_string()),
                action: "link".into(),
            });
        }
    }
}

fn collect_command_actions(
    commands: &indexmap::IndexMap<String, Command>,
    app: &AppType,
    linked: &std::collections::HashSet<String>,
    out: &mut Vec<BlueprintLinkAction>,
) {
    let app_str = app.as_str();
    for (id, command) in commands {
        if !command.is_enabled_for(app) {
            continue;
        }
        let key = format!("{id}:{app_str}");
        if !linked.contains(&key) {
            out.push(BlueprintLinkAction {
                asset_type: "command".into(),
                asset_id: id.clone(),
                app_type: Some(app_str.to_string()),
                action: "link".into(),
            });
        }
    }
}

fn collect_extended_actions(
    db: &Database,
    project_id: &str,
    asset_type: &str,
    app: &AppType,
    out: &mut Vec<BlueprintLinkAction>,
    warnings: &mut Vec<String>,
) {
    let app_str = app.as_str();
    let linked: std::collections::HashSet<_> = db
        .get_project_asset_links(project_id, Some(asset_type))
        .unwrap_or_default()
        .into_iter()
        .filter(|l| l.enabled)
        .map(|l| format!("{}:{}", l.asset_id, l.asset_app_type))
        .collect();

    let ids: Vec<String> = match asset_type {
        "hook" => db
            .get_all_hooks()
            .unwrap_or_default()
            .into_iter()
            .filter(|h| h.enabled_claude && *app == AppType::Claude)
            .map(|h| h.id)
            .collect(),
        "ignore" => db
            .get_all_ignore_rules()
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.is_enabled_for(app))
            .map(|r| r.id)
            .collect(),
        "permission" => db
            .get_all_tool_permissions()
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.enabled_claude && *app == AppType::Claude)
            .map(|p| p.id)
            .collect(),
        "subagent" => db
            .get_all_agents()
            .unwrap_or_default()
            .values()
            .filter(|a| a.is_enabled_for(app))
            .map(|a| a.id.clone())
            .collect(),
        _ => Vec::new(),
    };

    if ids.is_empty() {
        warnings.push(format!("全局库中无适用于 {app_str} 的 {asset_type} 资产"));
        return;
    }

    for id in ids {
        let key = format!("{id}:{app_str}");
        if !linked.contains(&key) {
            out.push(BlueprintLinkAction {
                asset_type: asset_type.to_string(),
                asset_id: id,
                app_type: Some(app_str.to_string()),
                action: "link".into(),
            });
        }
    }
}

pub fn apply_blueprint_to_project(
    state: &AppState,
    project_id: &str,
    blueprint_id: &str,
) -> Result<BlueprintApplyPreview, AppError> {
    let preview = preview_apply_blueprint(state, project_id, blueprint_id)?;
    let bp = get_blueprint(blueprint_id)?;
    let db = &state.db;

    db.set_project_target_app(project_id, Some(&bp.target_app))?;
    db.set_project_blueprint_id(project_id, Some(blueprint_id))?;

    for action in &preview.to_link {
        match action.asset_type.as_str() {
            "mcp" => {
                db.link_project_mcp_server(project_id, &action.asset_id, true)?;
            }
            "skill" => {
                db.link_project_skill(project_id, &action.asset_id, true)?;
            }
            "prompt" => {
                let app = action
                    .app_type
                    .as_deref()
                    .unwrap_or(bp.target_app.as_str());
                db.link_project_prompt(project_id, &action.asset_id, app, true)?;
            }
            other => {
                let app = action.app_type.as_deref().unwrap_or("");
                db.link_project_asset(
                    project_id,
                    other,
                    &action.asset_id,
                    app,
                    true,
                )?;
            }
        }
    }

    crate::services::project_artifacts::touch_project_governance(state, project_id);
    Ok(preview)
}
