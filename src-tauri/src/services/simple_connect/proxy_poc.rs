//! Simple Connect 本地代理 POC（Phase 1 T3）
//!
//! 独立于主 Proxy 模块，监听 `127.0.0.1:17172`，避免与现有代理端口冲突。
//! 请求路径：校验 local token → 密钥池 pick → Keychain 取真实 Key → upstream 转发 + failover。

use crate::error::AppError;
use crate::services::simple_connect::key_store;
use crate::services::simple_connect::pool::{build_runtime_pool, KeyPool};
use crate::services::simple_connect::suppliers::ApiProtocol;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use futures::StreamExt;
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
    api_protocol: ApiProtocol,
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

pub async fn start_spike_proxy(
    supplier_id: &str,
    upstream: &str,
) -> Result<SpikeProxyInfo, AppError> {
    // Idempotent: if the proxy is already running for the same supplier,
    // return the existing info — avoids port rebinding errors (os error 10048)
    // and keeps the same local token across all CLI configs.
    if let Some(existing) = info_store().read().await.clone() {
        if existing.supplier_id == supplier_id {
            return Ok(existing);
        }
    }

    stop_spike_proxy().await;
    // Give the OS a moment to release the port after aborting the old task
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let pool = build_runtime_pool(supplier_id)?;
    let pool_key_count = pool.len();
    let pool_arc = Arc::new(Mutex::new(pool));
    let local_token = format!("sc-local-{}", uuid::Uuid::new_v4());
    let state = ProxyState {
        upstream: upstream.trim().trim_end_matches('/').to_string(),
        supplier_id: supplier_id.to_string(),
        api_protocol: crate::services::simple_connect::suppliers::resolve_protocol(supplier_id),
        local_token: local_token.clone(),
        pool: pool_arc.clone(),
    };

    let app = Router::new()
        .route("/__simple_connect/health", axum::routing::get(health))
        .fallback(any(forward))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], SPIKE_PROXY_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        AppError::Message(format!("无法绑定 Spike 代理端口 {SPIKE_PROXY_PORT}: {e}"))
    })?;

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

