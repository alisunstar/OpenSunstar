//! IPC surface for the OpenSunstar product account.
//!
//! Unlike `commands::auth`, which manages upstream-provider credentials, these
//! commands only expose a token-free product-account session summary.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Notify;

use crate::product_auth::{
    begin_random_login_attempt, clear_session, load_session, session_summary, LoginCallback,
    ProductAuthConfig, ProductSession, ProductSessionSummary,
};

const DEVICE_ENTRY_KEY: &str = "product/auth/device-id-v1";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);
/// Fixed loopback port for the OAuth callback. WorkOS requires the redirect_uri
/// to exactly match a registered Redirect URL, so we cannot use an ephemeral port.
const LOOPBACK_CALLBACK_PORT: u16 = 17173;
static LOGIN_CANCEL: once_cell::sync::Lazy<Notify> = once_cell::sync::Lazy::new(Notify::new);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneAuthConfig {
    authorize_url: String,
    client_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeRequest<'a> {
    code: &'a str,
    code_verifier: &'a str,
    device_id: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeResponse {
    access_token: String,
    refresh_token: String,
    expires_at: String,
    user: ExchangeUser,
}

#[derive(Debug, Deserialize)]
struct ExchangeUser {
    id: String,
    email: String,
}

#[tauri::command]
pub fn product_auth_get_session() -> Result<ProductSessionSummary, String> {
    let session = load_session().map_err(|error| error.to_string())?;
    Ok(session_summary(session.as_ref()))
}

#[tauri::command]
pub async fn product_auth_logout() -> Result<(), String> {
    let remote_result = authenticated_json(reqwest::Method::POST, "/v1/auth/logout", None).await;
    clear_session().map_err(|error| error.to_string())?;
    remote_result.map(|_| ())
}

#[tauri::command]
pub async fn product_auth_login(app: AppHandle) -> Result<ProductSessionSummary, String> {
    let base_url = control_plane_base_url()?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "product_auth_client_failed".to_string())?;
    let auth_config = client
        .get(format!("{base_url}/v1/auth/config"))
        .send()
        .await
        .map_err(|_| "product_auth_control_plane_unavailable".to_string())?
        .error_for_status()
        .map_err(|_| "product_auth_config_failed".to_string())?
        .json::<ControlPlaneAuthConfig>()
        .await
        .map_err(|_| "product_auth_config_invalid".to_string())?;

    let listener = TcpListener::bind(format!("127.0.0.1:{LOOPBACK_CALLBACK_PORT}"))
        .await
        .map_err(|_| "product_auth_loopback_bind_failed".to_string())?;
    let attempt = begin_random_login_attempt(&ProductAuthConfig {
        authorize_url: auth_config.authorize_url,
        client_id: auth_config.client_id,
        redirect_uri: format!("http://127.0.0.1:{LOOPBACK_CALLBACK_PORT}/callback"),
    })
    .map_err(|error| error.to_string())?;

    app.opener()
        .open_url(&attempt.authorization_url, None::<String>)
        .map_err(|_| "product_auth_browser_open_failed".to_string())?;
    let callback = receive_loopback_callback(listener).await?;
    let code = attempt
        .complete_callback(callback)
        .map_err(|error| error.to_string())?;
    let device_id = load_or_create_device_id()?;
    let exchange = client
        .post(format!("{base_url}/v1/auth/exchange"))
        .json(&ExchangeRequest {
            code: &code,
            code_verifier: &attempt.code_verifier,
            device_id: &device_id,
        })
        .send()
        .await
        .map_err(|_| "product_auth_exchange_unavailable".to_string())?
        .error_for_status()
        .map_err(|_| "product_auth_exchange_failed".to_string())?
        .json::<ExchangeResponse>()
        .await
        .map_err(|_| "product_auth_exchange_invalid".to_string())?;
    let expires_at = chrono::DateTime::parse_from_rfc3339(&exchange.expires_at)
        .map_err(|_| "product_auth_expiry_invalid".to_string())?
        .timestamp();
    let session = ProductSession {
        access_token: exchange.access_token,
        refresh_token: exchange.refresh_token,
        expires_at_unix: expires_at,
        user_id: exchange.user.id,
        email: exchange.user.email,
        organization_id: None,
    };
    crate::product_auth::store_session(&session).map_err(|error| error.to_string())?;
    let profile = authenticated_json(reqwest::Method::GET, "/v1/me", None).await?;
    if let Some(organization_id) = profile
        .pointer("/memberships/0/orgId")
        .and_then(serde_json::Value::as_str)
    {
        update_session_organization(organization_id)?;
    }
    let stored = load_session().map_err(|error| error.to_string())?;
    Ok(session_summary(stored.as_ref()))
}

