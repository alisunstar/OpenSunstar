use open_sunstar_lib::product_auth::{begin_login_attempt, LoginCallback, ProductAuthConfig};

fn config() -> ProductAuthConfig {
    ProductAuthConfig {
        authorize_url: "https://api.workos.com/user_management/authorize".to_string(),
        client_id: "client_test".to_string(),
        redirect_uri: "http://127.0.0.1:48123/callback".to_string(),
    }
}

#[test]
fn login_attempt_uses_pkce_and_never_puts_the_verifier_in_the_browser_url() {
    let attempt = begin_login_attempt(&config(), &[7_u8; 96]).expect("build login attempt");

    let url = url::Url::parse(&attempt.authorization_url).expect("authorization URL");
    let query = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        query.get("redirect_uri").map(|value| value.as_ref()),
        Some("http://127.0.0.1:48123/callback")
    );
    assert_eq!(
        query.get("provider").map(|value| value.as_ref()),
        Some("authkit")
    );
    assert_eq!(
        query
            .get("code_challenge_method")
            .map(|value| value.as_ref()),
        Some("S256")
    );
    assert!(query.contains_key("code_challenge"));
    assert!(!attempt.authorization_url.contains(&attempt.code_verifier));
    assert!(attempt.code_verifier.len() >= 43);
}

#[test]
fn callback_requires_the_exact_state_before_accepting_an_authorization_code() {
    let attempt = begin_login_attempt(&config(), &[9_u8; 96]).expect("build login attempt");

    let callback = LoginCallback {
        code: Some("auth_code_123".to_string()),
        state: Some(attempt.state.clone()),
        error: None,
        error_description: None,
    };
    assert_eq!(
        attempt
            .complete_callback(callback)
            .expect("callback accepted"),
        "auth_code_123"
    );

    let invalid_state = LoginCallback {
        code: Some("auth_code_123".to_string()),
        state: Some("other-state".to_string()),
        error: None,
        error_description: None,
    };
    assert!(attempt.complete_callback(invalid_state).is_err());
}

#[test]
fn callback_surfaces_provider_denial_without_treating_it_as_a_success() {
    let attempt = begin_login_attempt(&config(), &[11_u8; 96]).expect("build login attempt");
    let denied = LoginCallback {
        code: None,
        state: Some(attempt.state.clone()),
        error: Some("access_denied".to_string()),
        error_description: Some("user cancelled".to_string()),
    };

    let error = attempt
        .complete_callback(denied)
        .expect_err("denial must fail");
    assert!(error.to_string().contains("access_denied"));
}

#[test]
fn login_rejects_insecure_or_non_loopback_configuration() {
    let mut insecure_authorize_url = config();
    insecure_authorize_url.authorize_url = "http://api.workos.com/authorize".to_string();
    assert!(begin_login_attempt(&insecure_authorize_url, &[1_u8; 96]).is_err());

    let mut non_loopback_redirect = config();
    non_loopback_redirect.redirect_uri = "http://localhost:48123/callback".to_string();
    assert!(begin_login_attempt(&non_loopback_redirect, &[1_u8; 96]).is_err());
}

#[test]
fn login_requires_sufficient_entropy_and_an_authorization_code() {
    assert!(begin_login_attempt(&config(), &[2_u8; 63]).is_err());

    let attempt = begin_login_attempt(&config(), &[3_u8; 96]).expect("build login attempt");
    let no_code = LoginCallback {
        code: None,
        state: Some(attempt.state.clone()),
        error: None,
        error_description: None,
    };
    assert!(attempt.complete_callback(no_code).is_err());
}
