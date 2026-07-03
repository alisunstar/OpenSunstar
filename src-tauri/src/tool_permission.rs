use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::app_config::AppType;

pub const PERMISSION_TYPES: [&str; 3] = ["allowedTools", "deniedTools", "autoApprove"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermission {
    pub id: String,
    pub permission_type: String,
    pub tool_pattern: String,
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
    #[serde(default)]
    pub enabled_openclaw: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub sort_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

fn default_enabled_claude() -> bool {
    true
}

impl ToolPermission {
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
        if self.enabled_openclaw {
            apps.insert(AppType::OpenClaw);
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
            AppType::OpenClaw => self.enabled_openclaw = enabled,
            AppType::ClaudeDesktop => {}
        }
    }

    pub fn is_enabled_for(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.enabled_claude,
            AppType::Codex => self.enabled_codex,
            AppType::Gemini => self.enabled_gemini,
            AppType::OpenCode => self.enabled_opencode,
            AppType::Hermes => self.enabled_hermes,
            AppType::OpenClaw => self.enabled_openclaw,
            AppType::ClaudeDesktop => false,
        }
    }
}

pub fn validate_permission_type(permission_type: &str) -> Result<(), String> {
    if PERMISSION_TYPES.contains(&permission_type) {
        Ok(())
    } else {
        Err(format!("无效的权限类型: {permission_type}"))
    }
}

pub fn validate_tool_pattern(pattern: &str) -> Result<(), String> {
    if pattern.trim().is_empty() {
        return Err("工具匹配模式不能为空".into());
    }
    Ok(())
}