#[tauri::command]
pub fn product_auth_cancel_login() {
    LOGIN_CANCEL.notify_waiters();
}

async fn receive_loopback_callback(listener: TcpListener) -> Result<LoginCallback, String> {
    let accepted = tokio::select! {
        result = tokio::time::timeout(LOGIN_TIMEOUT, listener.accept()) => {
            result
                .map_err(|_| "product_auth_callback_timeout".to_string())?
                .map_err(|_| "product_auth_callback_failed".to_string())?
        }
        () = LOGIN_CANCEL.notified() => {
            return Err("product_auth_login_cancelled".to_string());
        }
    };
    let (mut stream, _) = accepted;
    let mut buffer = vec![0_u8; 8192];
    let length = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
        .await
        .map_err(|_| "product_auth_callback_read_timeout".to_string())?
        .map_err(|_| "product_auth_callback_read_failed".to_string())?;
    let request = std::str::from_utf8(&buffer[..length])
        .map_err(|_| "product_auth_callback_invalid".to_string())?;
    let callback = parse_callback_request(request)?;
    let body = "<!doctype html><meta charset=utf-8><title>OpenSunstar</title>登录完成，可返回 OpenSunstar。";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|_| "product_auth_callback_response_failed".to_string())?;
    Ok(callback)
}

fn parse_callback_request(request: &str) -> Result<LoginCallback, String> {
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "product_auth_callback_invalid".to_string())?;
    let url = url::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "product_auth_callback_invalid".to_string())?;
    if url.path() != "/callback" {
        return Err("product_auth_callback_path_invalid".to_string());
    }
    let value = |key: &str| {
        url.query_pairs()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.into_owned())
    };
    Ok(LoginCallback {
        code: value("code"),
        state: value("state"),
        error: value("error"),
        error_description: value("error_description"),
    })
}

/// Compile-time default control plane URL, injected via build environment:
///   OPENSUNSTAR_CONTROL_PLANE_URL_DEFAULT=https://opensunstar-cp.fly.io cargo build
/// In release builds this is baked into the binary so users need zero configuration.
const CONTROL_PLANE_URL_DEFAULT: Option<&str> =
    option_env!("OPENSUNSTAR_CONTROL_PLANE_URL_DEFAULT");

/// Resolve the control plane base URL with three-tier priority:
///   1. Runtime env var OPENSUNSTAR_CONTROL_PLANE_URL (dev / self-hosted override)
///   2. Compile-time default (release builds)
fn control_plane_base_url() -> Result<String, String> {
    let raw = std::env::var("OPENSUNSTAR_CONTROL_PLANE_URL")
        .ok()
        .or_else(|| CONTROL_PLANE_URL_DEFAULT.map(str::to_string))
        .ok_or_else(|| "product_auth_control_plane_not_configured".to_string())?;
    let url = url::Url::parse(raw.trim())
        .map_err(|_| "product_auth_control_plane_url_invalid".to_string())?;
    let is_loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost"));
    if url.scheme() != "https" && !(url.scheme() == "http" && is_loopback) {
        return Err("product_auth_control_plane_url_insecure".to_string());
    }
    Ok(raw.trim_end_matches('/').to_string())
}

