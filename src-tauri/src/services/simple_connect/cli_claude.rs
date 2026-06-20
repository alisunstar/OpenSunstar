//! Claude Code 写入（兼容 Phase 0 Spike API）

use crate::error::AppError;
use crate::services::simple_connect::backup;
use crate::services::simple_connect::tools::{self, MANAGED_MARKER};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClaudeApplyResult {
    pub settings_path: String,
    pub backup_path: Option<String>,
    pub base_url: String,
    pub model: String,
}

pub fn apply_claude_code(
    api_key: &str,
    model: &str,
    anthropic_base: &str,
) -> Result<ClaudeApplyResult, AppError> {
    let paths = tools::claude::paths();
    let settings_path = paths
        .first()
        .ok_or_else(|| AppError::Message("Claude settings 路径缺失".into()))?
        .clone();
    let backup_path = backup::backup_file("claude-code", &settings_path)?;
    tools::claude::apply(api_key, model, anthropic_base)?;
    Ok(ClaudeApplyResult {
        settings_path: settings_path.display().to_string(),
        backup_path: backup_path.map(|p| p.display().to_string()),
        base_url: anthropic_base.trim().trim_end_matches('/').to_string(),
        model: model.trim().to_string(),
    })
}

pub fn is_managed_settings(path: &std::path::PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(v) = crate::config::read_json_file::<serde_json::Value>(path) else {
        return false;
    };
    v.pointer("/env/OPEN_SUNSTAR_SIMPLE_CONNECT")
        .and_then(|x| x.as_str())
        == Some(MANAGED_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{read_json_file, write_json_file};
    use serde_json::json;

    #[test]
    fn managed_marker_constant() {
        assert_eq!(MANAGED_MARKER, "opensunstar-simple-connect");
    }

    #[test]
    fn env_shape_for_claude_apply() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        let mut value = json!({ "env": {} });
        write_json_file(&path, &value).unwrap();
        value["env"]["ANTHROPIC_BASE_URL"] = json!("https://api.deepseek.com/anthropic");
        value["env"]["OPEN_SUNSTAR_SIMPLE_CONNECT"] = json!(MANAGED_MARKER);
        write_json_file(&path, &value).unwrap();
        let read = read_json_file::<serde_json::Value>(&path).unwrap();
        assert_eq!(
            read.pointer("/env/OPEN_SUNSTAR_SIMPLE_CONNECT")
                .and_then(|v| v.as_str()),
            Some(MANAGED_MARKER)
        );
    }
}
