use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::app_config::AppType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: String,
    #[serde(default = "default_arguments")]
    pub arguments: String,
    #[serde(default)]
    pub enabled_claude: bool,
    #[serde(default)]
    pub enabled_codex: bool,
    #[serde(default)]
    pub enabled_gemini: bool,
    #[serde(default)]
    pub enabled_opencode: bool,
    #[serde(default)]
    pub enabled_hermes: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

fn default_arguments() -> String {
    "[]".to_string()
}

impl Command {
    pub fn enabled_apps(&self) -> HashSet<AppType> {
        let mut apps = HashSet::new();
        if self.enabled_claude {
            apps.insert(AppType::Claude);
        }
        if self.enabled_codex {
            apps.insert(AppType::Codex);
        }
        if self.enabled_gemini {
            apps.insert(AppType::Gemini);
        }
        if self.enabled_opencode {
            apps.insert(AppType::OpenCode);
        }
        if self.enabled_hermes {
            apps.insert(AppType::Hermes);
        }
        apps
    }

    pub fn set_enabled_for(&mut self, app: &AppType, enabled: bool) {
        match app {
            AppType::Claude => self.enabled_claude = enabled,
            AppType::Codex => self.enabled_codex = enabled,
            AppType::Gemini => self.enabled_gemini = enabled,
            AppType::OpenCode => self.enabled_opencode = enabled,
            AppType::Hermes => self.enabled_hermes = enabled,
            AppType::OpenClaw | AppType::ClaudeDesktop => {}
        }
    }

    pub fn is_enabled_for(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.enabled_claude,
            AppType::Codex => self.enabled_codex,
            AppType::Gemini => self.enabled_gemini,
            AppType::OpenCode => self.enabled_opencode,
            AppType::Hermes => self.enabled_hermes,
            AppType::OpenClaw | AppType::ClaudeDesktop => false,
        }
    }
}

pub fn validate_command_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("命令名称不能为空".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err("命令名称不能包含 /、\\ 或 ..".into());
    }
    Ok(())
}
