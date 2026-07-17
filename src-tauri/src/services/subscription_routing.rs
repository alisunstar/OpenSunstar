//! 多订阅账号的本地路由策略。
//!
//! 该模块只保存账号标识和项目策略，绝不保存 OAuth token 或密码；实际凭据仍由
//! 各 CLI / OAuth manager 管理。路由结果是显式建议，调用方必须要求用户确认，
//! 不会在请求中静默轮换订阅账号。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::subscription::{CredentialStatus, SubscriptionQuota};

pub const DEFAULT_MAX_UTILIZATION: f64 = 90.0;
const STORE_FILE_NAME: &str = "subscription_routes.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionProvider {
    Claude,
    Codex,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionRouteMode {
    /// 固定使用首选账号；额度不足时只报告，不建议切换。
    #[default]
    Pinned,
    /// 仅建议下一可用账号，必须由用户在界面中确认后才可变更绑定。
    AdvisoryFailover,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSubscriptionRoute {
    pub project_path: String,
    pub provider: SubscriptionProvider,
    /// 优先级从前到后。账号 ID 仅是认证系统的引用，不包含任何 secret。
    pub account_ids: Vec<String>,
    #[serde(default)]
    pub mode: SubscriptionRouteMode,
    /// 用户确认仅将自己有权使用的账户用于该项目，并理解服务条款。
    pub terms_acknowledged: bool,
    #[serde(default = "default_max_utilization")]
    pub max_utilization: f64,
}

#[derive(Debug, Clone)]
pub struct SubscriptionAccountHealth {
    pub account_id: String,
    pub provider: SubscriptionProvider,
    pub enabled: bool,
    pub quota: SubscriptionQuota,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteDecision {
    UsePrimary,
    AdviseFallback,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteRecommendation {
    pub decision: RouteDecision,
    pub selected_account_id: Option<String>,
    /// `true` means the UI/CLI must ask the user to apply the proposed fallback.
    pub requires_confirmation: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionRouteStore {
    version: u32,
    #[serde(default)]
    routes: Vec<ProjectSubscriptionRoute>,
}

impl Default for SubscriptionRouteStore {
    fn default() -> Self {
        Self {
            version: 1,
            routes: Vec::new(),
        }
    }
}

fn default_max_utilization() -> f64 {
    DEFAULT_MAX_UTILIZATION
}

pub fn route_store_path(config_dir: &Path) -> PathBuf {
    config_dir.join(STORE_FILE_NAME)
}

pub fn load_routes(config_dir: &Path) -> Result<Vec<ProjectSubscriptionRoute>, String> {
    let path = route_store_path(config_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read subscription routes: {error}"))?;
    let store: SubscriptionRouteStore = serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to parse subscription routes: {error}"))?;
    Ok(store.routes)
}

pub fn upsert_route(
    config_dir: &Path,
    mut route: ProjectSubscriptionRoute,
) -> Result<ProjectSubscriptionRoute, String> {
    normalize_route(&mut route)?;
    let mut routes = load_routes(config_dir)?;
    routes.retain(|existing| {
        !(existing.project_path == route.project_path && existing.provider == route.provider)
    });
    routes.push(route.clone());
    let store = SubscriptionRouteStore { version: 1, routes };
    crate::config::write_json_file(&route_store_path(config_dir), &store)
        .map_err(|error| format!("Failed to save subscription routes: {error}"))?;
    Ok(route)
}

pub fn normalize_route(route: &mut ProjectSubscriptionRoute) -> Result<(), String> {
    route.project_path = normalize_project_path(&route.project_path)?;
    route.account_ids = route
        .account_ids
        .iter()
        .map(|account_id| account_id.trim())
        .filter(|account_id| !account_id.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    route.account_ids.dedup();

    if route.account_ids.is_empty() {
        return Err("At least one subscription account is required".to_string());
    }
    if !route.terms_acknowledged {
        return Err(
            "Terms acknowledgement is required before binding subscription accounts to a project"
                .to_string(),
        );
    }
    if !(0.0..=100.0).contains(&route.max_utilization) || route.max_utilization == 0.0 {
        return Err("max_utilization must be greater than 0 and at most 100".to_string());
    }
    Ok(())
}

pub fn recommend_account(
    route: &ProjectSubscriptionRoute,
    accounts: &[SubscriptionAccountHealth],
) -> RouteRecommendation {
    if !route.terms_acknowledged {
        return blocked("Project route has not acknowledged the subscription terms");
    }
    let Some(primary_id) = route.account_ids.first() else {
        return blocked("Project route does not contain an account");
    };

    let lookup = |account_id: &str| {
        accounts
            .iter()
            .find(|account| account.account_id == account_id && account.provider == route.provider)
    };

    if let Some(primary) = lookup(primary_id) {
        if account_is_eligible(primary, route.max_utilization) {
            return RouteRecommendation {
                decision: RouteDecision::UsePrimary,
                selected_account_id: Some(primary.account_id.clone()),
                requires_confirmation: false,
                reason: "Primary account is healthy and below the configured quota threshold"
                    .to_string(),
            };
        }
    }

    if route.mode == SubscriptionRouteMode::Pinned {
        return blocked("Primary account is not eligible and this project is pinned to it");
    }

    if let Some(fallback) = route
        .account_ids
        .iter()
        .skip(1)
        .filter_map(|account_id| lookup(account_id))
        .find(|account| account_is_eligible(account, route.max_utilization))
    {
        return RouteRecommendation {
            decision: RouteDecision::AdviseFallback,
            selected_account_id: Some(fallback.account_id.clone()),
            requires_confirmation: true,
            reason: "Primary account is unavailable; explicit confirmation is required to use the suggested fallback"
                .to_string(),
        };
    }

    blocked("No eligible subscription account is available for this project route")
}

fn account_is_eligible(account: &SubscriptionAccountHealth, max_utilization: f64) -> bool {
    account.enabled
        && account.quota.success
        && matches!(account.quota.credential_status, CredentialStatus::Valid)
        && !account.quota.tiers.is_empty()
        && account
            .quota
            .tiers
            .iter()
            .all(|tier| tier.utilization < max_utilization)
}

fn blocked(reason: impl Into<String>) -> RouteRecommendation {
    RouteRecommendation {
        decision: RouteDecision::Blocked,
        selected_account_id: None,
        requires_confirmation: false,
        reason: reason.into(),
    }
}

fn normalize_project_path(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("project_path is required".to_string());
    }
    let path = Path::new(trimmed);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Failed to resolve project_path: {error}"))?
            .join(path)
    };
    Ok(std::fs::canonicalize(&absolute)
        .unwrap_or(absolute)
        .to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::subscription::{CredentialStatus, QuotaTier};

    fn quota(utilization: f64) -> SubscriptionQuota {
        SubscriptionQuota {
            tool: "codex_oauth".to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: None,
            success: true,
            tiers: vec![QuotaTier {
                name: "five_hour".to_string(),
                utilization,
                resets_at: None,
                used_value_usd: None,
                max_value_usd: None,
            }],
            extra_usage: None,
            error: None,
            queried_at: Some(1),
        }
    }

    fn account(id: &str, utilization: f64) -> SubscriptionAccountHealth {
        SubscriptionAccountHealth {
            account_id: id.to_string(),
            provider: SubscriptionProvider::Codex,
            enabled: true,
            quota: quota(utilization),
        }
    }

    fn route(mode: SubscriptionRouteMode) -> ProjectSubscriptionRoute {
        ProjectSubscriptionRoute {
            project_path: ".".to_string(),
            provider: SubscriptionProvider::Codex,
            account_ids: vec!["primary".to_string(), "fallback".to_string()],
            mode,
            terms_acknowledged: true,
            max_utilization: 90.0,
        }
    }

    #[test]
    fn advisory_mode_suggests_a_healthy_fallback_with_confirmation() {
        let recommendation = recommend_account(
            &route(SubscriptionRouteMode::AdvisoryFailover),
            &[account("primary", 100.0), account("fallback", 25.0)],
        );

        assert_eq!(recommendation.decision, RouteDecision::AdviseFallback);
        assert_eq!(
            recommendation.selected_account_id.as_deref(),
            Some("fallback")
        );
        assert!(recommendation.requires_confirmation);
    }

    #[test]
    fn pinned_mode_never_falls_back_automatically_or_advises_a_switch() {
        let recommendation = recommend_account(
            &route(SubscriptionRouteMode::Pinned),
            &[account("primary", 100.0), account("fallback", 25.0)],
        );

        assert_eq!(recommendation.decision, RouteDecision::Blocked);
        assert!(recommendation.selected_account_id.is_none());
    }

    #[test]
    fn route_requires_terms_acknowledgement() {
        let mut value = route(SubscriptionRouteMode::AdvisoryFailover);
        value.terms_acknowledged = false;
        assert!(normalize_route(&mut value)
            .unwrap_err()
            .contains("acknowledgement"));
    }

    #[test]
    fn route_store_round_trips_without_secrets() {
        let temp = tempfile::tempdir().unwrap();
        let saved =
            upsert_route(temp.path(), route(SubscriptionRouteMode::AdvisoryFailover)).unwrap();
        let loaded = load_routes(temp.path()).unwrap();

        assert_eq!(loaded, vec![saved]);
        let serialized = std::fs::read_to_string(route_store_path(temp.path())).unwrap();
        assert!(!serialized.contains("token"));
    }
}
