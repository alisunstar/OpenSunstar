//! Multi-CLI permission sync orchestrator (L1-03).

mod claude;
mod codex;
mod gemini;
mod hermes;
mod openclaw;
mod opencode;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::store::AppState;
use crate::tool_permission::ToolPermission;

const PERM_SYNC_APPS: [AppType; 6] = [
    AppType::Claude,
    AppType::Codex,
    AppType::Gemini,
    AppType::OpenCode,
    AppType::OpenClaw,
    AppType::Hermes,
];

#[derive(Debug, Clone)]
pub struct PermissionLists {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub auto_approve: Vec<String>,
}

pub fn collect_permission_lists(perms: &[ToolPermission]) -> PermissionLists {
    let mut allow = Vec::new();
    let mut deny = Vec::new();
    let mut auto_approve = Vec::new();

    for perm in perms {
        match perm.permission_type.as_str() {
            "allowedTools" => allow.push(perm.tool_pattern.clone()),
            "deniedTools" => deny.push(perm.tool_pattern.clone()),
            "autoApprove" => auto_approve.push(perm.tool_pattern.clone()),
            _ => {}
        }
    }

    allow.sort();
    allow.dedup();
    deny.sort();
    deny.dedup();
    auto_approve.sort();
    auto_approve.dedup();

    PermissionLists {
        allow,
        deny,
        auto_approve,
    }
}

pub fn sync_all_apps(state: &AppState) -> Result<(), AppError> {
    for app in PERM_SYNC_APPS {
        sync_app(state, &app)?;
    }
    Ok(())
}

pub fn sync_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
    let perms: Vec<ToolPermission> = state
        .db
        .get_all_tool_permissions()?
        .into_iter()
        .filter(|p| p.is_enabled_for(app))
        .collect();
    let lists = collect_permission_lists(&perms);

    match app {
        AppType::Claude => claude::sync_permissions(&lists),
        AppType::Codex => codex::sync_permissions(&lists),
        AppType::Gemini => gemini::sync_permissions(&lists),
        AppType::OpenCode => opencode::sync_permissions(&lists),
        AppType::OpenClaw => openclaw::sync_permissions(&lists),
        AppType::Hermes => hermes::sync_permissions(&lists),
        AppType::ClaudeDesktop => Err(AppError::Config(
            "Claude Desktop 不支持 Permissions 同步".into(),
        )),
    }
}

pub fn sync_permissions_at_path(
    lists: &PermissionLists,
    app: &AppType,
    config_path: &std::path::Path,
) -> Result<(), AppError> {
    match app {
        AppType::Claude => claude::sync_permissions_at_path(lists, config_path),
        AppType::Codex => codex::sync_permissions_at_path(lists, config_path),
        AppType::Gemini => gemini::sync_permissions_at_path(lists, config_path),
        AppType::OpenCode => opencode::sync_permissions_at_path(lists, config_path),
        AppType::Hermes => hermes::sync_permissions_at_path(lists, config_path),
        AppType::OpenClaw | AppType::ClaudeDesktop => Err(AppError::Config(format!(
            "{app:?} 不支持项目级 Permissions 同步"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::opencode::parse_opencode_tool_pattern;

    #[test]
    fn parse_bash_git_pattern() {
        let (tool, pattern) = parse_opencode_tool_pattern("Bash(git *)").unwrap();
        assert_eq!(tool, "bash");
        assert_eq!(pattern, "git *");
    }

    #[test]
    fn parse_simple_tool() {
        let (tool, pattern) = parse_opencode_tool_pattern("Read").unwrap();
        assert_eq!(tool, "read");
        assert_eq!(pattern, "*");
    }
}
