use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::config::{get_claude_config_dir, get_claude_settings_path, get_home_dir};
use crate::gemini_config::{get_gemini_dir, get_gemini_env_path, get_gemini_settings_path};
use crate::services::simple_connect::list_tool_status;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCliAuthStatus {
    tool_key: String,
    simple_connect_tool: String,
    display_name: String,
    access_mode: String,
    credential_state: String,
    route_state: String,
    confidence: String,
    action: String,
    config_path: String,
    config_exists: bool,
    selected_auth_type: Option<String>,
    simple_connect_configured: bool,
    simple_connect_base_url: Option<String>,
    simple_connect_model: Option<String>,
    key_hint: Option<String>,
    detected_sources: Vec<String>,
    evidence: Vec<String>,
}

#[tauri::command]
pub fn get_local_cli_auth_status() -> Result<Vec<LocalCliAuthStatus>, String> {
    let simple_status = list_tool_status().map_err(|e| e.to_string())?;

    Ok(vec![
        probe_claude_status(&simple_status),
        probe_gemini_status(&simple_status),
    ])
}

fn probe_claude_status(
    simple_status: &[crate::services::simple_connect::ToolConfigStatus],
) -> LocalCliAuthStatus {
    let config_path = get_claude_settings_path();
    let config_exists = config_path.exists();
    let value = read_json_value(&config_path);
    let env = value
        .as_ref()
        .and_then(|v| v.get("env"))
        .and_then(Value::as_object);

    let mut detected_sources = Vec::new();
    let mut evidence = Vec::new();

    let managed_marker = env
        .and_then(|e| e.get("OPEN_SUNSTAR_SIMPLE_CONNECT"))
        .and_then(Value::as_str)
        .is_some_and(crate::services::simple_connect::is_managed_marker);
    if managed_marker {
        detected_sources.push("opensunstar_simple_connect".to_string());
        evidence.push("settings.json 中发现 OpenSunstar Simple Connect 标记".to_string());
    }

    let has_config_api_key =
        env.is_some_and(|e| has_any_non_empty(e, &["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"]));
    if has_config_api_key {
        detected_sources.push("config_api_key".to_string());
        evidence.push("Claude Code 配置中发现 Anthropic Key/Token 字段".to_string());
    }

    let has_env_api_key = has_process_env(&["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"]);
    if has_env_api_key {
        detected_sources.push("process_env_api_key".to_string());
        evidence.push("当前 OpenSunstar 进程环境中发现 Anthropic Key/Token 变量".to_string());
    }

    let credential_file = first_existing_path(&claude_credential_candidates());
    if let Some(path) = credential_file {
        detected_sources.push("official_cli_credential_file".to_string());
        evidence.push(format!(
            "发现 Claude Code 本地凭据文件信号: {}",
            redact_home_path(&path)
        ));
    }

    let simple = simple_status
        .iter()
        .find(|item| item.tool.as_str() == "claude-code");
    let simple_connect_configured = simple.is_some_and(|item| item.configured);
    let simple_connect_base_url = simple.and_then(|item| item.base_url.clone());
    let simple_connect_model = simple.and_then(|item| item.model.clone());
    let key_hint = simple.and_then(|item| item.key_hint.clone());

    if simple_connect_configured {
        evidence.push("Simple Connect 已应用到 Claude Code".to_string());
    }

    let has_official_signal = detected_sources
        .iter()
        .any(|source| source == "official_cli_credential_file");
    let has_key_signal =
        managed_marker || has_config_api_key || has_env_api_key || key_hint.is_some();
    let (access_mode, credential_state, route_state, confidence, action) = classify_status(
        simple_connect_configured,
        has_key_signal,
        has_official_signal,
        has_config_api_key || key_hint.is_some(),
        "open_auth_center",
    );

    LocalCliAuthStatus {
        tool_key: "claude".to_string(),
        simple_connect_tool: "claude-code".to_string(),
        display_name: "Claude Code".to_string(),
        access_mode,
        credential_state,
        route_state,
        confidence,
        action,
        config_path: config_path.to_string_lossy().to_string(),
        config_exists,
        selected_auth_type: None,
        simple_connect_configured,
        simple_connect_base_url,
        simple_connect_model,
        key_hint,
        detected_sources,
        evidence,
    }
}