fn load_or_create_device_id() -> Result<String, String> {
    if let Some(device_id) = crate::keychain::get_secret(DEVICE_ENTRY_KEY)
        .map_err(|_| "product_auth_device_storage_failed".to_string())?
    {
        return Ok(device_id);
    }
    let device_id = format!("device_{}", uuid::Uuid::new_v4());
    crate::keychain::store_secret(DEVICE_ENTRY_KEY, &device_id)
        .map_err(|_| "product_auth_device_storage_failed".to_string())?;
    Ok(device_id)
}

#[tauri::command]
pub async fn product_team_create_organization(
    name: String,
    slug: String,
) -> Result<serde_json::Value, String> {
    let result = authenticated_json(
        reqwest::Method::POST,
        "/v1/organizations",
        Some(serde_json::json!({ "name": name, "slug": slug })),
    )
    .await?;
    let organization_id = result
        .pointer("/organization/id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "product_team_organization_response_invalid".to_string())?;
    update_session_organization(organization_id)?;
    Ok(result)
}

#[tauri::command]
pub async fn product_team_accept_invite(raw_token: String) -> Result<serde_json::Value, String> {
    let result = authenticated_json(
        reqwest::Method::POST,
        "/v1/invites/accept",
        Some(serde_json::json!({ "rawToken": raw_token })),
    )
    .await?;
    let organization_id = result
        .pointer("/membership/orgId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "product_team_invite_response_invalid".to_string())?;
    update_session_organization(organization_id)?;
    Ok(result)
}

#[tauri::command]
pub async fn product_team_get_overview(org_id: String) -> Result<serde_json::Value, String> {
    authenticated_json(
        reqwest::Method::GET,
        &format!("/v1/organizations/{}/overview", encode_path(&org_id)?),
        None,
    )
    .await
}

#[tauri::command]
pub async fn product_team_list_members(org_id: String) -> Result<serde_json::Value, String> {
    authenticated_json(
        reqwest::Method::GET,
        &format!("/v1/organizations/{}/members", encode_path(&org_id)?),
        None,
    )
    .await
}

#[tauri::command]
pub async fn product_team_list_invites(org_id: String) -> Result<serde_json::Value, String> {
    authenticated_json(
        reqwest::Method::GET,
        &format!("/v1/organizations/{}/invites", encode_path(&org_id)?),
        None,
    )
    .await
}

#[tauri::command]
pub async fn product_team_invite_member(
    org_id: String,
    email: String,
    role: String,
) -> Result<serde_json::Value, String> {
    authenticated_json(
        reqwest::Method::POST,
        &format!("/v1/organizations/{}/invites", encode_path(&org_id)?),
        Some(serde_json::json!({ "email": email, "role": role })),
    )
    .await
}

#[tauri::command]
pub async fn product_team_remove_member(org_id: String, user_id: String) -> Result<(), String> {
    authenticated_json(
        reqwest::Method::DELETE,
        &format!(
            "/v1/organizations/{}/members/{}",
            encode_path(&org_id)?,
            encode_path(&user_id)?
        ),
        None,
    )
    .await
    .map(|_| ())
}

async fn authenticated_json(
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let base_url = control_plane_base_url()?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "product_auth_client_failed".to_string())?;
    let mut session = load_session()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "product_auth_session_required".to_string())?;
    let mut response = send_authenticated(
        &client,
        method.clone(),
        &format!("{base_url}{path}"),
        body.as_ref(),
        &session.access_token,
    )
    .await?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        session = refresh_session(&client, &base_url, session).await?;
        response = send_authenticated(
            &client,
            method,
            &format!("{base_url}{path}"),
            body.as_ref(),
            &session.access_token,
        )
        .await?;
    }
    if !response.status().is_success() {
        return Err(format!(
            "product_team_request_failed_{}",
            response.status().as_u16()
        ));
    }
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(serde_json::Value::Null);
    }
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "product_team_response_invalid".to_string())
}

