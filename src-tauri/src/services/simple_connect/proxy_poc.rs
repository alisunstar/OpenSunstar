//! Simple Connect 本地代理 POC（Phase 1 T3）
//!
//! 独立于主 Proxy 模块，监听 `127.0.0.1:17172`，避免与现有代理端口冲突。
//! 请求路径：校验 local token → 密钥池 pick → Keychain 取真实 Key → upstream 转发 + failover。

use crate::error::AppError;
use crate::services::simple_connect::key_store;
use crate::services::simple_connect::pool::{build_runtime_pool, KeyPool};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

pub const SPIKE_PROXY_PORT: u16 = 17172;

#[derive(Clone)]
struct ProxyState {
    upstream: String,
    supplier_id: String,
    local_token: String,
    pool: Arc<Mutex<KeyPool>>,
}

static SPIKE_HANDLE: OnceLock<Mutex<Option<tokio::task::JoinHandle<()>>>> = OnceLock::new();
static SPIKE_INFO: OnceLock<RwLock<Option<SpikeProxyInfo>>> = OnceLock::new();
static SPIKE_POOL: OnceLock<RwLock<Option<Arc<Mutex<KeyPool>>>>> = OnceLock::new();

fn pool_store() -> &'static RwLock<Option<Arc<Mutex<KeyPool>>>> {
    SPIKE_POOL.get_or_init(|| RwLock::new(None))
}

fn handle_store() -> &'static Mutex<Option<tokio::task::JoinHandle<()>>> {
    SPIKE_HANDLE.get_or_init(|| Mutex::new(None))
}

fn info_store() -> &'static RwLock<Option<SpikeProxyInfo>> {
    SPIKE_INFO.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeProxyInfo {
    pub local_base: String,
    pub upstream: String,
    pub supplier_id: String,
    pub local_token_hint: String,
    #[serde(skip_serializing)]
    pub local_token: String,
    pub pool_key_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleConnectRuntimeStats {
    pub running: bool,
    pub local_base: Option<String>,
    pub upstream: Option<String>,
    pub supplier_id: Option<String>,
    pub port: u16,
    pub pool_keys: Vec<crate::services::simple_connect::pool::PoolKeyStat>,
}

fn openai_root(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

pub async fn start_spike_proxy(supplier_id: &str, upstream: &str) -> Result<SpikeProxyInfo, AppError> {
    stop_spike_proxy().await;

    let pool = build_runtime_pool(supplier_id)?;
    let pool_key_count = pool.len();
    let pool_arc = Arc::new(Mutex::new(pool));
    let local_token = format!("sc-local-{}", uuid::Uuid::new_v4());
    let state = ProxyState {
        upstream: upstream.trim().trim_end_matches('/').to_string(),
        supplier_id: supplier_id.to_string(),
        local_token: local_token.clone(),
        pool: pool_arc.clone(),
    };

    let app = Router::new()
        .route("/__simple_connect/health", axum::routing::get(health))
        .fallback(any(forward))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], SPIKE_PROXY_PORT));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Message(format!("无法绑定 Spike 代理端口 {SPIKE_PROXY_PORT}: {e}")))?;

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log::error!("Simple Connect spike proxy stopped: {e}");
        }
    });

    *handle_store().lock().await = Some(handle);
    *pool_store().write().await = Some(pool_arc);

    let info = SpikeProxyInfo {
        local_base: format!("http://127.0.0.1:{SPIKE_PROXY_PORT}"),
        upstream: upstream.to_string(),
        supplier_id: supplier_id.to_string(),
        local_token_hint: key_store::key_hint(&local_token),
        local_token: local_token.clone(),
        pool_key_count,
    };

    *info_store().write().await = Some(info.clone());
    Ok(info)
}

pub async fn stop_spike_proxy() {
    if let Some(handle) = handle_store().lock().await.take() {
        handle.abort();
    }
    *info_store().write().await = None;
    *pool_store().write().await = None;
}

pub async fn spike_proxy_info() -> Option<SpikeProxyInfo> {
    info_store().read().await.clone()
}

