//! Simple Connect 持久化状态（供应商 / 密钥池元数据）

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PoolKeyMeta {
    pub id: String,
    pub label: String,
    pub weight: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleConnectState {
    pub supplier_id: String,
    pub custom_openai_base: Option<String>,
    pub pool_enabled: bool,
    pub fail_threshold: u32,
    pub preferred_key_id: Option<String>,
    pub pool_keys: Vec<PoolKeyMeta>,
    pub last_model: Option<String>,
    pub last_tool: Option<String>,
    pub last_applied_supplier_id: Option<String>,
    #[serde(default = "default_true")]
    pub deeplink_import_enabled: bool,
    #[serde(default = "default_true")]
    pub require_key_verify: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SimpleConnectState {
    fn default() -> Self {
        Self {
            supplier_id: "deepseek".into(),
            custom_openai_base: None,
            pool_enabled: true,
            fail_threshold: 1,
            preferred_key_id: None,
            pool_keys: vec![PoolKeyMeta {
                id: "primary".into(),
                label: "主 Key".into(),
                weight: 1,
                enabled: true,
            }],
            last_model: None,
            last_tool: None,
            last_applied_supplier_id: None,
            deeplink_import_enabled: true,
            require_key_verify: true,
        }
    }
}

pub fn state_path() -> PathBuf {
    crate::config::get_app_config_dir()
        .join("simple-connect")
        .join("state.json")
}

pub fn load_state() -> Result<SimpleConnectState, AppError> {
    let path = state_path();
    if !path.exists() {
        return Ok(SimpleConnectState::default());
    }
    crate::config::read_json_file(&path)
}

pub fn save_state(state: &SimpleConnectState) -> Result<(), AppError> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    crate::config::write_json_file(&path, state)
}

pub fn set_supplier(
    supplier_id: &str,
    custom_openai_base: Option<&str>,
) -> Result<SimpleConnectState, AppError> {
    let mut state = load_state()?;
    state.supplier_id = supplier_id.to_string();
    state.custom_openai_base = custom_openai_base
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    save_state(&state)?;
    Ok(state)
}
