//! 本地审计事件（Git MVP M8）
//!
//! Git 模式下 actor_ref 恒为 "local-device"（无身份系统）。
//! 审计事件仅存本地，不随云同步（SYNC_SENSITIVE_TABLES）。

use serde::Serialize;

/// 审计动作类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// 连接团队配置源
    Connect,
    /// 校验团队包
    Validate,
    /// 分配 Profile 到项目
    Assign,
    /// 取消分配
    Unassign,
    /// 部署写入
    Deploy,
    /// 偏差检测
    DriftCheck,
    /// 回滚
    Rollback,
    /// 凭证绑定
    CredentialBind,
    /// Release Diff
    ReleaseDiff,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Validate => "validate",
            Self::Assign => "assign",
            Self::Unassign => "unassign",
            Self::Deploy => "deploy",
            Self::DriftCheck => "drift_check",
            Self::Rollback => "rollback",
            Self::CredentialBind => "credential_bind",
            Self::ReleaseDiff => "release_diff",
        }
    }
}

/// Git 模式下的固定 actor 标识
pub const LOCAL_DEVICE_ACTOR: &str = "local-device";

/// 审计事件（写入 SQLite 的行）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAuditEvent {
    pub event_id: String,
    pub workspace_id: Option<String>,
    pub project_id: Option<String>,
    pub action: String,
    pub actor_ref: String,
    /// 操作结果摘要（JSON）
    pub details_json: Option<String>,
    /// 是否成功
    pub success: bool,
    pub created_at: i64,
}

/// 生成事件 ID（时间戳 + 随机后缀）
pub fn generate_event_id() -> String {
    use sha2::{Digest, Sha256};
    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let hash = Sha256::digest(format!("{now}_{}", std::process::id()).as_bytes());
    let hex: String = hash[..6].iter().map(|b| format!("{b:02x}")).collect();
    format!("evt_{hex}")
}

/// 构建审计事件的便捷函数
pub fn make_audit_event(
    action: AuditAction,
    workspace_id: Option<String>,
    project_id: Option<String>,
    details: Option<serde_json::Value>,
    success: bool,
) -> TeamAuditEvent {
    TeamAuditEvent {
        event_id: generate_event_id(),
        workspace_id,
        project_id,
        action: action.as_str().to_string(),
        actor_ref: LOCAL_DEVICE_ACTOR.to_string(),
        details_json: details.map(|d| d.to_string()),
        success,
        created_at: chrono::Utc::now().timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unique_event_ids() {
        let id1 = generate_event_id();
        let id2 = generate_event_id();
        assert!(id1.starts_with("evt_"));
        assert!(id2.starts_with("evt_"));
        assert_eq!(id1.len(), 4 + 12); // "evt_" + 12 hex chars
        assert_eq!(id2.len(), 4 + 12);
    }

    #[test]
    fn makes_audit_event_with_correct_fields() {
        let event = make_audit_event(
            AuditAction::Deploy,
            Some("ws_123".to_string()),
            Some("proj_456".to_string()),
            Some(serde_json::json!({ "files_written": 3 })),
            true,
        );
        assert_eq!(event.action, "deploy");
        assert_eq!(event.actor_ref, "local-device");
        assert_eq!(event.workspace_id, Some("ws_123".to_string()));
        assert_eq!(event.project_id, Some("proj_456".to_string()));
        assert!(event.success);
        assert!(event.details_json.unwrap().contains("files_written"));
    }

    #[test]
    fn action_as_str_covers_all_variants() {
        assert_eq!(AuditAction::Connect.as_str(), "connect");
        assert_eq!(AuditAction::Validate.as_str(), "validate");
        assert_eq!(AuditAction::Assign.as_str(), "assign");
        assert_eq!(AuditAction::Unassign.as_str(), "unassign");
        assert_eq!(AuditAction::Deploy.as_str(), "deploy");
        assert_eq!(AuditAction::DriftCheck.as_str(), "drift_check");
        assert_eq!(AuditAction::Rollback.as_str(), "rollback");
        assert_eq!(AuditAction::CredentialBind.as_str(), "credential_bind");
        assert_eq!(AuditAction::ReleaseDiff.as_str(), "release_diff");
    }
}
