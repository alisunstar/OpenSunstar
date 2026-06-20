use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::app_config::AppType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreRule {
    pub id: String,
    pub pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    #[serde(default)]
    pub sort_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

impl IgnoreRule {
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

pub fn validate_ignore_pattern(pattern: &str) -> Result<(), String> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err("忽略规则不能为空".into());
    }
    Ok(())
}

pub fn parse_gitignore_content(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('!') {
            continue;
        }
        patterns.push(trimmed.to_string());
    }
    patterns
}
