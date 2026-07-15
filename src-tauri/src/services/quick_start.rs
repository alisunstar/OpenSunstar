use crate::app_config::AppType;
use crate::database::{QuickStartOperation, QuickStartOperationStatus};
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::provider::{verify_key, VerifyProtocol};
use crate::services::ProviderService;
use crate::store::AppState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum QuickStartLiveSnapshot {
    Provider(crate::services::provider::LiveSnapshot),
    ClaudeDesktop(crate::claude_desktop_config::ClaudeDesktopConfigSnapshot),
}

/// Persisted audit evidence for the upstream credential probe. It deliberately
/// excludes the API key, the full endpoint URL and any upstream response body.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuickStartUpstreamVerificationReceipt {
    provider_fingerprint: String,
    protocol: String,
    endpoint_host: String,
    model_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickStartApplyRequest {
    pub idempotency_key: String,
    pub app_type: String,
    pub provider: Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickStartFaultPoint {
    AfterProviderCreatedBeforeReceipt,
    AfterProviderCreated,
    AfterProviderSwitchedBeforeReceipt,
    AfterProviderSwitched,
    AfterTakeoverEnabledBeforeReceipt,
    AfterTakeoverEnabled,
    AfterProxyStartedBeforeReceipt,
    AfterProxyStarted,
}

pub struct QuickStartService;

impl QuickStartService {
    pub async fn apply(
        state: &AppState,
        request: QuickStartApplyRequest,
    ) -> Result<QuickStartOperation, AppError> {
        Self::apply_inner(state, request, None).await
    }

    pub async fn rollback(
        state: &AppState,
        operation_id: &str,
        expected_revision: i64,
    ) -> Result<QuickStartOperation, AppError> {
        let mut operation = state
            .db
            .get_quick_start_operation(operation_id)?
            .ok_or_else(|| {
                AppError::InvalidInput(format!("QuickStart operation not found: {operation_id}"))
            })?;
        if operation.revision != expected_revision {
            return Err(AppError::Message(format!(
                "QUICKSTART_REVISION_CONFLICT: expected {expected_revision}, actual {}",
                operation.revision
            )));
        }
        if operation.status == QuickStartOperationStatus::RolledBack {
            return Ok(operation);
        }
        let app_type = AppType::from_str(&operation.app_type)?;
        if operation.status == QuickStartOperationStatus::Succeeded {
            ensure_operation_still_owns_current_provider(state, &app_type, &operation)?;
        }
        if matches!(
            operation.status,
            QuickStartOperationStatus::Pending
                | QuickStartOperationStatus::Succeeded
                | QuickStartOperationStatus::Failed
                | QuickStartOperationStatus::RollbackFailed
        ) {
            operation = state.db.transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::RollingBack,
                "manual_rollback_started",
                None,
                None,
            )?;
        }
        if !matches!(
            operation.status,
            QuickStartOperationStatus::Applying
                | QuickStartOperationStatus::Verifying
                | QuickStartOperationStatus::RollingBack
        ) {
            return Err(AppError::InvalidInput(format!(
                "QuickStart operation cannot be rolled back from status {}",
                operation.status
            )));
        }
        compensate_failed_apply(state, &app_type, &operation, "User requested rollback").await
    }

    async fn apply_inner(
        state: &AppState,
        request: QuickStartApplyRequest,
        fault_at: Option<QuickStartFaultPoint>,
    ) -> Result<QuickStartOperation, AppError> {
        let app_type = AppType::from_str(&request.app_type)?;
        if !matches!(
            app_type,
            AppType::Claude | AppType::ClaudeDesktop | AppType::Codex | AppType::Gemini
        ) {
            return Err(AppError::InvalidInput(format!(
                "Unsupported QuickStart app type: {}",
                request.app_type
            )));
        }
        if request.provider.id.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "QuickStart provider id cannot be empty".to_string(),
            ));
        }

        let fingerprint = request_fingerprint(&request)?;
        let begun = state.db.begin_quick_start_operation(
            &request.idempotency_key,
            &fingerprint,
            app_type.as_str(),
        )?;
        if !begun.created {
            return Ok(begun.operation);
        }

        let mut operation = state.db.transition_quick_start_operation(
            &begun.operation.id,
            begun.operation.revision,
            QuickStartOperationStatus::Applying,
            "preflight",
            None,
            None,
        )?;

        match state
            .db
            .get_provider_by_id(&request.provider.id, app_type.as_str())
        {
            Ok(None) => {}
            Ok(Some(_)) => {
                return compensate_failed_apply(
                    state,
                    &app_type,
                    &operation,
                    "QuickStart provider id already exists",
                )
                .await;
            }
            Err(error) => {
                return compensate_failed_apply(
                    state,
                    &app_type,
                    &operation,
                    &redact_error(&error.to_string(), &request.provider),
                )
                .await;
            }
        }

        let previous_provider = match ProviderService::current(state, app_type.clone()) {
            Ok(provider) => provider,
            Err(error) => {
                return compensate_failed_apply(
                    state,
                    &app_type,
                    &operation,
                    &redact_error(&error.to_string(), &request.provider),
                )
                .await;
            }
        };
        let previous_provider = (!previous_provider.is_empty()).then_some(previous_provider);
        let proxy_was_running = state.proxy_service.is_running().await;
        let takeover_was_enabled = if supports_proxy_takeover(&app_type) {
            match state.proxy_service.get_takeover_status().await {
                Ok(status) => takeover_enabled_for_app(&status, app_type.as_str()),
                Err(error) => {
                    return compensate_failed_apply(
                        state,
                        &app_type,
                        &operation,
                        &redact_error(&error, &request.provider),
                    )
                    .await;
                }
            }
        } else {
            false
        };
        operation = match state.db.record_quick_start_progress(
            &operation.id,
            operation.revision,
            "preflight_captured",
            Some(&request.provider.id),
            previous_provider.as_deref(),
            None,
            Some(takeover_was_enabled),
            Some(proxy_was_running),
        ) {
            Ok(operation) => operation,
            Err(error) => {
                return compensate_failed_apply(
                    state,
                    &app_type,
                    &operation,
                    &redact_error(&error.to_string(), &request.provider),
                )
                .await;
            }
        };

        let sealed_snapshot = match capture_live_snapshot(&app_type) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return compensate_failed_apply(
                    state,
                    &app_type,
                    &operation,
                    &redact_error(&error.to_string(), &request.provider),
                )
                .await;
            }
        };
        operation = match state.db.record_quick_start_live_snapshot(
            &operation.id,
            operation.revision,
            &sealed_snapshot,
        ) {
            Ok(operation) => operation,
            Err(error) => {
                return compensate_failed_apply(
                    state,
                    &app_type,
                    &operation,
                    &redact_error(&error.to_string(), &request.provider),
                )
                .await;
            }
        };

        let apply_result: Result<QuickStartOperation, AppError> = async {
            operation = state.db.record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_creation_started",
                None,
                None,
                None,
                None,
                None,
            )?;
            ProviderService::add(state, app_type.clone(), request.provider.clone(), true)?;
            fail_if_requested(
                fault_at,
                QuickStartFaultPoint::AfterProviderCreatedBeforeReceipt,
            )?;
            operation = state.db.record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_created",
                None,
                None,
                Some("provider_created"),
                None,
                None,
            )?;
            fail_if_requested(fault_at, QuickStartFaultPoint::AfterProviderCreated)?;

            operation = state.db.record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_switch_started",
                None,
                None,
                None,
                None,
                None,
            )?;
            ProviderService::switch(state, app_type.clone(), &request.provider.id)?;
            fail_if_requested(
                fault_at,
                QuickStartFaultPoint::AfterProviderSwitchedBeforeReceipt,
            )?;
            operation = state.db.record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_switched",
                None,
                None,
                Some("provider_switched"),
                None,
                None,
            )?;
            fail_if_requested(fault_at, QuickStartFaultPoint::AfterProviderSwitched)?;

            if supports_proxy_takeover(&app_type) && !takeover_was_enabled {
                operation = state.db.record_quick_start_progress(
                    &operation.id,
                    operation.revision,
                    "takeover_enable_started",
                    None,
                    None,
                    None,
                    None,
                    None,
                )?;
                state
                    .proxy_service
                    .set_takeover_for_app(app_type.as_str(), true)
                    .await
                    .map_err(AppError::Message)?;
                fail_if_requested(
                    fault_at,
                    QuickStartFaultPoint::AfterTakeoverEnabledBeforeReceipt,
                )?;
                operation = state.db.record_quick_start_progress(
                    &operation.id,
                    operation.revision,
                    "takeover_enabled",
                    None,
                    None,
                    Some("takeover_enabled"),
                    None,
                    None,
                )?;
                fail_if_requested(fault_at, QuickStartFaultPoint::AfterTakeoverEnabled)?;
            }

            if supports_proxy_takeover(&app_type) && !proxy_was_running {
                operation = state.db.record_quick_start_progress(
                    &operation.id,
                    operation.revision,
                    "proxy_start_started",
                    None,
                    None,
                    None,
                    None,
                    None,
                )?;
                state
                    .proxy_service
                    .start()
                    .await
                    .map_err(AppError::Message)?;
                fail_if_requested(
                    fault_at,
                    QuickStartFaultPoint::AfterProxyStartedBeforeReceipt,
                )?;
                operation = state.db.record_quick_start_progress(
                    &operation.id,
                    operation.revision,
                    "proxy_started",
                    None,
                    None,
                    Some("proxy_started"),
                    None,
                    None,
                )?;
                fail_if_requested(fault_at, QuickStartFaultPoint::AfterProxyStarted)?;
            }

            operation = state.db.transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Verifying,
                "post_apply_verification",
                None,
                None,
            )?;
            let applied_provider =
                verify_applied_state(state, &app_type, &request.provider.id).await?;
            let receipt = verify_upstream_provider(&applied_provider, &app_type).await?;
            operation = state.db.record_quick_start_upstream_verification(
                &operation.id,
                operation.revision,
                &receipt,
            )?;
            operation = state.db.record_quick_start_progress(
                &operation.id,
                operation.revision,
                "post_apply_verified",
                None,
                None,
                Some("post_verified"),
                None,
                None,
            )?;
            let applied_live_fingerprint = capture_live_fingerprint(&app_type)?;
            operation = state.db.record_quick_start_applied_live_fingerprint(
                &operation.id,
                operation.revision,
                &applied_live_fingerprint,
            )?;
            state.db.transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Succeeded,
                "completed",
                None,
                None,
            )
        }
        .await;

        match apply_result {
            Ok(completed) => Ok(completed),
            Err(error) => {
                let safe_message = redact_error(&error.to_string(), &request.provider);
                compensate_failed_apply(state, &app_type, &operation, &safe_message).await
            }
        }
    }

    #[cfg(test)]
    pub async fn apply_with_fault(
        state: &AppState,
        request: QuickStartApplyRequest,
        fault_at: QuickStartFaultPoint,
    ) -> Result<QuickStartOperation, AppError> {
        Self::apply_inner(state, request, Some(fault_at)).await
    }
}