fn probe_gemini_status(
    simple_status: &[crate::services::simple_connect::ToolConfigStatus],
) -> LocalCliAuthStatus {
    let settings_path = get_gemini_settings_path();
    let env_path = get_gemini_env_path();
    let settings_exists = settings_path.exists();
    let env_exists = env_path.exists();
    let value = read_json_value(&settings_path);
    let selected_auth_type = value.as_ref().and_then(read_gemini_selected_auth_type);

    let mut detected_sources = Vec::new();
    let mut evidence = Vec::new();

    if let Some(auth_type) = selected_auth_type.as_deref() {
        detected_sources.push(format!("selected_auth_type:{auth_type}"));
        evidence.push(format!("Gemini settings.json 指定认证类型: {auth_type}"));
    }

    let env_text = std::fs::read_to_string(&env_path).unwrap_or_default();
    let has_env_file_key = env_text.lines().any(|line| {
        let trimmed = line.trim();
        ["GEMINI_API_KEY=", "GOOGLE_API_KEY=", "OPENAI_API_KEY="]
            .iter()
            .any(|prefix| trimmed.starts_with(prefix) && trimmed.len() > prefix.len())
    });
    let has_managed_marker = env_text.contains("opensunstar-simple-connect");
    if has_env_file_key {
        detected_sources.push("gemini_env_api_key".to_string());
        evidence.push("Gemini .env 中发现 API Key 字段".to_string());
    }
    if has_managed_marker {
        detected_sources.push("opensunstar_simple_connect".to_string());
        evidence.push("Gemini .env 中发现 OpenSunstar Simple Connect 标记".to_string());
    }

    let oauth_credential_file = first_existing_path(&gemini_oauth_credential_candidates());
    if let Some(path) = oauth_credential_file {
        detected_sources.push("google_oauth_credential_file".to_string());
        evidence.push(format!(
            "发现 Gemini CLI Google OAuth 凭据文件信号: {}",
            redact_home_path(&path)
        ));
    }

    let has_vertex_env = has_process_env(&[
        "GOOGLE_APPLICATION_CREDENTIALS",
        "GOOGLE_CLOUD_PROJECT",
        "GOOGLE_GENAI_USE_VERTEXAI",
    ]);
    if has_vertex_env {
        detected_sources.push("google_cloud_env".to_string());
        evidence.push("当前 OpenSunstar 进程环境中发现 Google Cloud/Vertex AI 变量".to_string());
    }

    let simple = simple_status
        .iter()
        .find(|item| item.tool.as_str() == "gemini-cli");
    let simple_connect_configured = simple.is_some_and(|item| item.configured);
    let simple_connect_base_url = simple.and_then(|item| item.base_url.clone());
    let simple_connect_model = simple.and_then(|item| item.model.clone());
    let key_hint = simple.and_then(|item| item.key_hint.clone());

    if simple_connect_configured {
        evidence.push("Simple Connect 已应用到 Gemini CLI".to_string());
    }

    let has_official_signal = selected_auth_type.as_deref() == Some("oauth-personal")
        || detected_sources
            .iter()
            .any(|source| source == "google_oauth_credential_file" || source == "google_cloud_env");
    let has_key_signal = has_managed_marker || has_env_file_key || key_hint.is_some();
    let key_is_present = has_env_file_key || key_hint.is_some();
    let (access_mode, credential_state, route_state, confidence, action) = classify_status(
        simple_connect_configured,
        has_key_signal,
        has_official_signal,
        key_is_present,
        "open_auth_center",
    );

    LocalCliAuthStatus {
        tool_key: "gemini".to_string(),
        simple_connect_tool: "gemini-cli".to_string(),
        display_name: "Gemini CLI".to_string(),
        access_mode,
        credential_state,
        route_state,
        confidence,
        action,
        config_path: settings_path.to_string_lossy().to_string(),
        config_exists: settings_exists || env_exists,
        selected_auth_type,
        simple_connect_configured,
        simple_connect_base_url,
        simple_connect_model,
        key_hint,
        detected_sources,
        evidence,
    }
}

