//! Simple Connect Tauri commands（Phase 0 Spike + Phase 1 MVP）

use crate::services::simple_connect::{
    apply_claude_code, apply_tool, build_usage_summary, clear_tool, delete_api_key,
    fetch_models_via_proxy, get_primary_key, get_state, import_keys, is_simple_connect_import_url,
    key_hint, list_builtin_suppliers, list_tool_status, pool_runtime_stats, resolve_supplier,
    run_backup_audit, run_pool_demo, run_spike_report, save_pool_state, set_supplier, spike_proxy_info,
    start_spike_proxy, stop_spike_proxy, store_api_key, store_primary_key, try_parse_url,
    verify_api_key, ApplyResult, BackupAuditReport, ClaudeApplyResult, PoolSimulationStep, SimpleConnectImportPayload,
    SimpleConnectImportResult, SimpleConnectRuntimeStats, SimpleConnectState,
    SimpleConnectUsageSummary, SpikeProxyInfo, SpikeReport, SupplierProfile, ToolConfigStatus,
    VerifyKeyResult, ALL_TOOLS, PHASE1_TOOLS, run_p0_security_audit, P0SecurityReport,
};

#[tauri::command]
pub fn simple_connect_list_suppliers() -> Vec<SupplierProfile> {
    list_builtin_suppliers()
}

#[tauri::command]
pub fn simple_connect_list_tools() -> Vec<&'static str> {
    PHASE1_TOOLS.to_vec()
}

#[tauri::command]
pub fn simple_connect_all_tools() -> Vec<&'static str> {
    ALL_TOOLS.to_vec()
}

#[tauri::command]
pub fn simple_connect_get_state() -> Result<SimpleConnectState, String> {
    get_state().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_set_supplier(
    supplier_id: String,
    custom_openai_base: Option<String>,
) -> Result<SimpleConnectState, String> {
    set_supplier(&supplier_id, custom_openai_base.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_save_state(state: SimpleConnectState) -> Result<(), String> {
    save_pool_state(state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_store_key(
    supplier_id: String,
    api_key: String,
) -> Result<String, String> {
    store_primary_key(&supplier_id, &api_key).map_err(|e| e.to_string())?;
    Ok(key_hint(api_key.trim()))
}

#[tauri::command]
pub fn simple_connect_store_pool_key(
    supplier_id: String,
    key_id: String,
    api_key: String,
) -> Result<String, String> {
    store_api_key(&supplier_id, &key_id, &api_key).map_err(|e| e.to_string())?;
    Ok(key_hint(api_key.trim()))
}

#[tauri::command]
pub fn simple_connect_remove_pool_key(
    supplier_id: String,
    key_id: String,
) -> Result<(), String> {
    delete_api_key(&supplier_id, &key_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_key_configured(supplier_id: String) -> Result<bool, String> {
    get_primary_key(&supplier_id)
        .map(|k| k.is_some())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_list_tool_status() -> Result<Vec<ToolConfigStatus>, String> {
    list_tool_status().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_apply(
    tool: String,
    supplier_id: String,
    model: String,
    custom_base: Option<String>,
    use_pool: Option<bool>,
) -> Result<ApplyResult, String> {
    apply_tool(
        &tool,
        &supplier_id,
        &model,
        custom_base.as_deref(),
        use_pool.unwrap_or(false),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_clear(tool: String) -> Result<(), String> {
    clear_tool(&tool).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_spike_apply_claude(
    supplier_id: String,
    model: Option<String>,
    custom_base: Option<String>,
) -> Result<ClaudeApplyResult, String> {
    let supplier = resolve_supplier(&supplier_id, custom_base.as_deref())
        .ok_or_else(|| format!("未知供应商: {supplier_id}"))?;
    let anthropic_base = supplier
        .anthropic_base
        .as_deref()
        .unwrap_or(supplier.openai_base.as_str());
    let model = model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| supplier.default_model.clone());
    let key = get_primary_key(&supplier_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "请先在 Keychain 中保存 API Key".to_string())?;
    apply_claude_code(&key, &model, anthropic_base).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_spike_start_proxy(
    supplier_id: String,
    custom_base: Option<String>,
) -> Result<SpikeProxyInfo, String> {
    let supplier = resolve_supplier(&supplier_id, custom_base.as_deref())
        .ok_or_else(|| format!("未知供应商: {supplier_id}"))?;
    start_spike_proxy(&supplier_id, &supplier.openai_base)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_spike_stop_proxy() {
    stop_spike_proxy().await;
}

#[tauri::command]
pub async fn simple_connect_spike_proxy_info() -> Option<SpikeProxyInfo> {
    spike_proxy_info().await
}

#[tauri::command]
pub async fn simple_connect_spike_fetch_models(
    supplier_id: String,
    custom_base: Option<String>,
) -> Result<Vec<String>, String> {
    let supplier = resolve_supplier(&supplier_id, custom_base.as_deref())
        .ok_or_else(|| format!("未知供应商: {supplier_id}"))?;
    if get_primary_key(&supplier_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err("请先在 Keychain 中保存 API Key".into());
    }
    fetch_models_via_proxy(&supplier_id, &supplier.openai_base)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_fetch_models(
    supplier_id: String,
    custom_base: Option<String>,
) -> Result<Vec<String>, String> {
    simple_connect_spike_fetch_models(supplier_id, custom_base).await
}

#[tauri::command]
pub fn simple_connect_spike_pool_demo() -> Vec<PoolSimulationStep> {
    run_pool_demo()
}

#[tauri::command]
pub async fn simple_connect_pool_stats() -> SimpleConnectRuntimeStats {
    pool_runtime_stats().await
}

#[tauri::command]
pub fn simple_connect_security_p0_audit() -> Result<P0SecurityReport, String> {
    run_p0_security_audit().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_spike_run_all(
    supplier_id: String,
    test_api_key: Option<String>,
) -> Result<SpikeReport, String> {
    run_spike_report(&supplier_id, test_api_key.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_is_import_url(url: String) -> bool {
    is_simple_connect_import_url(&url)
}

#[tauri::command]
pub fn simple_connect_parse_import_url(
    url: String,
) -> Result<SimpleConnectImportPayload, String> {
    try_parse_url(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_import_from_url(
    url: String,
    skip_verify: Option<bool>,
) -> Result<SimpleConnectImportResult, String> {
    let payload = try_parse_url(&url).map_err(|e| e.to_string())?;
    import_keys(&payload, skip_verify.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_import_keys(
    payload: SimpleConnectImportPayload,
    skip_verify: Option<bool>,
) -> Result<SimpleConnectImportResult, String> {
    import_keys(&payload, skip_verify.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn simple_connect_verify_key(
    supplier_id: String,
    api_key: String,
    custom_base: Option<String>,
) -> Result<VerifyKeyResult, String> {
    verify_api_key(
        &supplier_id,
        &api_key,
        custom_base.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_usage_summary() -> Result<SimpleConnectUsageSummary, String> {
    build_usage_summary().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn simple_connect_backup_audit() -> Result<BackupAuditReport, String> {
    run_backup_audit().map_err(|e| e.to_string())
}