async fn forward(State(state): State<Arc<ProxyState>>, req: Request<Body>) -> Response {
    match forward_inner(state, req).await {
        Ok(response) => response,
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn should_retry(status: StatusCode) -> bool {
    matches!(
        status.as_u16(),
        401 | 402 | 403 | 407 | 408 | 425 | 429 | 500 | 502 | 503 | 504
    )
}

/// 检测请求是否为 SSE 流式请求。
/// 检查 JSON body 中的 `"stream": true` 或 Accept 头中的 `text/event-stream`。
fn is_streaming_request(body: &[u8], headers: &HeaderMap) -> bool {
    // 检查 Accept 头
    if let Some(accept) = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
    {
        if accept.contains("text/event-stream") {
            return true;
        }
    }
    // 检查 JSON body 中的 stream 字段
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) {
        if v.get("stream").and_then(|s| s.as_bool()).unwrap_or(false) {
            return true;
        }
    }
    false
}

async fn forward_inner(state: Arc<ProxyState>, req: Request<Body>) -> Result<Response, AppError> {
    // ── 本地 token 校验：支持 Authorization: Bearer 和 x-api-key 两种格式 ──
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.strip_prefix("Bearer ").unwrap_or(v).trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_default();
    if token != state.local_token {
        return Ok((
            StatusCode::UNAUTHORIZED,
            "invalid local token for Simple Connect proxy",
        )
            .into_response());
    }

    // ── 在 into_body() 消费 req 之前，保存需要转发的头部和方法 ──
    let method = req.method().clone();
    let orig_headers = req.headers().clone();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", openai_root(&state.upstream), normalize_path(path));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Message(format!("HTTP client: {e}")))?;

    let body_bytes = axum::body::to_bytes(req.into_body(), 4 * 1024 * 1024)
        .await
        .map_err(|e| AppError::Message(format!("read body: {e}")))?;

    // ── P0-3: 幂等性判断 — POST 类非幂等请求仅在连接错误/408/429 时重试 ──
    let is_idempotent = matches!(
        method,
        axum::http::Method::GET
            | axum::http::Method::HEAD
            | axum::http::Method::PUT
            | axum::http::Method::DELETE
            | axum::http::Method::OPTIONS
    );

    // ── P0-2: 检测流式请求 ──
    let want_stream = is_streaming_request(&body_bytes, &orig_headers);

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

        // ── 按协议构造上游认证头 ──
        let mut rb = client.request(method.clone(), &url);
        match state.api_protocol {
            ApiProtocol::Anthropic => {
                rb = rb.header("x-api-key", &secret);
                rb = rb.header("anthropic-version", "2023-06-01");
            }
            ApiProtocol::OpenAi => {
                rb = rb.bearer_auth(&secret);
            }
        }

        // ── 选择性转发原始请求头部 ──
        if let Some(ct) = orig_headers.get(axum::http::header::CONTENT_TYPE) {
            rb = rb.header(axum::http::header::CONTENT_TYPE, ct.clone());
        }
        if let Some(beta) = orig_headers.get("anthropic-beta") {
            rb = rb.header("anthropic-beta", beta.clone());
        }
        // Accept 头：保留客户端原始值，不再强制覆盖为 application/json
        if let Some(accept) = orig_headers.get(axum::http::header::ACCEPT) {
            rb = rb.header(axum::http::header::ACCEPT, accept.clone());
        } else {
            rb = rb.header("Accept", "application/json");
        }

        if !body_bytes.is_empty() {
            rb = rb.body(body_bytes.clone());
        }

        match rb.send().await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();

                // ── P0-2: SSE 流式透传 ──
                // 如果客户端请求流式 且 上游返回成功，直接以字节流透传
                if want_stream && status.is_success() {
                    let mut pool = state.pool.lock().await;
                    pool.record(true, Some(status.as_u16()), now);

                    let stream = resp.bytes_stream().map(|result| {
                        result.map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("upstream stream error: {e}"),
                            )
                        })
                    });

                    let mut out = Response::new(Body::from_stream(stream));
                    *out.status_mut() = status;
                    // 转发关键响应头（Content-Type、Cache-Control 等）
                    copy_response_headers(&headers, out.headers_mut());
                    return Ok(out);
                }

                // ── 非流式路径：整包读取 ──
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| AppError::Message(format!("read upstream body: {e}")))?;

                // ── P0-3: 重试策略区分幂等性 ──
                if should_retry(status) {
                    // 非幂等请求（POST 等）仅在 408/429 时重试，不在 5xx 时重试
                    // 避免上游已处理请求后重发导致重复扣费
                    if !is_idempotent && !matches!(status.as_u16(), 408 | 429) {
                        let mut pool = state.pool.lock().await;
                        pool.record(false, Some(status.as_u16()), now);
                        last_status = status;
                        last_body = Some(bytes.to_vec());
                        last_headers = headers;
                        break; // 非幂等 + 5xx → 不重试，直接返回
                    }

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
                    crate::services::simple_connect::token_usage::add_usage(input, output, cache);
                }

                let mut out = Response::new(Body::from(bytes));
                *out.status_mut() = status;
                copy_content_type(&headers, out.headers_mut());
                return Ok(out);
            }
            Err(e) => {
                // 连接级错误：幂等/非幂等都可重试（请求未到达上游）
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

/// 去掉客户端路径中的 /v1 前缀（如果存在），统一由 openai_root 负责补全。
/// 这样 openai_root(base) + normalize_path(path) 不会出现 /v1/v1 双拼。
fn normalize_path(path_and_query: &str) -> String {
    // 先分离 query string
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (path_and_query, None),
    };

    // 去掉 /v1 前缀，由 openai_root 统一负责补全
    let result = if path == "/v1" {
        "/".to_string()
    } else if let Some(rest) = path.strip_prefix("/v1/") {
        format!("/{rest}")
    } else if let Some(rest) = path.strip_prefix("/v1") {
        if rest.is_empty() {
            "/".to_string()
        } else {
            format!("/{rest}")
        }
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };

    match query {
        Some(q) => format!("{result}?{q}"),
        None => result,
    }
}

fn copy_content_type(from: &HeaderMap, to: &mut HeaderMap) {
    if let Some(ct) = from.get(axum::http::header::CONTENT_TYPE) {
        let _ = to.insert(axum::http::header::CONTENT_TYPE, ct.clone());
    }
}

/// 转发上游响应中有意义的头部，过滤掉逐跳（hop-by-hop）头。
/// 用于 SSE 流式响应，确保 Content-Type、Cache-Control 等关键头部被保留。
fn copy_response_headers(from: &HeaderMap, to: &mut HeaderMap) {
    // 逐跳头部列表，不应被转发
    const HOP_BY_HOP: &[&str] = &[
        "connection",
        "keep-alive",
        "transfer-encoding",
        "te",
        "trailer",
        "upgrade",
        "proxy-authorization",
        "proxy-authenticate",
    ];
    for (name, value) in from.iter() {
        let name_lower = name.as_str().to_ascii_lowercase();
        if HOP_BY_HOP.iter().any(|h| *h == name_lower) {
            continue;
        }
        let _ = to.insert(name.clone(), value.clone());
    }
}

