//! 订阅账号的统一观测与项目绑定命令。
//!
//! 此处刻意不实现静默换号：账号建议只供用户确认后修改项目绑定。凭据保留在
//! 各 OAuth manager 或本机 CLI 中，路由文件仅保存引用 ID 和条款确认记录。

use futures::future::join_all;
use serde::Serialize;
use tauri::State;

use crate::commands::codex_oauth::CodexOAuthState;
use crate::services::subscription::{self, CredentialStatus, SubscriptionQuota};
use crate::services::subscription_routing::{
    self, load_routes, recommend_account, upsert_route, ProjectSubscriptionRoute,
    SubscriptionAccountHealth, SubscriptionProvider,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionAccountView {
    pub id: String,
    pub provider: SubscriptionProvider,
    pub display_name: String,
    pub source: String,
    pub is_default: bool,
    pub quota: SubscriptionQuota,
    pub health: String,
}

fn quota_health(quota: &SubscriptionQuota) -> String {
    if !quota.success || !matches!(quota.credential_status, CredentialStatus::Valid) {
        return "unavailable".to_string();
    }
    if quota.tiers.is_empty() {
        return "unknown".to_string();
    }
    if quota.tiers.iter().any(|tier| tier.utilization >= 90.0) {
        return "constrained".to_string();
    }
    "healthy".to_string()
}

async fn collect_subscription_accounts(
    codex_state: &CodexOAuthState,
) -> Vec<SubscriptionAccountView> {
    let (codex_accounts, default_account_id) = {
        let manager = codex_state.0.read().await;
        (manager.list_accounts().await, manager.default_account_id().await)
    };

    // Claude/Gemini use their existing CLI credentials as an observable current
    // session, not as OpenSunstar-managed multi-account identities. Every managed
    // Codex OAuth identity is queried independently; all requests are started together.
    let codex_queries = codex_accounts.into_iter().map(|account| {
        let account_id = account.id;
        let display_name = account.login;
        let is_default = default_account_id.as_deref() == Some(account_id.as_str());
        async move {
            let manager = codex_state.0.read().await;
            let quota =
                match manager.get_valid_token_for_account(&account_id).await {
                    Ok(token) => subscription::query_codex_quota(
                        &token,
                        Some(&account_id),
                        "codex_oauth",
                        "Codex OAuth token expired or rejected. Please re-login via OpenSunstar.",
                    )
                    .await,
                    Err(error) => SubscriptionQuota::error(
                        "codex_oauth",
                        CredentialStatus::Expired,
                        format!("Codex OAuth token unavailable: {error}"),
                    ),
                };
            SubscriptionAccountView {
                id: account_id,
                provider: SubscriptionProvider::Codex,
                display_name,
                source: "managed_oauth".to_string(),
                is_default,
                health: quota_health(&quota),
                quota,
            }
        }
    });

    let claude_query = async {
        let quota = subscription::get_subscription_quota("claude")
            .await
            .unwrap_or_else(|error| {
                SubscriptionQuota::error("claude", CredentialStatus::Valid, error)
            });
        SubscriptionAccountView {
            id: "claude:local-cli".to_string(),
            provider: SubscriptionProvider::Claude,
            display_name: "Claude CLI (local profile)".to_string(),
            source: "local_cli".to_string(),
            is_default: false,
            health: quota_health(&quota),
            quota,
        }
    };
    let gemini_query = async {
        let quota = subscription::get_subscription_quota("gemini")
            .await
            .unwrap_or_else(|error| {
                SubscriptionQuota::error("gemini", CredentialStatus::Valid, error)
            });
        SubscriptionAccountView {
            id: "gemini:local-cli".to_string(),
            provider: SubscriptionProvider::Gemini,
            display_name: "Gemini CLI (local profile)".to_string(),
            source: "local_cli".to_string(),
            is_default: false,
            health: quota_health(&quota),
            quota,
        }
    };

    let (mut codex, claude, gemini) =
        tokio::join!(join_all(codex_queries), claude_query, gemini_query);
    codex.push(claude);
    codex.push(gemini);
    codex
}

#[tauri::command(rename_all = "camelCase")]
pub async fn subscription_list_accounts(
    codex_state: State<'_, CodexOAuthState>,
) -> Result<Vec<SubscriptionAccountView>, String> {
    Ok(collect_subscription_accounts(codex_state.inner()).await)
}

#[tauri::command(rename_all = "camelCase")]
pub fn subscription_list_project_routes() -> Result<Vec<ProjectSubscriptionRoute>, String> {
    load_routes(&crate::config::get_app_config_dir())
}

#[tauri::command(rename_all = "camelCase")]
pub fn subscription_save_project_route(
    route: ProjectSubscriptionRoute,
) -> Result<ProjectSubscriptionRoute, String> {
    upsert_route(&crate::config::get_app_config_dir(), route)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn subscription_recommend_project_account(
    project_path: String,
    provider: SubscriptionProvider,
    codex_state: State<'_, CodexOAuthState>,
) -> Result<subscription_routing::RouteRecommendation, String> {
    let routes = load_routes(&crate::config::get_app_config_dir())?;
    let mut requested = ProjectSubscriptionRoute {
        project_path,
        provider,
        account_ids: vec!["placeholder".to_string()],
        mode: Default::default(),
        terms_acknowledged: true,
        max_utilization: subscription_routing::DEFAULT_MAX_UTILIZATION,
    };
    subscription_routing::normalize_route(&mut requested)?;
    let route = routes
        .into_iter()
        .find(|candidate| {
            candidate.project_path == requested.project_path
                && candidate.provider == requested.provider
        })
        .ok_or_else(|| {
            "No subscription route is configured for this project and provider".to_string()
        })?;
    let accounts = collect_subscription_accounts(codex_state.inner()).await;
    let health = accounts
        .into_iter()
        .map(|account| SubscriptionAccountHealth {
            account_id: account.id,
            provider: account.provider,
            enabled: true,
            quota: account.quota,
        })
        .collect::<Vec<_>>();
    Ok(recommend_account(&route, &health))
}
