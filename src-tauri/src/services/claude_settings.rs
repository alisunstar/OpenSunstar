use serde_json::{json, Value};
use std::path::Path;

use crate::config::{get_claude_settings_path, read_json_file, write_json_file};
use crate::error::AppError;

pub struct ClaudeSettingsMerger;

impl ClaudeSettingsMerger {
    pub fn read_settings_or_default() -> Value {
        Self::read_settings_at_path(&get_claude_settings_path())
    }

    pub fn read_settings_at_path(path: &Path) -> Value {
        if path.exists() {
            read_json_file::<Value>(path).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        }
    }

    /// Atomically update a top-level field in Claude settings.json.
    pub fn update_field(field: &str, value: Value) -> Result<(), AppError> {
        Self::update_field_at_path(&get_claude_settings_path(), field, value)
    }

    pub fn update_field_at_path(path: &Path, field: &str, value: Value) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }
        let mut settings = Self::read_settings_at_path(path);
        let backup = settings.clone();
        settings[field] = value;

        match write_json_file(path, &settings) {
            Ok(_) => Ok(()),
            Err(e) => {
                let _ = write_json_file(path, &backup);
                Err(e)
            }
        }
    }

    pub fn remove_field_at_path(path: &Path, field: &str) -> Result<(), AppError> {
        if !path.exists() {
            return Ok(());
        }
        let mut settings = Self::read_settings_at_path(path);
        let backup = settings.clone();
        if let Some(obj) = settings.as_object_mut() {
            obj.remove(field);
        }
        match write_json_file(path, &settings) {
            Ok(_) => Ok(()),
            Err(e) => {
                let _ = write_json_file(path, &backup);
                Err(e)
            }
        }
    }
}
