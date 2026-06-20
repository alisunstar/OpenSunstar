use serde::{Deserialize, Serialize};

pub const PERMISSION_TYPES: [&str; 3] = ["allowedTools", "deniedTools", "autoApprove"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermission {
    pub id: String,
    pub permission_type: String,
    pub tool_pattern: String,
    #[serde(default = "default_enabled_claude")]
    pub enabled_claude: bool,
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
