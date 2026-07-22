//! Product-account OAuth helpers shared by the desktop app and `os` CLI.
//!
//! The module intentionally only builds and validates the native-client side
//! of the AuthKit authorization-code flow. Token exchange remains in the
//! OpenSunstar control plane, where the WorkOS secret is never exposed to a
//! desktop binary.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

const MIN_ENTROPY_BYTES: usize = 64;
const SESSION_ENTRY_KEY: &str = "product/auth/session-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductAuthConfig {
    pub authorize_url: String,
    pub client_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAttempt {
    pub authorization_url: String,
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCallback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Credentials issued by the OpenSunstar control plane after it validates the
/// WorkOS authorization code. This type must never be sent to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at_unix: i64,
    pub user_id: String,
    pub email: String,
    pub organization_id: Option<String>,
}

/// The only session representation allowed to cross the Tauri command bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProductSessionSummary {
    pub signed_in: bool,
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub organization_id: Option<String>,
    pub expires_at_unix: Option<i64>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProductAuthError {
    #[error("产品登录授权端点必须使用 HTTPS")]
    InsecureAuthorizeUrl,
    #[error("产品登录回调必须使用 http://127.0.0.1")]
    InvalidLoopbackRedirect,
    #[error("产品登录客户端 ID 不能为空")]
    MissingClientId,
    #[error("产品登录随机熵不足")]
    InsufficientEntropy,
    #[error("产品登录回调 state 不匹配")]
    StateMismatch,
    #[error("身份提供方拒绝登录: {0}")]
    ProviderDenied(String),
    #[error("产品登录回调缺少授权码")]
    MissingAuthorizationCode,
    #[error("产品登录授权地址无效: {0}")]
    InvalidAuthorizeUrl(String),
    #[error("产品会话安全存储失败: {0}")]
    SessionStorage(String),
}

/// Build a native-client AuthKit authorization request from caller-provided
/// entropy. Keeping entropy injectable makes the protocol deterministic in
/// tests; production callers should use [`begin_random_login_attempt`].
pub fn begin_login_attempt(
    config: &ProductAuthConfig,
    entropy: &[u8],
) -> Result<LoginAttempt, ProductAuthError> {
    validate_config(config)?;
    if entropy.len() < MIN_ENTROPY_BYTES {
        return Err(ProductAuthError::InsufficientEntropy);
    }

    let verifier = URL_SAFE_NO_PAD.encode(&entropy[..MIN_ENTROPY_BYTES]);
    let state = URL_SAFE_NO_PAD.encode(&entropy[MIN_ENTROPY_BYTES..]);
    if state.is_empty() {
        return Err(ProductAuthError::InsufficientEntropy);
    }
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse(&config.authorize_url)
        .map_err(|error| ProductAuthError::InvalidAuthorizeUrl(error.to_string()))?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("provider", "authkit")
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");

    Ok(LoginAttempt {
        authorization_url: url.into(),
        state,
        code_verifier: verifier,
        redirect_uri: config.redirect_uri.clone(),
    })
}

/// Build a login attempt using cryptographically secure operating-system RNG.
pub fn begin_random_login_attempt(
    config: &ProductAuthConfig,
) -> Result<LoginAttempt, ProductAuthError> {
    let mut entropy = [0_u8; 96];
    rand::thread_rng().fill_bytes(&mut entropy);
    begin_login_attempt(config, &entropy)
}

impl LoginAttempt {
    /// Validate a loopback callback before the authorization code is submitted
    /// to the control plane for exchange.
    pub fn complete_callback(&self, callback: LoginCallback) -> Result<String, ProductAuthError> {
        if callback.state.as_deref() != Some(self.state.as_str()) {
            return Err(ProductAuthError::StateMismatch);
        }
        if let Some(error) = callback.error {
            let description = callback
                .error_description
                .filter(|value| !value.trim().is_empty())
                .map(|value| format!("{error}: {value}"))
                .unwrap_or(error);
            return Err(ProductAuthError::ProviderDenied(description));
        }
        callback
            .code
            .filter(|value| !value.trim().is_empty())
            .ok_or(ProductAuthError::MissingAuthorizationCode)
    }
}