fn ensure_operation_still_owns_current_provider(
    state: &AppState,
    app_type: &AppType,
    operation: &QuickStartOperation,
) -> Result<(), AppError> {
    let expected_provider_id = operation.provider_id.as_deref().ok_or_else(|| {
        AppError::Message(
            "QUICKSTART_ROLLBACK_CONFLICT: operation has no applied provider".to_string(),
        )
    })?;
    let current_provider_id = ProviderService::current(state, app_type.clone())?;
    if current_provider_id != expected_provider_id {
        return Err(AppError::Message(format!(
            "QUICKSTART_ROLLBACK_CONFLICT: current provider is '{current_provider_id}', not operation provider '{expected_provider_id}'"
        )));
    }
    let expected_fingerprint = operation
        .applied_live_fingerprint
        .as_deref()
        .ok_or_else(|| {
            AppError::Message(
                "QUICKSTART_ROLLBACK_CONFLICT: operation predates rollback ownership guards"
                    .to_string(),
            )
        })?;
    let current_fingerprint = capture_live_fingerprint(app_type)?;
    if current_fingerprint != expected_fingerprint {
        return Err(AppError::Message(
            "QUICKSTART_ROLLBACK_CONFLICT: live configuration changed after this operation"
                .to_string(),
        ));
    }
    Ok(())
}