fn classify_status(
    simple_connect_configured: bool,
    has_key_signal: bool,
    has_official_signal: bool,
    key_is_present: bool,
    fallback_action: &str,
) -> (String, String, String, String, String) {
    if simple_connect_configured || has_key_signal {
        let credential_state = if key_is_present {
            "present_unverified"
        } else {
            "unknown"
        };
        let route_state = if simple_connect_configured {
            "applied"
        } else {
            "not_applied"
        };
        let action = if simple_connect_configured {
            "none"
        } else {
            "apply"
        };
        return (
            "third_party_key".to_string(),
            credential_state.to_string(),
            route_state.to_string(),
            "medium".to_string(),
            action.to_string(),
        );
    }

    if has_official_signal {
        return (
            "official_cli_login".to_string(),
            "logged_in_detected".to_string(),
            "not_applicable".to_string(),
            "medium".to_string(),
            "none".to_string(),
        );
    }

    (
        "unknown".to_string(),
        "missing".to_string(),
        "unknown".to_string(),
        "low".to_string(),
        fallback_action.to_string(),
    )
}

fn read_json_value(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

fn read_gemini_selected_auth_type(value: &Value) -> Option<String> {
    value
        .pointer("/security/auth/selectedType")
        .and_then(Value::as_str)
        .or_else(|| value.get("selectedAuthType").and_then(Value::as_str))
        .or_else(|| value.get("selectedType").and_then(Value::as_str))
        .map(str::to_string)
}

fn has_any_non_empty(object: &serde_json::Map<String, Value>, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        object
            .get(*key)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn has_process_env(keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        std::env::var(key)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

fn claude_credential_candidates() -> Vec<PathBuf> {
    let dir = get_claude_config_dir();
    vec![
        dir.join(".credentials.json"),
        dir.join("credentials.json"),
        dir.join("auth.json"),
        dir.join(".auth.json"),
    ]
}

fn gemini_oauth_credential_candidates() -> Vec<PathBuf> {
    let dir = get_gemini_dir();
    vec![dir.join("oauth_creds.json"), dir.join("credentials.json")]
}

fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

fn redact_home_path(path: &Path) -> String {
    let home = get_home_dir();
    if let Ok(rest) = path.strip_prefix(&home) {
        return format!("~/{}", rest.to_string_lossy());
    }
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_nested_gemini_selected_auth_type() {
        let value = json!({
            "security": {
                "auth": {
                    "selectedType": "oauth-personal"
                }
            }
        });

        assert_eq!(
            read_gemini_selected_auth_type(&value),
            Some("oauth-personal".to_string())
        );
    }

    #[test]
    fn classifies_simple_connect_as_third_party_key() {
        let (access_mode, credential_state, route_state, confidence, action) =
            classify_status(true, true, false, true, "open_auth_center");

        assert_eq!(access_mode, "third_party_key");
        assert_eq!(credential_state, "present_unverified");
        assert_eq!(route_state, "applied");
        assert_eq!(confidence, "medium");
        assert_eq!(action, "none");
    }

    #[test]
    fn classifies_official_signal_without_route_as_cli_login() {
        let (access_mode, credential_state, route_state, confidence, action) =
            classify_status(false, false, true, false, "open_auth_center");

        assert_eq!(access_mode, "official_cli_login");
        assert_eq!(credential_state, "logged_in_detected");
        assert_eq!(route_state, "not_applicable");
        assert_eq!(confidence, "medium");
        assert_eq!(action, "none");
    }
}
