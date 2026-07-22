use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The narrow P0 policy vocabulary used by the first resolver Spike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Required,
    Recommended,
    Denied,
}

impl PolicyAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Recommended => "recommended",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub struct RequirementKey {
    pub project_id: String,
    pub asset_type: String,
    pub asset_id: String,
    pub target_app: String,
}

/// A policy input. It remains independent until compilation, rather than
/// competing for the single `project_asset_expectations` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementSource {
    pub source_id: String,
    pub key: RequirementKey,
    pub scope_kind: String,
    pub scope_id: String,
    pub source_revision: Option<String>,
    pub policy_action: PolicyAction,
    pub required_revision_id: Option<String>,
    pub constraint_json: String,
    pub priority_class: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionConflict {
    pub code: String,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementResolution {
    pub key: RequirementKey,
    pub desired_action: PolicyAction,
    pub required_revision_id: Option<String>,
    pub source_ids: Vec<String>,
    pub conflict: Option<ResolutionConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementResolutionPlan {
    pub entries: Vec<RequirementResolution>,
    pub plan_sha256: String,
}

/// Compile policy sources deterministically. Deny is monotonic: adding a
/// denial can never widen the resolved permission. A required/denied pair or
/// incompatible pinned revisions becomes an explicit conflict, never an
/// implicit precedence decision.
pub fn resolve_requirement_sources(sources: &[RequirementSource]) -> RequirementResolutionPlan {
    let mut grouped = BTreeMap::<RequirementKey, Vec<&RequirementSource>>::new();
    for source in sources {
        grouped.entry(source.key.clone()).or_default().push(source);
    }

    let entries = grouped
        .into_iter()
        .map(|(key, sources)| resolve_group(key, sources))
        .collect::<Vec<_>>();
    let serialized = serde_json::to_vec(&entries).expect("resolution entries are serializable");
    let plan_sha256 = format!("{:x}", Sha256::digest(serialized));
    RequirementResolutionPlan {
        entries,
        plan_sha256,
    }
}

fn resolve_group(
    key: RequirementKey,
    mut sources: Vec<&RequirementSource>,
) -> RequirementResolution {
    sources.sort_by(|left, right| {
        (
            left.priority_class,
            left.policy_action,
            &left.required_revision_id,
            &left.scope_kind,
            &left.scope_id,
            &left.source_id,
        )
            .cmp(&(
                right.priority_class,
                right.policy_action,
                &right.required_revision_id,
                &right.scope_kind,
                &right.scope_id,
                &right.source_id,
            ))
    });
    let source_ids = sources
        .iter()
        .map(|source| source.source_id.clone())
        .collect::<Vec<_>>();
    let has_denied = sources
        .iter()
        .any(|source| source.policy_action == PolicyAction::Denied);
    let required = sources
        .iter()
        .filter(|source| source.policy_action == PolicyAction::Required)
        .collect::<Vec<_>>();

    if has_denied && !required.is_empty() {
        return RequirementResolution {
            key,
            desired_action: PolicyAction::Denied,
            required_revision_id: None,
            source_ids: source_ids.clone(),
            conflict: Some(ResolutionConflict {
                code: "policy_invalid".to_string(),
                source_ids,
            }),
        };
    }

    let revisions = required
        .iter()
        .filter_map(|source| source.required_revision_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if revisions.len() > 1 {
        return RequirementResolution {
            key,
            desired_action: PolicyAction::Required,
            required_revision_id: None,
            source_ids: source_ids.clone(),
            conflict: Some(ResolutionConflict {
                code: "revision_conflict".to_string(),
                source_ids,
            }),
        };
    }

    let desired_action = if has_denied {
        PolicyAction::Denied
    } else if !required.is_empty() {
        PolicyAction::Required
    } else {
        PolicyAction::Recommended
    };
    RequirementResolution {
        key,
        desired_action,
        required_revision_id: revisions.into_iter().next(),
        source_ids,
        conflict: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, action: PolicyAction, revision: Option<&str>) -> RequirementSource {
        RequirementSource {
            source_id: id.to_string(),
            key: RequirementKey {
                project_id: "project-a".to_string(),
                asset_type: "permission".to_string(),
                asset_id: "Bash".to_string(),
                target_app: "claude".to_string(),
            },
            scope_kind: "team_release".to_string(),
            scope_id: "release-1".to_string(),
            source_revision: Some("rev-1".to_string()),
            policy_action: action,
            required_revision_id: revision.map(str::to_string),
            constraint_json: "{}".to_string(),
            priority_class: 10,
        }
    }

    #[test]
    fn deny_is_monotonic_when_lower_priority_recommends_the_same_asset() {
        let plan = resolve_requirement_sources(&[
            source("team-deny", PolicyAction::Denied, None),
            source("project-recommend", PolicyAction::Recommended, None),
        ]);
        assert_eq!(plan.entries[0].desired_action, PolicyAction::Denied);
        assert!(plan.entries[0].conflict.is_none());
    }

    #[test]
    fn required_and_denied_are_an_explicit_policy_conflict() {
        let plan = resolve_requirement_sources(&[
            source("team-required", PolicyAction::Required, Some("revision-a")),
            source("team-denied", PolicyAction::Denied, None),
        ]);
        assert_eq!(
            plan.entries[0]
                .conflict
                .as_ref()
                .map(|conflict| conflict.code.as_str()),
            Some("policy_invalid")
        );
    }

    #[test]
    fn incompatible_pinned_revisions_are_an_explicit_conflict() {
        let plan = resolve_requirement_sources(&[
            source(
                "team-required-a",
                PolicyAction::Required,
                Some("revision-a"),
            ),
            source(
                "project-required-b",
                PolicyAction::Required,
                Some("revision-b"),
            ),
        ]);
        assert_eq!(
            plan.entries[0]
                .conflict
                .as_ref()
                .map(|conflict| conflict.code.as_str()),
            Some("revision_conflict")
        );
    }

    #[test]
    fn source_order_does_not_change_the_compiled_plan_digest() {
        let required = source("team-required", PolicyAction::Required, Some("revision-a"));
        let recommended = source("project-recommended", PolicyAction::Recommended, None);
        let first = resolve_requirement_sources(&[required.clone(), recommended.clone()]);
        let second = resolve_requirement_sources(&[recommended, required]);
        assert_eq!(first, second);
    }
}