async fn send_authenticated(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    body: Option<&serde_json::Value>,
    access_token: &str,
) -> Result<reqwest::Response, String> {
    let mut request = client.request(method, url).bearer_auth(access_token);
    if let Some(body) = body {
        request = request.json(body);
    }
    request
        .send()
        .await
        .map_err(|_| "product_team_control_plane_unavailable".to_string())
}

async fn refresh_session(
    client: &reqwest::Client,
    base_url: &str,
    previous: ProductSession,
) -> Result<ProductSession, String> {
    let device_id = load_or_create_device_id()?;
    let exchange = client
        .post(format!("{base_url}/v1/auth/refresh"))
        .json(&serde_json::json!({
            "refreshToken": previous.refresh_token,
            "deviceId": device_id,
        }))
        .send()
        .await
        .map_err(|_| "product_auth_refresh_unavailable".to_string())?
        .error_for_status()
        .map_err(|_| "product_auth_refresh_failed".to_string())?
        .json::<ExchangeResponse>()
        .await
        .map_err(|_| "product_auth_refresh_invalid".to_string())?;
    let expires_at_unix = chrono::DateTime::parse_from_rfc3339(&exchange.expires_at)
        .map_err(|_| "product_auth_expiry_invalid".to_string())?
        .timestamp();
    let session = ProductSession {
        access_token: exchange.access_token,
        refresh_token: exchange.refresh_token,
        expires_at_unix,
        user_id: exchange.user.id,
        email: exchange.user.email,
        organization_id: previous.organization_id,
    };
    crate::product_auth::store_session(&session).map_err(|error| error.to_string())?;
    Ok(session)
}

fn update_session_organization(organization_id: &str) -> Result<(), String> {
    let mut session = load_session()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "product_auth_session_required".to_string())?;
    session.organization_id = Some(organization_id.to_string());
    crate::product_auth::store_session(&session).map_err(|error| error.to_string())
}

fn encode_path(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err("product_team_identifier_invalid".to_string());
    }
    Ok(value.to_string())
}

// ─── D16 Team Key commands ─────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrantEntry {
    grant_id: String,
    slot_slug: String,
    provider_kind: String,
    endpoint_url: Option<String>,
    version_seq: i64,
    value_sha256: String,
    plaintext: String,
    expires_at: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrantsResponse {
    grants: Vec<GrantEntry>,
}

/// Sync team keys from the control plane: fetch grants, store keys in Keychain,
/// update local SQLite references, and ack receipt.
#[tauri::command]
pub async fn team_key_sync(
    state: tauri::State<'_, crate::store::AppState>,
    org_id: String,
) -> Result<serde_json::Value, String> {
    let device_id = load_or_create_device_id()?;
    let path = format!(
        "/v1/organizations/{}/keys/grants?device_id={}",
        encode_path(&org_id)?,
        device_id
    );
    let response = authenticated_json(reqwest::Method::GET, &path, None).await?;
    let grants_response: GrantsResponse = serde_json::from_value(response)
        .map_err(|_| "team_key_sync_response_invalid".to_string())?;

    let db = &state.db;
    let mut synced = 0u32;
    let mut grant_ids_to_ack: Vec<String> = Vec::new();

    for grant in &grants_response.grants {
        let grant_expires = chrono::DateTime::parse_from_rfc3339(&grant.expires_at)
            .map_err(|_| "team_key_grant_expiry_invalid".to_string())?
            .timestamp_millis();

        crate::team_key::store_team_key(
            db,
            &org_id,
            &grant.slot_slug,
            &grant.provider_kind,
            grant.endpoint_url.as_deref(),
            &grant.plaintext,
            grant.version_seq,
            &grant.value_sha256,
            &grant.grant_id,
            grant_expires,
        )
        .map_err(|e| format!("team_key_store_failed: {e}"))?;

        grant_ids_to_ack.push(grant.grant_id.clone());
        synced += 1;
    }

    // Ack receipt to control plane
    if !grant_ids_to_ack.is_empty() {
        let ack_path = format!(
            "/v1/organizations/{}/keys/grants/ack",
            encode_path(&org_id)?
        );
        let _ = authenticated_json(
            reqwest::Method::POST,
            &ack_path,
            Some(serde_json::json!({ "grantIds": grant_ids_to_ack })),
        )
        .await;
    }

    Ok(serde_json::json!({ "synced": synced }))
}