pub async fn pool_runtime_stats() -> SimpleConnectRuntimeStats {
    let info = spike_proxy_info().await;
    let pool = pool_store().read().await.clone();
    let now = Instant::now();
    let pool_keys = if let Some(pool) = pool {
        pool.lock().await.snapshot_stats(now)
    } else {
        Vec::new()
    };

    SimpleConnectRuntimeStats {
        running: info.is_some(),
        local_base: info.as_ref().map(|i| i.local_base.clone()),
        upstream: info.as_ref().map(|i| i.upstream.clone()),
        supplier_id: info.as_ref().map(|i| i.supplier_id.clone()),
        port: SPIKE_PROXY_PORT,
        pool_keys,
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn forward(
    State(state): State<Arc<ProxyState>>,
    req: Request<Body>,
) -> Response {
    match forward_inner(state, req).await {
        Ok(response) => response,
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            err.to_string(),
        )
            .into_response(),
    }
}

fn should_retry(status: StatusCode) -> bool {
    matches!(
        status.as_u16(),
        401 | 402 | 403 | 407 | 408 | 425 | 429 | 500 | 502 | 503 | 504
    )
}

async fn forward_inner(
    state: Arc<ProxyState>,
    req: Request<Body>,
) -> Result<Response, AppError> {
    let auth = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or(auth).trim();
    if token != state.local_token {
        return Ok((
            StatusCode::UNAUTHORIZED,
            "invalid local token for Simple Connect proxy",
        )
            .into_response());
    }

    let path = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    let url = format!("{}{}", openai_root(&state.upstream), normalize_path(path));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Message(format!("HTTP client: {e}")))?;

    let method = req.method().clone();
    let body_bytes = axum::body::to_bytes(req.into_body(), 4 * 1024 * 1024)
        .await
        .map_err(|e| AppError::Message(format!("read body: {e}")))?;

    let max_attempts = {
        let pool = state.pool.lock().await;
        pool.len().max(1)
    };

    let now = Instant::now();
    let mut last_status = StatusCode::BAD_GATEWAY;
    let mut last_body: Option<Vec<u8>> = None;
    let mut last_headers = HeaderMap::new();

    for _ in 0..max_attempts {
        let pick = {
            let mut pool = state.pool.lock().await;
            pool.pick_next(now)
        };
        let Some(pick) = pick else {
            break;
        };

        let secret = key_store::get_api_key(&state.supplier_id, &pick.key_id)?
            .ok_or_else(|| AppError::Message(format!("Keychain 缺少密钥 {}", pick.key_id)))?;

        let mut rb = client.request(method.clone(), &url).bearer_auth(&secret);
        rb = rb.header("Accept", "application/json");
        if !body_bytes.is_empty() {
            rb = rb.body(body_bytes.clone());
        }

        match rb.send().await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| AppError::Message(format!("read upstream body: {e}")))?;

                if should_retry(status) {
                    let mut pool = state.pool.lock().await;
                    pool.record(false, Some(status.as_u16()), now);
                    last_status = status;
                    last_body = Some(bytes.to_vec());
                    last_headers = headers;
                    continue;
                }

                let mut pool = state.pool.lock().await;
                pool.record(true, Some(status.as_u16()), now);

                if let Some((input, output, cache)) =
                    crate::services::simple_connect::token_usage::extract_usage_from_body(&bytes)
                {
                    crate::services::simple_connect::token_usage::add_usage(
                        input, output, cache,
                    );
                }

                let mut out = Response::new(Body::from(bytes));
                *out.status_mut() = status;
                copy_content_type(&headers, out.headers_mut());
                return Ok(out);
            }
            Err(e) => {
                let mut pool = state.pool.lock().await;
                pool.record(false, None, now);
                log::warn!(
                    "Simple Connect proxy upstream error key={} url={}: {e}",
                    pick.key_id,
                    url
                );
                last_status = StatusCode::BAD_GATEWAY;
                continue;
            }
        }
    }

    if let Some(bytes) = last_body {
        let mut out = Response::new(Body::from(bytes));
        *out.status_mut() = last_status;
        copy_content_type(&last_headers, out.headers_mut());
        return Ok(out);
    }

    Ok((
        StatusCode::BAD_GATEWAY,
        "所有 API Key 都请求失败，请检查密钥与上游网络",
    )
        .into_response())
}

fn normalize_path(path_and_query: &str) -> String {
    if path_and_query.starts_with("/v1/") || path_and_query == "/v1" {
        path_and_query.to_string()
    } else if path_and_query.starts_with('/') {
        format!("/v1{path_and_query}")
    } else {
        format!("/v1/{path_and_query}")
    }
}

fn copy_content_type(from: &HeaderMap, to: &mut HeaderMap) {
    if let Some(ct) = from.get(axum::http::header::CONTENT_TYPE) {
        let _ = to.insert(axum::http::header::CONTENT_TYPE, ct.clone());
    }
}

pub async fn fetch_models_via_proxy(
    supplier_id: &str,
    upstream: &str,
) -> Result<Vec<String>, AppError> {
    let info = start_spike_proxy(supplier_id, upstream).await?;

    let url = format!("{}/v1/models", info.local_base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| AppError::Message(format!("HTTP client: {e}")))?;

    let resp = client
        .get(&url)
        .bearer_auth(&info.local_token)
        .send()
        .await
        .map_err(|e| AppError::Message(format!("GET {url} via proxy failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Message(format!(
            "GET {url} via proxy → {status}: {}",
            &body[..body.len().min(200)]
        )));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Message(format!("parse models JSON: {e}")))?;

    let ids = parse_model_ids(&v);

    log::info!(
        "Simple Connect proxy at {} (upstream {}, {} pool keys), fetched {} models via local token",
        info.local_base,
        upstream,
        info.pool_key_count,
        ids.len()
    );

    Ok(ids)
}

fn parse_model_ids(v: &serde_json::Value) -> Vec<String> {
    let mut ids: Vec<String> = v
        .pointer("/data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_retry_matches_beeapi_statuses() {
        assert!(should_retry(StatusCode::TOO_MANY_REQUESTS));
        assert!(should_retry(StatusCode::UNAUTHORIZED));
        assert!(!should_retry(StatusCode::OK));
        assert!(!should_retry(StatusCode::NOT_FOUND));
    }

    #[test]
    fn parse_model_ids_extracts_sorted_unique() {
        let v = serde_json::json!({
            "data": [
                { "id": "b" },
                { "id": "a" },
                { "id": "a" }
            ]
        });
        assert_eq!(parse_model_ids(&v), vec!["a", "b"]);
    }
}
