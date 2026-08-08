//! 项目 × Profile 分配（Git MVP M1）
//!
//! 约束（范围冻结 §二 约束 1）：单 Profile，无继承、无组合。
//! 一个项目同一时刻最多绑定一个 Profile。

use serde::Serialize;

/// 分配状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    /// 已分配，待部署或已部署
    Active,
    /// 暂停（用户手动暂停偏差检测）
    Suspended,
    /// 已移除
    Removed,
}

impl AssignmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Removed => "removed",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "suspended" => Self::Suspended,
            "removed" => Self::Removed,
            _ => Self::Active,
        }
    }
}

/// 项目 × Profile 分配记录
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAssignment {
    pub assignment_id: String,
    pub project_id: String,
    pub workspace_id: String,
    pub profile_id: String,
    pub status: AssignmentStatus,
    /// 上次成功部署的时间戳（Unix seconds）
    pub deployed_at: Option<i64>,
    /// 上次部署对应的 release_id
    pub deployed_release_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 生成确定性分配 ID（project_id + workspace_id 的 SHA-256 前 16 hex）
pub fn generate_assignment_id(project_id: &str, workspace_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let input = format!("{project_id}:{workspace_id}");
    let hash = Sha256::digest(input.as_bytes());
    let hex: String = hash[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("asgn_{hex}")
}

/// 分配前校验：确认 profile_id 存在于工作区的 team.toml 中
pub fn validate_assignment(
    profile_id: &str,
    available_profiles: &[String],
) -> Result<(), AssignmentError> {
    if profile_id.is_empty() {
        return Err(AssignmentError::EmptyProfileId);
    }
    if !available_profiles.iter().any(|p| p == profile_id) {
        return Err(AssignmentError::ProfileNotFound(profile_id.to_string()));
    }
    Ok(())
}

/// 分配错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentError {
    /// profile_id 为空
    EmptyProfileId,
    /// 指定的 Profile 在工作区中不存在
    ProfileNotFound(String),
    /// 项目已有活跃分配（单 Profile 约束）
    AlreadyAssigned { existing_profile_id: String },
    /// 项目不存在
    ProjectNotFound(String),
    /// 工作区不存在
    WorkspaceNotFound(String),
}

impl std::fmt::Display for AssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProfileId => write!(f, "profile_id cannot be empty"),
            Self::ProfileNotFound(id) => {
                write!(f, "profile '{id}' not found in team workspace")
            }
            Self::AlreadyAssigned {
                existing_profile_id,
            } => write!(
                f,
                "project already assigned to profile '{existing_profile_id}' (single-profile constraint)"
            ),
            Self::ProjectNotFound(id) => write!(f, "project '{id}' not found"),
            Self::WorkspaceNotFound(id) => write!(f, "workspace '{id}' not found"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_deterministic_assignment_id() {
        let id1 = generate_assignment_id("proj_abc", "ws_xyz");
        let id2 = generate_assignment_id("proj_abc", "ws_xyz");
        assert_eq!(id1, id2);
        assert!(id1.starts_with("asgn_"));
        assert_eq!(id1.len(), 5 + 16); // "asgn_" + 16 hex chars
    }

    #[test]
    fn different_inputs_produce_different_ids() {
        let id1 = generate_assignment_id("proj_a", "ws_1");
        let id2 = generate_assignment_id("proj_b", "ws_1");
        let id3 = generate_assignment_id("proj_a", "ws_2");
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn validates_profile_exists() {
        let profiles = vec!["backend".to_string(), "frontend".to_string()];
        assert!(validate_assignment("backend", &profiles).is_ok());
        assert!(matches!(
            validate_assignment("devops", &profiles),
            Err(AssignmentError::ProfileNotFound(_))
        ));
    }

    #[test]
    fn rejects_empty_profile_id() {
        let profiles = vec!["backend".to_string()];
        assert!(matches!(
            validate_assignment("", &profiles),
            Err(AssignmentError::EmptyProfileId)
        ));
    }

    #[test]
    fn status_roundtrip() {
        assert_eq!(
            AssignmentStatus::from_str("active"),
            AssignmentStatus::Active
        );
        assert_eq!(
            AssignmentStatus::from_str("suspended"),
            AssignmentStatus::Suspended
        );
        assert_eq!(
            AssignmentStatus::from_str("removed"),
            AssignmentStatus::Removed
        );
        assert_eq!(
            AssignmentStatus::from_str("unknown"),
            AssignmentStatus::Active
        );
    }
}
