use std::path::PathBuf;

use crate::app_config::AppType;
use crate::command::{validate_command_name, Command};
use crate::codex_config::get_codex_config_dir;
use crate::config::{delete_file, write_text_file};
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::hermes_config::get_hermes_dir;
use crate::opencode_config::get_opencode_dir;
use crate::store::AppState;

pub struct CommandService;

impl CommandService {
    pub fn get_all_commands(state: &AppState) -> Result<indexmap::IndexMap<String, Command>, AppError> {
        state.db.get_all_commands()
    }

    pub fn upsert_command(state: &AppState, command: Command) -> Result<(), AppError> {
        validate_command_name(&command.name).map_err(AppError::Config)?;

        let prev_apps = state
            .db
            .get_all_commands()?
            .get(&command.id)
            .map(|c| c.enabled_apps())
            .unwrap_or_default();

        state.db.save_command(&command)?;

        let next_apps = command.enabled_apps();
        for app in prev_apps.difference(&next_apps) {
            Self::remove_command_from_app(&command.name, &app)?;
        }
        for app in next_apps {
            Self::sync_command_to_app(&command, &app)?;
        }

        Ok(())
    }

    pub fn delete_command(state: &AppState, id: &str) -> Result<bool, AppError> {
        let command = state.db.get_all_commands()?.shift_remove(id);
        if let Some(command) = command {
            state.db.delete_command(id)?;
            for app in command.enabled_apps() {
                Self::remove_command_from_app(&command.name, &app)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn toggle_app(
        state: &AppState,
        command_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        let mut commands = state.db.get_all_commands()?;
        if let Some(command) = commands.get_mut(command_id) {
            command.set_enabled_for(&app, enabled);
            let snapshot = command.clone();
            state.db.save_command(&snapshot)?;
            if enabled {
                Self::sync_command_to_app(&snapshot, &app)?;
            } else {
                Self::remove_command_from_app(&snapshot.name, &app)?;
            }
        }
        Ok(())
    }

    fn command_file_path(name: &str, app: &AppType) -> Result<PathBuf, AppError> {
        let safe_name = format!("{name}.md");
        let path = match app {
            AppType::Claude => crate::config::get_claude_config_dir()
                .join("commands")
                .join(&safe_name),
            AppType::Gemini => get_gemini_dir().join("commands").join(&safe_name),
            AppType::OpenCode => get_opencode_dir().join("commands").join(&safe_name),
            AppType::Hermes => get_hermes_dir().join("commands").join(&safe_name),
            AppType::Codex => get_codex_config_dir().join("commands").join(&safe_name),
            AppType::OpenClaw | AppType::ClaudeDesktop => {
                return Err(AppError::Config(format!("{app:?} 不支持 slash 命令同步")));
            }
        };
        Ok(path)
    }

    fn sync_command_to_app(command: &Command, app: &AppType) -> Result<(), AppError> {
        if !command.is_enabled_for(app) {
            return Ok(());
        }
        let path = Self::command_file_path(&command.name, app)?;
        write_text_file(&path, &command.content)?;
        Ok(())
    }

    fn remove_command_from_app(name: &str, app: &AppType) -> Result<(), AppError> {
        let path = Self::command_file_path(name, app)?;
        if path.exists() {
            delete_file(&path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_command_name_rejects_path_chars() {
        assert!(validate_command_name("foo/bar").is_err());
        assert!(validate_command_name("..").is_err());
        assert!(validate_command_name("review-pr").is_ok());
    }
}