async fn compensate_failed_apply(
    state: &AppState,
    app_type: &AppType,
    operation: &QuickStartOperation,
    safe_message: &str,
) -> Result<QuickStartOperation, AppError> {
    let mut operation = operation.clone();
    // The durable `*_started` event is a write-ahead intent record. Preserve
    // its meaning before changing current_step to compensation_started: a
    // process may stop after the external effect succeeds but before its
    // completion receipt is persisted.
    let provider_creation_may_have_completed =
        operation.provider_created || operation.current_step == "provider_creation_started";
    let provider_switch_may_have_completed =
        operation.provider_switched || operation.current_step == "provider_switch_started";
    let takeover_enable_may_have_completed =
        operation.takeover_enabled || operation.current_step == "takeover_enable_started";
    let proxy_start_may_have_completed =
        operation.proxy_started || operation.current_step == "proxy_start_started";
    if matches!(
        operation.status,
        QuickStartOperationStatus::Applying | QuickStartOperationStatus::Verifying
    ) {
        operation = state.db.transition_quick_start_operation(
            &operation.id,
            operation.revision,
            QuickStartOperationStatus::RollingBack,
            "compensation_started",
            Some("QUICKSTART_APPLY_FAILED"),
            Some(safe_message),
        )?;
    }

    let compensation: Result<(), AppError> = async {
        if takeover_enable_may_have_completed {
            state
                .proxy_service
                .set_takeover_for_app(app_type.as_str(), false)
                .await
                .map_err(AppError::Message)?;
        }
        if proxy_start_may_have_completed && !operation.proxy_was_running {
            if state.proxy_service.is_running().await {
                state
                    .proxy_service
                    .stop()
                    .await
                    .map_err(AppError::Message)?;
            }
        }
        if provider_switch_may_have_completed {
            if let Some(previous_provider_id) = operation.previous_provider_id.as_deref() {
                ProviderService::switch(state, app_type.clone(), previous_provider_id)?;
            }
        }
        if operation.live_snapshot.is_some() {
            restore_live_snapshot(&operation)?;
        }
        if provider_creation_may_have_completed {
            let provider_id = operation
                .provider_id
                .as_deref()
                .ok_or_else(|| AppError::Message("Missing created provider id".to_string()))?;
            let provider_exists = state
                .db
                .get_provider_by_id(provider_id, app_type.as_str())?
                .is_some();
            if provider_exists && operation.previous_provider_id.is_none() {
                // ProviderService::delete protects the current provider. When
                // this operation created the first provider, clear both
                // current markers before removing it so rollback leaves no
                // orphaned provider or stale selection.
                state.db.clear_current_provider(app_type.as_str())?;
                crate::settings::set_current_provider(app_type, None)?;
            }
            if provider_exists {
                ProviderService::delete(state, app_type.clone(), provider_id)?;
            }
        }
        Ok(())
    }
    .await;

    match compensation {
        Ok(()) => state.db.transition_quick_start_operation(
            &operation.id,
            operation.revision,
            QuickStartOperationStatus::RolledBack,
            "compensation_completed",
            Some("QUICKSTART_APPLY_FAILED"),
            Some(safe_message),
        ),
        Err(compensation_error) => {
            let message = format!(
                "apply failed: {safe_message}; compensation failed: {}",
                compensation_error
            );
            state.db.transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::RollbackFailed,
                "compensation_failed",
                Some("QUICKSTART_COMPENSATION_FAILED"),
                Some(&message.chars().take(800).collect::<String>()),
            )
        }
    }
}