/// Persists the full session in the OS Keychain only. The database and project
/// configuration must hold neither tokens nor a Keychain reference for it.
pub fn store_session(session: &ProductSession) -> Result<(), ProductAuthError> {
    if session.access_token.trim().is_empty() || session.refresh_token.trim().is_empty() {
        return Err(ProductAuthError::SessionStorage(
            "access and refresh tokens must both be present".to_string(),
        ));
    }

    let serialized = serde_json::to_string(session)
        .map_err(|error| ProductAuthError::SessionStorage(error.to_string()))?;
    crate::keychain::store_secret(SESSION_ENTRY_KEY, &serialized)
        .map_err(|error| ProductAuthError::SessionStorage(error.to_string()))
}

pub fn load_session() -> Result<Option<ProductSession>, ProductAuthError> {
    let Some(serialized) = crate::keychain::get_secret(SESSION_ENTRY_KEY)
        .map_err(|error| ProductAuthError::SessionStorage(error.to_string()))?
    else {
        return Ok(None);
    };

    let session = serde_json::from_str(&serialized)
        .map_err(|error| ProductAuthError::SessionStorage(error.to_string()))?;
    Ok(Some(session))
}

pub fn clear_session() -> Result<(), ProductAuthError> {
    crate::keychain::delete_secret(SESSION_ENTRY_KEY)
        .map_err(|error| ProductAuthError::SessionStorage(error.to_string()))
}

pub fn session_summary(session: Option<&ProductSession>) -> ProductSessionSummary {
    match session {
        Some(session) => ProductSessionSummary {
            signed_in: true,
            user_id: Some(session.user_id.clone()),
            email: Some(session.email.clone()),
            organization_id: session.organization_id.clone(),
            expires_at_unix: Some(session.expires_at_unix),
        },
        None => ProductSessionSummary {
            signed_in: false,
            user_id: None,
            email: None,
            organization_id: None,
            expires_at_unix: None,
        },
    }
}

fn validate_config(config: &ProductAuthConfig) -> Result<(), ProductAuthError> {
    if config.client_id.trim().is_empty() {
        return Err(ProductAuthError::MissingClientId);
    }
    let authorize_url = Url::parse(&config.authorize_url)
        .map_err(|error| ProductAuthError::InvalidAuthorizeUrl(error.to_string()))?;
    if authorize_url.scheme() != "https" {
        return Err(ProductAuthError::InsecureAuthorizeUrl);
    }
    let redirect_uri =
        Url::parse(&config.redirect_uri).map_err(|_| ProductAuthError::InvalidLoopbackRedirect)?;
    if redirect_uri.scheme() != "http"
        || redirect_uri.host_str() != Some("127.0.0.1")
        || redirect_uri.port().is_none()
        || redirect_uri.path() != "/callback"
    {
        return Err(ProductAuthError::InvalidLoopbackRedirect);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{clear_session, load_session, session_summary, store_session, ProductSession};

    #[test]
    fn product_session_is_persisted_in_keychain_but_summary_has_no_tokens() {
        clear_session().expect("clear previous session");
        let session = ProductSession {
            access_token: "access-secret".to_string(),
            refresh_token: "refresh-secret".to_string(),
            expires_at_unix: 1_800_000_000,
            user_id: "user_123".to_string(),
            email: "developer@example.test".to_string(),
            organization_id: Some("org_123".to_string()),
        };

        store_session(&session).expect("store session in keychain");
        assert_eq!(load_session().expect("read session"), Some(session));

        let loaded = load_session().expect("read session");
        let serialized_summary =
            serde_json::to_string(&session_summary(loaded.as_ref())).expect("serialize summary");
        assert!(!serialized_summary.contains("access-secret"));
        assert!(!serialized_summary.contains("refresh-secret"));
        clear_session().expect("clear session");
    }
}
