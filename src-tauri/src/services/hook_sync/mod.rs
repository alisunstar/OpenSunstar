//! Multi-CLI hook sync orchestrator (L1-02).

mod claude;
mod codex;
mod gemini;
mod hermes;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::hook::Hook;
use crate::store::AppState;

const HOOK_SYNC_APPS: [AppType; 4] = [
    AppType::Claude,
    AppType::Codex,
    AppType::Gemini,
    AppType::Hermes,
];

pub fn sync_all_apps(state: &AppState) -> Result<(), AppError> {
    for app in HOOK_SYNC_APPS {
        sync_app(state, &app)?;
    }
    Ok(())
}

pub fn sync_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
    let hooks: Vec<Hook> = state
        .db
        .get_all_hooks()?
        .into_iter()
        .filter(|h| h.is_enabled_for(app))
        .collect();

    match app {
        AppType::Claude => claude::sync_hooks(&hooks),
        AppType::Codex => codex::sync_hooks(&hooks),
        AppType::Gemini => gemini::sync_hooks(&hooks),
        AppType::Hermes => hermes::sync_hooks(&hooks),
        AppType::OpenCode | AppType::OpenClaw | AppType::ClaudeDesktop => {
            Err(AppError::Config(format!("{app:?} 不支持 Hooks 同步")))
        }
    }
}

pub fn sync_hooks_at_path(
    hooks: &[Hook],
    app: &AppType,
    config_path: &std::path::Path,
) -> Result<(), AppError> {
    match app {
        AppType::Claude => claude::sync_hooks_at_path(hooks, config_path),
        AppType::Codex => codex::sync_hooks_at_path(hooks, config_path),
        AppType::Gemini => gemini::sync_hooks_at_path(hooks, config_path),
        AppType::Hermes => hermes::sync_hooks_at_path(hooks, config_path),
        AppType::OpenCode | AppType::OpenClaw | AppType::ClaudeDesktop => {
            Err(AppError::Config(format!("{app:?} 不支持项目级 Hooks 同步")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::codex::build_codex_hooks_toml;
    use crate::hook::Hook;

    #[test]
    fn codex_hooks_toml_includes_event_table() {
        let hooks = vec![Hook {
            id: "h1".into(),
            event_type: "PreToolUse".into(),
            tool_pattern: "*".into(),
            hook_command: "echo hi".into(),
            timeout_seconds: 30,
            enabled_claude: false,
            enabled_codex: true,
            enabled_gemini: false,
            enabled_opencode: false,
            enabled_hermes: false,
            description: None,
            sort_index: 0,
            created_at: None,
        }];
        let toml = build_codex_hooks_toml(&hooks);
        assert!(toml.contains("[[hooks.PreToolUse]]"));
        assert!(toml.contains("command = \"echo hi\""));
    }
}