pub async fn fetch_models_via_proxy(
    supplier_id: &str,
    upstream: &str,
) -> Result<Vec<String>, AppError> {
    let protocol = crate::services::simple_connect::suppliers::resolve_protocol(supplier_id);

    if protocol == ApiProtocol::Anthropic {
        // Anthropic 不提供 /v1/models 端点，返回空列表
        // 由前端回退到 default_model
        log::info!(
            "Anthropic supplier: skipping /v1/models fetch (endpoint not available), \
             returning empty list for default_model fallback"
        );
        // 仍需启动代理（后续 CLI 请求需要）
        let _info = start_spike_proxy(supplier_id, upstream).await?;
        return Ok(vec![]);
    }

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

    // ── normalize_path: /v1 前缀剥离 ──

    #[test]
    fn normalize_path_strips_v1_prefix() {
        // Anthropic 风格路径
        assert_eq!(normalize_path("/v1/messages"), "/messages");
        assert_eq!(normalize_path("/v1/models"), "/models");

        // OpenAI 风格路径
        assert_eq!(normalize_path("/v1/chat/completions"), "/chat/completions");

        // 不带 /v1 前缀的路径（保持原样）
        assert_eq!(normalize_path("/messages"), "/messages");
        assert_eq!(normalize_path("/models"), "/models");

        // 裸路径（无前置 /）
        assert_eq!(normalize_path("messages"), "/messages");

        // 仅 /v1
        assert_eq!(normalize_path("/v1"), "/");

        // 根路径
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn normalize_path_preserves_query_string() {
        assert_eq!(
            normalize_path("/v1/messages?stream=true"),
            "/messages?stream=true"
        );
        assert_eq!(normalize_path("/v1/models?limit=100"), "/models?limit=100");
    }

    // ── URL 拼接：无双拼 /v1/v1 ──

    #[test]
    fn url_construction_no_double_v1_anthropic() {
        let base = "https://api.anthropic.com";
        let root = openai_root(base);
        assert_eq!(root, "https://api.anthropic.com/v1");
        assert_eq!(
            format!("{}{}", root, normalize_path("/v1/messages")),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn url_construction_no_double_v1_deepseek() {
        let base = "https://api.deepseek.com";
        let root = openai_root(base);
        assert_eq!(root, "https://api.deepseek.com/v1");
        assert_eq!(
            format!("{}{}", root, normalize_path("/v1/chat/completions")),
            "https://api.deepseek.com/v1/chat/completions"
        );
        // Anthropic 路径也不会双拼
        assert_eq!(
            format!("{}{}", root, normalize_path("/v1/messages")),
            "https://api.deepseek.com/v1/messages"
        );
    }

    #[test]
    fn url_construction_no_double_v1_openrouter() {
        let base = "https://openrouter.ai/api";
        let root = openai_root(base);
        assert_eq!(root, "https://openrouter.ai/api/v1");
        assert_eq!(
            format!("{}{}", root, normalize_path("/v1/messages")),
            "https://openrouter.ai/api/v1/messages"
        );
    }

    #[test]
    fn url_construction_no_double_v1_zhipu() {
        let base = "https://open.bigmodel.cn/api/coding/paas/v4";
        let root = openai_root(base);
        assert_eq!(root, "https://open.bigmodel.cn/api/coding/paas/v4/v1");
        assert_eq!(
            format!("{}{}", root, normalize_path("/v1/chat/completions")),
            "https://open.bigmodel.cn/api/coding/paas/v4/v1/chat/completions"
        );
    }

    #[test]
    fn url_construction_no_double_v1_custom() {
        let base = "https://custom-api.example.com";
        let root = openai_root(base);
        assert_eq!(
            format!("{}{}", root, normalize_path("/v1/chat/completions")),
            "https://custom-api.example.com/v1/chat/completions"
        );
    }

    // ── is_streaming_request 测试 ──

    #[test]
    fn streaming_detected_from_json_body() {
        let body = br#"{"model":"claude-3","max_tokens":1024,"stream":true,"messages":[]}"#;
        let headers = HeaderMap::new();
        assert!(is_streaming_request(body, &headers));
    }

    #[test]
    fn streaming_not_detected_when_stream_false() {
        let body = br#"{"model":"claude-3","stream":false,"messages":[]}"#;
        let headers = HeaderMap::new();
        assert!(!is_streaming_request(body, &headers));
    }

    #[test]
    fn streaming_detected_from_accept_header() {
        let body = b"{}";
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ACCEPT,
            "text/event-stream".parse().unwrap(),
        );
        assert!(is_streaming_request(body, &headers));
    }

    #[test]
    fn streaming_not_detected_for_plain_json() {
        let body = br#"{"model":"gpt-4","messages":[]}"#;
        let headers = HeaderMap::new();
        assert!(!is_streaming_request(body, &headers));
    }
}
