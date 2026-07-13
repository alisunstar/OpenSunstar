use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::app_config::AppType;

pub const HOOK_EVENT_TYPES: [&str; 4] = ["PreToolUse", "PostToolUse", "Notification", "Stop"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hook {
    pub id: String,
    pub event_type: String,
    #[serde(default = "default_tool_pattern")]
    pub tool_pattern: String,
    pub hook_command: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: i32,
    #[serde(default = "default_enabled_claude")]
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
    pub description: Option<String>,
    #[serde(default)]
    pub sort_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

fn default_tool_pattern() -> String {
    "*".to_string()
}

fn default_timeout() -> i32 {
    30
}

fn default_enabled_claude() -> bool {
    true
}

impl Hook {
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

pub fn validate_hook_event_type(event_type: &str) -> Result<(), String> {
    if HOOK_EVENT_TYPES.contains(&event_type) {
        Ok(())
    } else {
        Err(format!("无效的事件类型: {event_type}"))
    }
}

pub fn validate_timeout(timeout_seconds: i32) -> Result<(), String> {
    if (1..=300).contains(&timeout_seconds) {
        Ok(())
    } else {
        Err("超时时间必须在 1-300 秒之间".into())
    }
}
