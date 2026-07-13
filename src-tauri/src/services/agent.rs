use std::path::PathBuf;

use crate::agent::{agent_sync_supported, validate_agent_name, Agent};
use crate::app_config::AppType;
use crate::codex_config::get_codex_config_dir;
use crate::config::{delete_file, write_text_file};
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::opencode_config::get_opencode_dir;
use crate::services::agent_codex::markdown_agent_to_codex_toml;
use crate::store::AppState;

pub struct AgentService;

impl AgentService {
    pub fn get_all_agents(state: &AppState) -> Result<indexmap::IndexMap<String, Agent>, AppError> {
        state.db.get_all_agents()
    }

    pub fn upsert_agent(state: &AppState, mut agent: Agent) -> Result<(), AppError> {
        validate_agent_name(&agent.name).map_err(AppError::Config)?;
        agent.normalize_sync_flags();

        let prev_apps = state
            .db
            .get_all_agents()?
            .get(&agent.id)
            .map(|a| a.enabled_apps())
            .unwrap_or_default();

        state.db.save_agent(&agent)?;

        let next_apps = agent.enabled_apps();
        for app in prev_apps.difference(&next_apps) {
            Self::remove_agent_from_app(&agent.name, &app)?;
        }
        for app in next_apps {
            Self::sync_agent_to_app(&agent, &app)?;
        }

        Ok(())
    }

    pub fn delete_agent(state: &AppState, id: &str) -> Result<bool, AppError> {
        let agent = state.db.get_all_agents()?.shift_remove(id);
        if let Some(agent) = agent {
            state.db.delete_agent(id)?;
            for app in agent.enabled_apps() {
                Self::remove_agent_from_app(&agent.name, &app)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn toggle_app(
        state: &AppState,
        agent_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        if !agent_sync_supported(&app) {
            return Err(AppError::Config(Self::unsupported_app_message(&app)));
        }

        let mut agents = state.db.get_all_agents()?;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.set_enabled_for(&app, enabled);
            agent.normalize_sync_flags();
            let snapshot = agent.clone();
            state.db.save_agent(&snapshot)?;
            if enabled {
                Self::sync_agent_to_app(&snapshot, &app)?;
            } else {
                Self::remove_agent_from_app(&snapshot.name, &app)?;
            }
        }
        Ok(())
    }

    pub fn preview_codex_toml(agent: &Agent) -> Result<String, AppError> {
        markdown_agent_to_codex_toml(&agent.name, agent.description.as_deref(), &agent.content)
    }

    fn agent_file_path(name: &str, app: &AppType) -> Result<PathBuf, AppError> {
        if !agent_sync_supported(app) {
            return Err(AppError::Config(Self::unsupported_app_message(app)));
        }

        match app {
            AppType::Claude => Ok(crate::config::get_claude_config_dir()
                .join("agents")
                .join(format!("{name}.md"))),
            AppType::Gemini => Ok(get_gemini_dir().join("agents").join(format!("{name}.md"))),
            AppType::OpenCode => Ok(get_opencode_dir().join("agents").join(format!("{name}.md"))),
            AppType::Codex => Ok(get_codex_config_dir()
                .join("agents")
                .join(format!("{name}.toml"))),
            _ => Err(AppError::Config(Self::unsupported_app_message(app))),
        }
    }

    fn sync_agent_to_app(agent: &Agent, app: &AppType) -> Result<(), AppError> {
        if !agent.is_enabled_for(app) {
            return Ok(());
        }
        let path = Self::agent_file_path(&agent.name, app)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let payload = if matches!(app, AppType::Codex) {
            markdown_agent_to_codex_toml(&agent.name, agent.description.as_deref(), &agent.content)?
        } else {
            agent.content.clone()
        };

        write_text_file(&path, &payload)?;
        Ok(())
    }

    fn remove_agent_from_app(name: &str, app: &AppType) -> Result<(), AppError> {
        if !agent_sync_supported(app) {
            return Ok(());
        }
        let path = Self::agent_file_path(name, app)?;
        if path.exists() {
            delete_file(&path)?;
        }
        Ok(())
    }

    fn unsupported_app_message(app: &AppType) -> String {
        match app {
            AppType::Hermes => "Hermes 暂不支持 Subagent 文件同步（委派走 config.yaml）".into(),
            other => format!("{other:?} 不支持 Subagent 文件同步"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_agent_name_rejects_path_chars() {
        assert!(validate_agent_name("foo/bar").is_err());
        assert!(validate_agent_name("code-reviewer").is_ok());
    }

    #[test]
    fn normalize_clears_hermes_only() {
        let mut agent = Agent {
            id: "a1".into(),
            name: "test".into(),
            description: None,
            content: "Do work.".into(),
            enabled_claude: true,
            enabled_codex: true,
            enabled_gemini: false,
            enabled_opencode: false,
            enabled_hermes: true,
            created_at: None,
            updated_at: None,
        };
        agent.normalize_sync_flags();
        assert!(agent.enabled_codex);
        assert!(!agent.enabled_hermes);
    }

    #[test]
    fn preview_codex_toml_from_markdown() {
        let agent = Agent {
            id: "a1".into(),
            name: "reviewer".into(),
            description: Some("Review PRs".into()),
            content: "---\nname: reviewer\nmodel: gpt-5.5\n---\nReview carefully.".into(),
            enabled_claude: false,
            enabled_codex: true,
            enabled_gemini: false,
            enabled_opencode: false,
            enabled_hermes: false,
            created_at: None,
            updated_at: None,
        };
        let toml = AgentService::preview_codex_toml(&agent).expect("toml");
        assert!(toml.contains("developer_instructions"));
        assert!(toml.contains("gpt-5.5"));
    }
}