/// Renew team key grants: extend TTL and pick up rotated keys.
#[tauri::command]
pub async fn team_key_renew(
    state: tauri::State<'_, crate::store::AppState>,
    org_id: String,
) -> Result<serde_json::Value, String> {
    let device_id = load_or_create_device_id()?;
    let path = format!(
        "/v1/organizations/{}/keys/grants/renew",
        encode_path(&org_id)?
    );
    let response = authenticated_json(
        reqwest::Method::POST,
        &path,
        Some(serde_json::json!({ "deviceId": device_id })),
    )
    .await?;

    // Process rotated keys (plaintext only present when rotated=true)
    let db = &state.db;
    let mut rotated_count = 0u32;

    if let Some(grants) = response.get("grants").and_then(|g| g.as_array()) {
        for grant in grants {
            let is_rotated = grant.get("rotated").and_then(|r| r.as_bool()).unwrap_or(false);
            if !is_rotated {
                continue;
            }
            let slot_slug = grant.get("slotSlug").and_then(|s| s.as_str()).unwrap_or_default();
            let plaintext = grant.get("plaintext").and_then(|p| p.as_str()).unwrap_or_default();
            let version_seq = grant.get("versionSeq").and_then(|v| v.as_i64()).unwrap_or(0);
            let value_sha256 = grant.get("valueSha256").and_then(|v| v.as_str()).unwrap_or_default();
            let grant_id = grant.get("grantId").and_then(|g| g.as_str()).unwrap_or_default();
            let expires_at = grant.get("expiresAt").and_then(|e| e.as_str()).unwrap_or_default();

            if slot_slug.is_empty() || plaintext.is_empty() {
                continue;
            }

            let grant_expires = chrono::DateTime::parse_from_rfc3339(expires_at)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);

            // Look up existing key for provider_kind/endpoint_url
            let existing = db.get_team_key(slot_slug).map_err(|e| e.to_string())?;
            let provider_kind = existing.as_ref().map(|k| k.provider_kind.as_str()).unwrap_or("custom");
            let endpoint_url = existing.as_ref().and_then(|k| k.endpoint_url.as_deref());

            crate::team_key::store_team_key(
                db,
                &org_id,
                slot_slug,
                provider_kind,
                endpoint_url,
                plaintext,
                version_seq,
                value_sha256,
                grant_id,
                grant_expires,
            )
            .map_err(|e| format!("team_key_renew_store_failed: {e}"))?;

            rotated_count += 1;
        }
    }

    Ok(serde_json::json!({ "rotated": rotated_count, "detail": response }))
}

/// List all local team keys (metadata only, no plaintext).
#[tauri::command]
pub fn team_key_list(
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<Vec<crate::database::TeamKeyLocal>, String> {
    state
        .db
        .list_all_team_keys()
        .map_err(|e| e.to_string())
}

/// Get status of a single team key slot.
#[tauri::command]
pub fn team_key_status(
    state: tauri::State<'_, crate::store::AppState>,
    slot_slug: String,
) -> Result<Option<crate::database::TeamKeyLocal>, String> {
    state
        .db
        .get_team_key(&slot_slug)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_callback_request;

    #[test]
    fn callback_parser_accepts_only_the_loopback_callback_path() {
        let parsed = parse_callback_request(
            "GET /callback?code=auth_code&state=expected HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        )
        .expect("valid callback");
        assert_eq!(parsed.code.as_deref(), Some("auth_code"));
        assert_eq!(parsed.state.as_deref(), Some("expected"));
        assert!(parse_callback_request("GET /other?code=x HTTP/1.1\r\n\r\n").is_err());
    }
}