fn capture_live_snapshot(app_type: &AppType) -> Result<String, AppError> {
    let snapshot = capture_live_snapshot_value(app_type)?;
    let serialized = serde_json::to_string(&snapshot)
        .map_err(|e| AppError::Serialization(format!("Serialize live snapshot failed: {e}")))?;
    crate::keychain::seal_local_secret(&serialized)
}

fn capture_live_fingerprint(app_type: &AppType) -> Result<String, AppError> {
    let snapshot = capture_live_snapshot_value(app_type)?;
    let serialized = serde_json::to_vec(&snapshot).map_err(|e| {
        AppError::Serialization(format!("Serialize live snapshot fingerprint failed: {e}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(serialized)))
}

fn capture_live_snapshot_value(app_type: &AppType) -> Result<QuickStartLiveSnapshot, AppError> {
    if matches!(app_type, AppType::ClaudeDesktop) {
        Ok(QuickStartLiveSnapshot::ClaudeDesktop(
            crate::claude_desktop_config::capture_config_snapshot()?,
        ))
    } else {
        Ok(QuickStartLiveSnapshot::Provider(
            crate::services::provider::capture_live_snapshot(app_type)?,
        ))
    }
}

fn restore_live_snapshot(operation: &QuickStartOperation) -> Result<(), AppError> {
    let sealed = operation
        .live_snapshot
        .as_deref()
        .ok_or_else(|| AppError::Message("QuickStart live snapshot is missing".to_string()))?;
    let serialized = crate::keychain::open_local_secret(sealed)?;
    let snapshot: QuickStartLiveSnapshot = serde_json::from_str(&serialized)
        .map_err(|e| AppError::Serialization(format!("Parse live snapshot failed: {e}")))?;
    match snapshot {
        QuickStartLiveSnapshot::Provider(snapshot) => snapshot.restore(),
        QuickStartLiveSnapshot::ClaudeDesktop(snapshot) => {
            crate::claude_desktop_config::restore_config_snapshot(&snapshot)
        }
    }
}

async fn verify_applied_state(
    state: &AppState,
    app_type: &AppType,
    provider_id: &str,
) -> Result<Provider, AppError> {
    let current = ProviderService::current(state, app_type.clone())?;
    if current != provider_id {
        return Err(AppError::Message(format!(
            "Current provider mismatch: expected {provider_id}, got {current}"
        )));
    }
    let stored_provider = state
        .db
        .get_provider_by_id(provider_id, app_type.as_str())?
        .ok_or_else(|| {
            AppError::Message("Applied provider is missing from database".to_string())
        })?;
    crate::provider_keychain::verify_provider_keychain_refs(&stored_provider)?;
    let resolved_provider =
        crate::provider_keychain::resolve_provider_settings_for_use(&stored_provider, app_type)?;

    if supports_proxy_takeover(app_type) {
        let proxy_status = state
            .proxy_service
            .get_status()
            .await
            .map_err(AppError::Message)?;
        if !proxy_status.running || proxy_status.port == 0 {
            return Err(AppError::Message(
                "Proxy did not remain running after QuickStart apply".to_string(),
            ));
        }
        let takeover = state
            .proxy_service
            .get_takeover_status()
            .await
            .map_err(AppError::Message)?;
        if !takeover_enabled_for_app(&takeover, app_type.as_str()) {
            return Err(AppError::Message(
                "Proxy takeover is not active after QuickStart apply".to_string(),
            ));
        }
        if !state
            .proxy_service
            .detect_takeover_in_live_config_for_app(app_type)
        {
            return Err(AppError::Message(
                "Client Live config does not point to the managed proxy".to_string(),
            ));
        }
    } else if matches!(app_type, AppType::ClaudeDesktop) {
        let desktop = crate::claude_desktop_config::get_status(
            state.db.as_ref(),
            state.proxy_service.is_running().await,
        )?;
        if !desktop.supported
            || !desktop.configured
            || desktop.applied_id.as_deref() != Some(provider_id)
        {
            return Err(AppError::Message(
                "Claude Desktop did not report the provider as applied".to_string(),
            ));
        }
    }
    Ok(resolved_provider)
}

async fn verify_upstream_provider(
    provider: &Provider,
    app_type: &AppType,
) -> Result<QuickStartUpstreamVerificationReceipt, AppError> {
    let (api_key, base_url) = ProviderService::extract_credentials(provider, app_type)?;
    let protocol = verification_protocol_for(provider);
    let result = verify_key(&base_url, &api_key, protocol).await?;
    if !result.ok {
        // Do not persist provider-supplied response text. The operation event records
        // a stable error code through compensation, while secrets and endpoint paths
        // remain outside the audit trail.
        return Err(AppError::Message(
            "QUICKSTART_UPSTREAM_VERIFICATION_FAILED".to_string(),
        ));
    }
    build_upstream_verification_receipt(provider, &base_url, protocol, result.model_count)
}

fn verification_protocol_for(provider: &Provider) -> VerifyProtocol {
    match provider
        .meta
        .as_ref()
        .and_then(|meta| meta.api_format.as_deref())
    {
        Some("anthropic") => VerifyProtocol::Anthropic,
        // OpenAI-compatible and Gemini-native QuickStart presets both retain the
        // existing /v1/models probe contract. All other formats default to it too.
        _ => VerifyProtocol::OpenAi,
    }
}

fn build_upstream_verification_receipt(
    provider: &Provider,
    base_url: &str,
    protocol: VerifyProtocol,
    model_count: usize,
) -> Result<QuickStartUpstreamVerificationReceipt, AppError> {
    let endpoint_host = url::Url::parse(base_url)
        .map_err(|_| AppError::Message("QuickStart upstream endpoint is invalid".to_string()))?
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| AppError::Message("QuickStart upstream endpoint has no host".to_string()))?
        .to_string();
    let provider_bytes = serde_json::to_vec(provider).map_err(|error| {
        AppError::Serialization(format!("QuickStart provider fingerprint failed: {error}"))
    })?;
    Ok(QuickStartUpstreamVerificationReceipt {
        provider_fingerprint: format!("sha256:{:x}", Sha256::digest(provider_bytes)),
        protocol: match protocol {
            VerifyProtocol::OpenAi => "openai",
            VerifyProtocol::Anthropic => "anthropic",
        }
        .to_string(),
        endpoint_host,
        model_count,
    })
}

fn supports_proxy_takeover(app_type: &AppType) -> bool {
    matches!(app_type, AppType::Claude | AppType::Codex | AppType::Gemini)
}

fn takeover_enabled_for_app(
    status: &crate::proxy::types::ProxyTakeoverStatus,
    app_type: &str,
) -> bool {
    match app_type {
        "claude" => status.claude,
        "codex" => status.codex,
        "gemini" => status.gemini,
        _ => false,
    }
}

fn fail_if_requested(
    configured: Option<QuickStartFaultPoint>,
    reached: QuickStartFaultPoint,
) -> Result<(), AppError> {
    if configured == Some(reached) {
        return Err(AppError::Message(format!(
            "Injected QuickStart failure at {reached:?}"
        )));
    }
    Ok(())
}

fn request_fingerprint(request: &QuickStartApplyRequest) -> Result<String, AppError> {
    let mut stable_request = request.clone();
    stable_request.provider.created_at = None;
    redact_secrets_for_fingerprint(&mut stable_request.provider.settings_config);
    let bytes = serde_json::to_vec(&stable_request).map_err(|e| {
        AppError::Serialization(format!("QuickStart request serialization failed: {e}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Preserve only credential presence in the persisted idempotency fingerprint.
/// Storing a raw SHA-256 of an API Key would permit offline guessing if the
/// database is exposed, while the transaction's generated idempotency key
/// already scopes a retry to one renderer attempt.
fn redact_secrets_for_fingerprint(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if crate::provider_keychain::is_known_key_field(key) {
                    if child.as_str().is_some_and(|secret| !secret.is_empty()) {
                        *child = serde_json::Value::String("[credential-present]".to_string());
                    }
                    continue;
                }
                redact_secrets_for_fingerprint(child);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                redact_secrets_for_fingerprint(child);
            }
        }
        _ => {}
    }
}

fn redact_error(error: &str, provider: &Provider) -> String {
    let mut redacted = error.to_string();
    redact_known_secrets(&provider.settings_config, &mut redacted);
    redacted.chars().take(800).collect()
}

fn redact_known_secrets(value: &serde_json::Value, text: &mut String) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if crate::provider_keychain::is_known_key_field(key) {
                    if let Some(secret) = value.as_str() {
                        if !secret.is_empty() {
                            *text = text.replace(secret, "[redacted]");
                        }
                    }
                }
                redact_known_secrets(value, text);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                redact_known_secrets(value, text);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::json;
    use serial_test::serial;
    use std::env;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct TempHome {
        _dir: TempDir,
        home: Option<String>,
        userprofile: Option<String>,
        test_home: Option<String>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = TempDir::new().expect("temp home");
            let home = env::var("HOME").ok();
            let userprofile = env::var("USERPROFILE").ok();
            let test_home = env::var("OPEN_SUNSTAR_TEST_HOME").ok();
            env::set_var("HOME", dir.path());
            env::set_var("USERPROFILE", dir.path());
            env::set_var("OPEN_SUNSTAR_TEST_HOME", dir.path());
            Self {
                _dir: dir,
                home,
                userprofile,
                test_home,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            restore_env("HOME", self.home.as_deref());
            restore_env("USERPROFILE", self.userprofile.as_deref());
            restore_env("OPEN_SUNSTAR_TEST_HOME", self.test_home.as_deref());
        }
    }

    fn restore_env(key: &str, value: Option<&str>) {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    fn codex_provider(id: &str, key: &str) -> Provider {
        Provider::with_id(
            id.to_string(),
            id.to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": key},
                "config": "model = \"gpt-5\"\nmodel_provider = \"custom\"\n[model_providers.custom]\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\n"
            }),
            None,
        )
    }

    fn invalid_codex_provider(id: &str) -> Provider {
        Provider::with_id(id.to_string(), id.to_string(), json!({}), None)
    }

    #[test]
    fn request_fingerprint_ignores_retry_timestamp_but_not_configuration() {
        let mut first_provider = codex_provider("provider", "sk-same");
        first_provider.created_at = Some(100);
        let mut second_provider = first_provider.clone();
        second_provider.created_at = Some(200);
        let first = QuickStartApplyRequest {
            idempotency_key: "retry-key".to_string(),
            app_type: "codex".to_string(),
            provider: first_provider,
        };
        let mut second = QuickStartApplyRequest {
            idempotency_key: "retry-key".to_string(),
            app_type: "codex".to_string(),
            provider: second_provider,
        };
        assert_eq!(
            request_fingerprint(&first).expect("first fingerprint"),
            request_fingerprint(&second).expect("second fingerprint")
        );
        second.provider.name = "changed".to_string();
        assert_ne!(
            request_fingerprint(&first).expect("first fingerprint"),
            request_fingerprint(&second).expect("changed fingerprint")
        );

        let mut rotated_key = first.clone();
        rotated_key.provider.settings_config["auth"]["OPENAI_API_KEY"] =
            json!("sk-rotated-secret");
        assert_eq!(
            request_fingerprint(&first).expect("first fingerprint"),
            request_fingerprint(&rotated_key).expect("rotated-key fingerprint"),
            "the persisted idempotency fingerprint must not derive API Key material"
        );
    }

    #[test]
    fn upstream_verification_receipt_is_secret_free_and_bound_to_configuration() {
        let provider = codex_provider("provider", "sk-never-persist");
        let receipt = build_upstream_verification_receipt(
            &provider,
            "https://api.example.test/v1",
            VerifyProtocol::OpenAi,
            3,
        )
        .expect("safe upstream verification receipt");
        let serialized = serde_json::to_string(&receipt).expect("serialize receipt");

        assert_eq!(receipt.endpoint_host, "api.example.test");
        assert_eq!(receipt.protocol, "openai");
        assert_eq!(receipt.model_count, 3);
        assert!(receipt.provider_fingerprint.starts_with("sha256:"));
        assert!(!serialized.contains("sk-never-persist"));
    }

    #[tokio::test]
    #[serial]
    async fn fault_after_switch_restores_previous_provider_and_removes_created_provider() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));
        let previous = codex_provider("previous", "sk-previous");
        ProviderService::add(&state, AppType::Codex, previous, true).expect("add previous");
        ProviderService::switch(&state, AppType::Codex, "previous").expect("switch previous");

        let result = QuickStartService::apply_with_fault(
            &state,
            QuickStartApplyRequest {
                idempotency_key: "fault-after-switch".to_string(),
                app_type: "codex".to_string(),
                provider: codex_provider("created", "sk-created"),
            },
            QuickStartFaultPoint::AfterProviderSwitchedBeforeReceipt,
        )
        .await
        .expect("operation result");

        assert_eq!(result.status, QuickStartOperationStatus::RolledBack);
        assert!(result
            .live_snapshot
            .as_deref()
            .is_some_and(|snapshot| snapshot.starts_with("enc:v1:")));
        assert!(!result
            .live_snapshot
            .as_deref()
            .unwrap_or_default()
            .contains("sk-previous"));
        assert_eq!(
            ProviderService::current(&state, AppType::Codex).expect("current"),
            "previous"
        );
        assert!(state
            .db
            .get_provider_by_id("created", "codex")
            .expect("lookup")
            .is_none());
    }

    #[tokio::test]
    #[serial]
    async fn first_provider_failure_clears_current_selection_and_removes_provider() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));

        let result = QuickStartService::apply_with_fault(
            &state,
            QuickStartApplyRequest {
                idempotency_key: "first-provider-failure".to_string(),
                app_type: "codex".to_string(),
                provider: codex_provider("first", "sk-first"),
            },
            QuickStartFaultPoint::AfterProviderCreatedBeforeReceipt,
        )
        .await
        .expect("operation result");

        assert_eq!(result.status, QuickStartOperationStatus::RolledBack);
        assert_eq!(
            ProviderService::current(&state, AppType::Codex).expect("current"),
            ""
        );
        assert!(state
            .db
            .get_provider_by_id("first", "codex")
            .expect("lookup")
            .is_none());
        assert!(state
            .db
            .get_current_provider("codex")
            .expect("database current")
            .is_none());
        assert!(!crate::codex_config::get_codex_auth_path().exists());
        assert!(!crate::codex_config::get_codex_config_path().exists());
        assert!(!crate::codex_config::get_codex_model_catalog_path().exists());
    }

    #[tokio::test]
    #[serial]
    async fn restart_recovery_compensates_provider_created_after_intent_before_receipt() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let database = Arc::new(Database::memory().expect("database"));
        let state = AppState::new(Arc::clone(&database));

        let mut operation = state
            .db
            .begin_quick_start_operation("restart-recovery", "fingerprint", "codex")
            .expect("begin")
            .operation;
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Applying,
                "preflight",
                None,
                None,
            )
            .expect("enter applying");
        operation = state
            .db
            .record_quick_start_progress(
                &operation.id,
                operation.revision,
                "preflight_captured",
                Some("created"),
                None,
                None,
                Some(false),
                Some(false),
            )
            .expect("record preflight");
        operation = state
            .db
            .record_quick_start_live_snapshot(
                &operation.id,
                operation.revision,
                &capture_live_snapshot(&AppType::Codex).expect("capture snapshot"),
            )
            .expect("record snapshot");
        operation = state
            .db
            .record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_creation_started",
                None,
                None,
                None,
                None,
                None,
            )
            .expect("record provider creation intent");
        ProviderService::add(
            &state,
            AppType::Codex,
            codex_provider("created", "sk-created"),
            true,
        )
        .expect("external provider action succeeds before receipt");
        drop(state);

        let restarted_state = AppState::new(database);
        let recovered = QuickStartService::rollback(
            &restarted_state,
            &operation.id,
            operation.revision,
        )
        .await
        .expect("restart recovery");

        assert_eq!(recovered.status, QuickStartOperationStatus::RolledBack);
        assert!(restarted_state
            .db
            .get_provider_by_id("created", "codex")
            .expect("lookup")
            .is_none());
    }

    #[tokio::test]
    #[serial]
    async fn failed_provider_creation_is_not_recorded_as_a_created_provider() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));

        let result = QuickStartService::apply(
            &state,
            QuickStartApplyRequest {
                idempotency_key: "invalid-provider-create".to_string(),
                app_type: "codex".to_string(),
                provider: invalid_codex_provider("invalid"),
            },
        )
        .await
        .expect("failed apply must compensate");

        assert_eq!(result.status, QuickStartOperationStatus::RolledBack);
        assert!(
            !result.provider_created,
            "only a successful ProviderService::add may set the created flag"
        );
        assert!(state
            .db
            .get_provider_by_id("invalid", "codex")
            .expect("lookup")
            .is_none());
    }

    #[tokio::test]
    #[serial]
    async fn fault_after_takeover_restores_live_config_and_clears_backup() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));
        ProviderService::add(
            &state,
            AppType::Codex,
            codex_provider("previous", "sk-previous"),
            true,
        )
        .expect("add previous");

        let result = QuickStartService::apply_with_fault(
            &state,
            QuickStartApplyRequest {
                idempotency_key: "fault-after-takeover".to_string(),
                app_type: "codex".to_string(),
                provider: codex_provider("created", "sk-created"),
            },
            QuickStartFaultPoint::AfterTakeoverEnabledBeforeReceipt,
        )
        .await
        .expect("operation result");

        assert_eq!(result.status, QuickStartOperationStatus::RolledBack);
        assert!(
            !state
                .proxy_service
                .get_takeover_status()
                .await
                .expect("takeover status")
                .codex
        );
        assert!(state
            .db
            .get_live_backup("codex")
            .await
            .expect("backup query")
            .is_none());
    }

    #[tokio::test]
    #[serial]
    async fn fault_after_proxy_start_before_receipt_stops_proxy_and_restores_takeover() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));
        ProviderService::add(
            &state,
            AppType::Codex,
            codex_provider("previous", "sk-previous"),
            true,
        )
        .expect("add previous");

        let result = QuickStartService::apply_with_fault(
            &state,
            QuickStartApplyRequest {
                idempotency_key: "fault-after-proxy-start-before-receipt".to_string(),
                app_type: "codex".to_string(),
                provider: codex_provider("created", "sk-created"),
            },
            QuickStartFaultPoint::AfterProxyStartedBeforeReceipt,
        )
        .await
        .expect("operation result");

        assert_eq!(result.status, QuickStartOperationStatus::RolledBack);
        assert!(!state.proxy_service.is_running().await);
        assert!(
            !state
                .proxy_service
                .get_takeover_status()
                .await
                .expect("takeover status")
                .codex
        );
    }

    #[tokio::test]
    #[serial]
    async fn successful_operation_can_be_revision_guarded_and_auditably_rolled_back() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));
        ProviderService::add(
            &state,
            AppType::Codex,
            codex_provider("previous", "sk-previous"),
            true,
        )
        .expect("add previous");
        ProviderService::add(
            &state,
            AppType::Codex,
            codex_provider("created", "sk-created"),
            true,
        )
        .expect("add created");
        ProviderService::switch(&state, AppType::Codex, "created").expect("switch created");

        let mut operation = state
            .db
            .begin_quick_start_operation("manual-rollback", "fingerprint", "codex")
            .expect("begin")
            .operation;
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Applying,
                "preflight",
                None,
                None,
            )
            .expect("applying");
        operation = state
            .db
            .record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_created",
                Some("created"),
                Some("previous"),
                Some("provider_created"),
                Some(false),
                Some(false),
            )
            .expect("record created");
        operation = state
            .db
            .record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_switched",
                None,
                None,
                Some("provider_switched"),
                None,
                None,
            )
            .expect("record switched");
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Verifying,
                "verify",
                None,
                None,
            )
            .expect("verifying");
        operation = state
            .db
            .record_quick_start_applied_live_fingerprint(
                &operation.id,
                operation.revision,
                &capture_live_fingerprint(&AppType::Codex).expect("capture guard"),
            )
            .expect("record rollback guard");
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Succeeded,
                "completed",
                None,
                None,
            )
            .expect("succeeded");

        let rolled_back = QuickStartService::rollback(&state, &operation.id, operation.revision)
            .await
            .expect("rollback");
        assert_eq!(rolled_back.status, QuickStartOperationStatus::RolledBack);
        assert_eq!(
            ProviderService::current(&state, AppType::Codex).expect("current"),
            "previous"
        );
        assert!(state
            .db
            .get_provider_by_id("created", "codex")
            .expect("lookup")
            .is_none());

        let events = state
            .db
            .list_quick_start_operation_events(&operation.id)
            .expect("events");
        assert!(events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence));
        assert_eq!(
            events.last().map(|event| event.to_status.as_deref()),
            Some(Some("rolled_back"))
        );
    }

    #[tokio::test]
    #[serial]
    async fn rollback_refuses_to_overwrite_a_provider_selected_after_the_operation() {
        let _home = TempHome::new();
        crate::settings::reload_settings().expect("reload settings");
        let state = AppState::new(Arc::new(Database::memory().expect("database")));
        for id in ["previous", "created", "user-selected"] {
            ProviderService::add(
                &state,
                AppType::Codex,
                codex_provider(id, &format!("sk-{id}")),
                true,
            )
            .expect("seed provider");
        }
        ProviderService::switch(&state, AppType::Codex, "created").expect("switch created");

        let mut operation = state
            .db
            .begin_quick_start_operation("rollback-ownership", "fingerprint", "codex")
            .expect("begin")
            .operation;
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Applying,
                "preflight",
                None,
                None,
            )
            .expect("applying");
        operation = state
            .db
            .record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_created",
                Some("created"),
                Some("previous"),
                Some("provider_created"),
                Some(false),
                Some(false),
            )
            .expect("record created");
        operation = state
            .db
            .record_quick_start_progress(
                &operation.id,
                operation.revision,
                "provider_switched",
                None,
                None,
                Some("provider_switched"),
                None,
                None,
            )
            .expect("record switched");
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Verifying,
                "verify",
                None,
                None,
            )
            .expect("verifying");
        operation = state
            .db
            .record_quick_start_applied_live_fingerprint(
                &operation.id,
                operation.revision,
                &capture_live_fingerprint(&AppType::Codex).expect("capture guard"),
            )
            .expect("record rollback guard");
        operation = state
            .db
            .transition_quick_start_operation(
                &operation.id,
                operation.revision,
                QuickStartOperationStatus::Succeeded,
                "completed",
                None,
                None,
            )
            .expect("succeeded");

        ProviderService::switch(&state, AppType::Codex, "user-selected")
            .expect("user changes selection after success");
        let error = QuickStartService::rollback(&state, &operation.id, operation.revision)
            .await
            .expect_err("rollback must not overwrite user changes");

        assert!(error.to_string().contains("QUICKSTART_ROLLBACK_CONFLICT"));
        assert_eq!(
            ProviderService::current(&state, AppType::Codex).expect("current"),
            "user-selected"
        );
        assert!(state
            .db
            .get_provider_by_id("created", "codex")
            .expect("created provider retained")
            .is_some());
    }
}
