use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::app_config::AppType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: String,
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

impl Agent {
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
        apps
    }

    pub fn set_enabled_for(&mut self, app: &AppType, enabled: bool) {
        match app {
            AppType::Claude => self.enabled_claude = enabled,
            AppType::Codex => self.enabled_codex = enabled,
            AppType::Gemini => self.enabled_gemini = enabled,
            AppType::OpenCode => self.enabled_opencode = enabled,
            AppType::Hermes => {}
            AppType::OpenClaw | AppType::ClaudeDesktop => {}
        }
    }

    pub fn is_enabled_for(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.enabled_claude,
            AppType::Codex => self.enabled_codex,
            AppType::Gemini => self.enabled_gemini,
            AppType::OpenCode => self.enabled_opencode,
            AppType::Hermes | AppType::OpenClaw | AppType::ClaudeDesktop => false,
        }
    }

    /// Hermes 暂不支持文件同步；Codex 通过 TOML 转换写入。
    pub fn normalize_sync_flags(&mut self) {
        self.enabled_hermes = false;
    }
}

pub fn validate_agent_name(name: &str) -> Result<(), String> {
    crate::command::validate_command_name(name)
}

pub fn agent_sync_supported(app: &AppType) -> bool {
    matches!(
        app,
        AppType::Claude | AppType::Codex | AppType::Gemini | AppType::OpenCode
    )
}
