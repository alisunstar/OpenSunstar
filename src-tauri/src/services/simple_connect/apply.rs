//! Simple Connect 统一 apply / clear / status（共享内核写入层）

use crate::error::AppError;
use crate::services::simple_connect::backup;
use crate::services::simple_connect::key_store::{get_primary_key, key_hint};
use crate::services::simple_connect::proxy_poc::{start_spike_proxy, SpikeProxyInfo, SPIKE_PROXY_PORT};
use crate::services::simple_connect::state::{load_state, save_state, SimpleConnectState};
use crate::services::simple_connect::suppliers::resolve_supplier;
use crate::services::simple_connect::tools::{self, tool_paths, tool_status, MANAGED_MARKER};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub tool: String,
    pub files: Vec<String>,
    pub backup_path: Option<String>,
    pub used_pool: bool,
    pub proxy_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolConfigStatus {
    pub tool: String,
    pub configured: bool,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub key_hint: Option<String>,
    pub supported: bool,
}

#[derive(Debug, Clone)]
struct ResolvedCredentials {
    auth_token: String,
    openai_base: String,
    anthropic_base: String,
    used_pool: bool,
    proxy_info: Option<SpikeProxyInfo>,
}

fn backup_tool_files(tool: &str) -> Result<Option<PathBuf>, AppError> {
    let paths = tool_paths(tool)?;
    let mut last: Option<PathBuf> = None;
    for path in paths {
        if let Some(b) = backup::backup_file(tool, &path)? {
            last = Some(b);
        }
    }
    Ok(last)
}

async fn resolve_credentials(
    supplier_id: &str,
    custom_base: Option<&str>,
    _use_pool: bool,
) -> Result<ResolvedCredentials, AppError> {
    let supplier = resolve_supplier(supplier_id, custom_base)
        .ok_or_else(|| AppError::Message(format!("未知供应商: {supplier_id}")))?;
    let openai_base = supplier.openai_base.clone();

    let _ = get_primary_key(supplier_id)?
        .ok_or_else(|| AppError::Message("请先在 Keychain 中保存 API Key".into()))?;

    // Phase 1 决议：真实 Key 仅存 Keychain，CLI 一律写 local token + 本地代理
    let info = start_spike_proxy(supplier_id, &openai_base).await?;
    Ok(ResolvedCredentials {
        auth_token: info.local_token.clone(),
        openai_base: format!("{}/v1", info.local_base.trim_end_matches('/')),
        anthropic_base: info.local_base.clone(),
        used_pool: true,
        proxy_info: Some(info),
    })
}

pub async fn apply_tool(
    tool: &str,
    supplier_id: &str,
    model: &str,
    custom_base: Option<&str>,
    use_pool: bool,
) -> Result<ApplyResult, AppError> {
    if !tools::PHASE1_TOOLS.contains(&tool) {
        return Err(AppError::Message(format!(
            "不支持的工具 {tool}，当前支持: {}",
            tools::PHASE1_TOOLS.join(" / ")
        )));
    }
    if model.trim().is_empty() {
        return Err(AppError::Message("请选择模型".into()));
    }

    let creds = resolve_credentials(supplier_id, custom_base, use_pool).await?;
    let backup_path = backup_tool_files(tool)?;

    let outcome = match tool {
        "claude-code" => tools::claude::apply(&creds.auth_token, model, &creds.anthropic_base)?,
        "codex" => tools::codex::apply(supplier_id, &creds.auth_token, model, &creds.openai_base)?,
        "gemini-cli" => tools::gemini::apply(&creds.auth_token, model, &creds.openai_base)?,
        "opencode" => tools::opencode::apply(&creds.auth_token, model, &creds.openai_base)?,
        "openclaw" => tools::openclaw::apply(&creds.auth_token, model, &creds.anthropic_base)?,
        "hermes" => tools::hermes::apply(&creds.auth_token, model, &creds.openai_base)?,
        other => return Err(AppError::Message(format!("未知工具: {other}"))),
    };

    let mut state = load_state()?;
    state.supplier_id = supplier_id.to_string();
    state.custom_openai_base = custom_base
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    state.pool_enabled = use_pool;
    state.last_model = Some(model.trim().to_string());
    state.last_tool = Some(tool.to_string());
    state.last_applied_supplier_id = Some(supplier_id.to_string());
    save_state(&state)?;

    Ok(ApplyResult {
        tool: tool.to_string(),
        files: outcome.files.iter().map(|p| p.display().to_string()).collect(),
        backup_path: backup_path.map(|p| p.display().to_string()),
        used_pool: creds.used_pool,
        proxy_port: creds.proxy_info.map(|_| SPIKE_PROXY_PORT),
    })
}

pub fn clear_tool(tool: &str) -> Result<(), AppError> {
    let _ = backup_tool_files(tool)?;
    match tool {
        "claude-code" => tools::claude::clear(),
        "codex" => tools::codex::clear(),
        "gemini-cli" => tools::gemini::clear(),
        "opencode" => tools::opencode::clear(),
        "openclaw" => tools::openclaw::clear(),
        "hermes" => tools::hermes::clear(),
        other => Err(AppError::Message(format!("未知工具: {other}"))),
    }
}

pub fn list_tool_status() -> Result<Vec<ToolConfigStatus>, AppError> {
    let mut out = Vec::new();
    for tool in tools::ALL_TOOLS {
        let status = tool_status(tool)?;
        out.push(ToolConfigStatus {
            tool: tool.to_string(),
            configured: status.configured,
            base_url: status.base_url,
            model: status.model,
            key_hint: status.key.as_ref().map(|k| key_hint(k)),
            supported: true,
        });
    }
    Ok(out)
}

pub fn get_state() -> Result<SimpleConnectState, AppError> {
    load_state()
}

pub fn save_pool_state(state: SimpleConnectState) -> Result<(), AppError> {
    save_state(&state)
}

pub fn is_managed_marker(value: &str) -> bool {
    value == MANAGED_MARKER
}
