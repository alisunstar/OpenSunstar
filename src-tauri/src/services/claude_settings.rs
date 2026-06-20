use serde_json::{json, Value};

use crate::config::{get_claude_settings_path, read_json_file, write_json_file};
use crate::error::AppError;

pub struct ClaudeSettingsMerger;

impl ClaudeSettingsMerger {
    pub fn read_settings_or_default() -> Value {
        let path = get_claude_settings_path();
        if path.exists() {
            read_json_file::<Value>(&path).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        }
    }

    /// Atomically update a top-level field in Claude settings.json.
    pub fn update_field(field: &str, value: Value) -> Result<(), AppError> {
        let path = get_claude_settings_path();
        let mut settings = Self::read_settings_or_default();
        let backup = settings.clone();
        settings[field] = value;

        match write_json_file(&path, &settings) {
            Ok(_) => Ok(()),
            Err(e) => {
                let _ = write_json_file(&path, &backup);
                Err(e)
            }
        }
    }
}
