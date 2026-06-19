//! Environment scanning engine for first-run onboarding wizard.
//! Detects existing AI tool configurations on the user's system.

use crate::config::get_home_dir;
use crate::error::AppError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub providers_found: Vec<DetectedProvider>,
    pub mcp_servers_found: Vec<DetectedMcp>,
    pub total_items: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProvider {
    pub app_type: String,
    pub name: String,
    pub config_path: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedMcp {
    pub name: String,
    pub source_app: String,
    pub command: Option<String>,
}

pub fn scan_environment() -> Result<ScanResult, AppError> {
    let mut providers = Vec::new();
    let mut mcp_servers = Vec::new();

    let home = get_home_dir();

    // Claude Code
    let claude_settings = home.join(".claude").join("settings.json");
    if claude_settings.exists() {
        if let Ok(content) = std::fs::read_to_string(&claude_settings) {
            providers.push(DetectedProvider {
                app_type: "claude".to_string(),
                name: "Claude Code".to_string(),
                config_path: claude_settings.to_string_lossy().to_string(),
                has_api_key: content.contains("apiKey") || content.contains("ANTHROPIC_API_KEY"),
            });
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = json.get("mcpServers").and_then(|s| s.as_object()) {
                    for (name, config) in servers {
                        mcp_servers.push(DetectedMcp {
                            name: name.clone(),
                            source_app: "claude".to_string(),
                            command: config
                                .get("command")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string()),
                        });
                    }
                }
            }
        }
    }

    // Codex
    let codex_dir = home.join(".codex");
    if codex_dir.exists() {
        let auth_path = codex_dir.join("auth.json");
        providers.push(DetectedProvider {
            app_type: "codex".to_string(),
            name: "Codex".to_string(),
            config_path: codex_dir.to_string_lossy().to_string(),
            has_api_key: auth_path.exists(),
        });
    }

    // Gemini CLI
    let gemini_dir = home.join(".gemini");
    let gemini_settings = gemini_dir.join("settings.json");
    if gemini_settings.exists() || gemini_dir.exists() {
        providers.push(DetectedProvider {
            app_type: "gemini".to_string(),
            name: "Gemini CLI".to_string(),
            config_path: gemini_dir.to_string_lossy().to_string(),
            has_api_key: gemini_settings.exists(),
        });
    }

    // OpenCode
    let opencode_dir = home.join(".config").join("opencode");
    if opencode_dir.exists() {
        providers.push(DetectedProvider {
            app_type: "opencode".to_string(),
            name: "OpenCode".to_string(),
            config_path: opencode_dir.to_string_lossy().to_string(),
            has_api_key: false,
        });
    }

    // Hermes
    let hermes_dir = home.join(".config").join("hermes");
    if hermes_dir.exists() {
        providers.push(DetectedProvider {
            app_type: "hermes".to_string(),
            name: "Hermes".to_string(),
            config_path: hermes_dir.to_string_lossy().to_string(),
            has_api_key: false,
        });
    }

    // OpenClaw
    let openclaw_dir = home.join(".config").join("openclaw");
    if openclaw_dir.exists() {
        providers.push(DetectedProvider {
            app_type: "openclaw".to_string(),
            name: "OpenClaw".to_string(),
            config_path: openclaw_dir.to_string_lossy().to_string(),
            has_api_key: false,
        });
    }

    let total = providers.len() + mcp_servers.len();
    Ok(ScanResult {
        providers_found: providers,
        mcp_servers_found: mcp_servers,
        total_items: total,
    })
}
