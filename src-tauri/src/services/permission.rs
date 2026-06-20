use serde_json::json;

use crate::error::AppError;
use crate::services::claude_settings::ClaudeSettingsMerger;
use crate::store::AppState;
use crate::tool_permission::{
    validate_permission_type, validate_tool_pattern, ToolPermission,
};

pub struct PermissionService;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionPreset {
    pub id: String,
    pub label: String,
    pub description: String,
}

impl PermissionService {
    pub fn get_presets() -> Vec<PermissionPreset> {
        vec![
            PermissionPreset {
                id: "loose".into(),
                label: "宽松".into(),
                description: "允许 Read、Write、Edit、Bash".into(),
            },
            PermissionPreset {
                id: "standard".into(),
                label: "标准".into(),
                description: "允许 Read、Write 及常见 npm/git 命令".into(),
            },
            PermissionPreset {
                id: "strict".into(),
                label: "严格".into(),
                description: "仅允许 Read，拒绝所有 Bash".into(),
            },
        ]
    }

    pub fn get_all_permissions(state: &AppState) -> Result<Vec<ToolPermission>, AppError> {
        state.db.get_all_tool_permissions()
    }

    pub fn upsert_permission(state: &AppState, perm: ToolPermission) -> Result<(), AppError> {
        validate_permission_type(&perm.permission_type).map_err(AppError::Config)?;
        validate_tool_pattern(&perm.tool_pattern).map_err(AppError::Config)?;
        state.db.save_tool_permission(&perm)?;
        Self::sync_permissions_to_claude(state)
    }

    pub fn delete_permission(state: &AppState, id: &str) -> Result<bool, AppError> {
        let existed = state
            .db
            .get_all_tool_permissions()?
            .iter()
            .any(|p| p.id == id);
        if !existed {
            return Ok(false);
        }
        state.db.delete_tool_permission(id)?;
        Self::sync_permissions_to_claude(state)?;
        Ok(true)
    }

    pub fn apply_preset(state: &AppState, preset_id: &str) -> Result<(), AppError> {
        state.db.clear_tool_permissions()?;
        let now = chrono::Utc::now().timestamp();
        let rules = preset_rules(preset_id, now);
        if rules.is_empty() {
            return Err(AppError::Config(format!("未知预设: {preset_id}")));
        }
        for rule in rules {
            state.db.save_tool_permission(&rule)?;
        }
        Self::sync_permissions_to_claude(state)
    }

    pub fn sync_permissions_to_claude(state: &AppState) -> Result<(), AppError> {
        let perms = state
            .db
            .get_all_tool_permissions()?
            .into_iter()
            .filter(|p| p.enabled_claude)
            .collect::<Vec<_>>();

        let mut allow: Vec<String> = Vec::new();
        let mut deny: Vec<String> = Vec::new();

        for perm in perms {
            match perm.permission_type.as_str() {
                "allowedTools" | "autoApprove" => allow.push(perm.tool_pattern),
                "deniedTools" => deny.push(perm.tool_pattern),
                _ => {}
            }
        }

        allow.sort();
        allow.dedup();
        deny.sort();
        deny.dedup();

        let permissions = json!({
            "allow": allow,
            "deny": deny,
            "additionalDirectories": []
        });

        ClaudeSettingsMerger::update_field("permissions", permissions)
    }
}

fn preset_rules(preset_id: &str, now: i64) -> Vec<ToolPermission> {
    let mk = |idx: i32, permission_type: &str, tool_pattern: &str, desc: &str| ToolPermission {
        id: format!("perm-{preset_id}-{idx}"),
        permission_type: permission_type.into(),
        tool_pattern: tool_pattern.into(),
        enabled_claude: true,
        description: Some(desc.into()),
        sort_index: idx,
        created_at: Some(now),
    };

    match preset_id {
        "loose" => vec![
            mk(0, "allowedTools", "Read", "宽松预设"),
            mk(1, "allowedTools", "Write", "宽松预设"),
            mk(2, "allowedTools", "Edit", "宽松预设"),
            mk(3, "allowedTools", "Bash", "宽松预设"),
        ],
        "standard" => vec![
            mk(0, "allowedTools", "Read", "标准预设"),
            mk(1, "allowedTools", "Write", "标准预设"),
            mk(2, "allowedTools", "Bash(npm run *)", "标准预设"),
            mk(3, "allowedTools", "Bash(git *)", "标准预设"),
            mk(4, "deniedTools", "Bash(rm -rf *)", "标准预设"),
        ],
        "strict" => vec![
            mk(0, "allowedTools", "Read", "严格预设"),
            mk(1, "deniedTools", "Bash(*)", "严格预设"),
            mk(2, "deniedTools", "Write", "严格预设"),
            mk(3, "deniedTools", "Edit", "严格预设"),
        ],
        _ => vec![],
    }
}
