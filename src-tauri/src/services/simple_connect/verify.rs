//! API Key 校验（分协议校验，Phase 2 P1）

use crate::error::AppError;
use crate::services::simple_connect::suppliers::{resolve_protocol, resolve_supplier, ApiProtocol};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VerifyKeyResult {
    pub ok: bool,
    pub model_count: usize,
    pub error: Option<String>,
}

fn openai_v1_root(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

fn humanize_status(status: u16) -> String {
    match status {
        401 => "密钥无效或已过期 (401)".into(),
        402 => "余额不足 (402)".into(),
        403 => "访问被拒绝 (403)".into(),
        404 => "接口不存在，请检查上游地址 (404)".into(),
        429 => "请求过于频繁 (429)".into(),
        500 => "上游服务器错误 (500)".into(),
        502 => "上游网关错误 (502)".into(),
        503 => "上游服务不可用 (503)".into(),
        _ => format!("上游返回错误 ({status})"),
    }
}

pub async fn verify_api_key(
    supplier_id: &str,
    secret: &str,
    custom_base: Option<&str>,
) -> Result<VerifyKeyResult, AppError> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Ok(VerifyKeyResult {
            ok: false,
            model_count: 0,
            error: Some("API Key 不能为空".into()),
        });
    }

    let supplier = resolve_supplier(supplier_id, custom_base)
        .ok_or_else(|| AppError::Message(format!("未知供应商: {supplier_id}")))?;
    let protocol = resolve_protocol(supplier_id);

    match protocol {
        ApiProtocol::Anthropic => verify_anthropic_key(&supplier, trimmed).await,
        ApiProtocol::OpenAi => verify_openai_key(&supplier, trimmed).await,
    }
}

/// OpenAI 兼容协议校验：GET {base}/v1/models + Bearer Auth
async fn verify_openai_key(
    supplier: &crate::services::simple_connect::suppliers::SupplierProfile,
    api_key: &str,
) -> Result<VerifyKeyResult, AppError> {
    let url = format!("{}/models", openai_v1_root(&supplier.openai_base));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| AppError::Message(format!("HTTP client: {e}")))?;

    let resp = match client.get(&url).bearer_auth(api_key).send().await {
        Ok(r) => r,
        Err(e) => {
            let msg = if e.is_timeout() {
                "连接超时，请检查网络".into()
            } else if e.is_connect() {
                "无法连接到上游服务器".into()
            } else {
                format!("网络错误: {e}")
            };
            return Ok(VerifyKeyResult {
                ok: false,
                model_count: 0,
                error: Some(msg),
            });
        }
    };

    if !resp.status().is_success() {
        return Ok(VerifyKeyResult {
            ok: false,
            model_count: 0,
            error: Some(humanize_status(resp.status().as_u16())),
        });
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| AppError::Message("上游响应格式异常".into()))?;
    let count = v
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(VerifyKeyResult {
        ok: true,
        model_count: count,
        error: None,
    })
}

/// Anthropic 原生协议校验：POST {base}/v1/messages 最小请求
///
/// Anthropic 没有 /v1/models 端点，使用最小 messages 请求验证 Key 有效性。
/// 注意：此校验会消耗少量 token（max_tokens=1）。
async fn verify_anthropic_key(
    supplier: &crate::services::simple_connect::suppliers::SupplierProfile,
    api_key: &str,
) -> Result<VerifyKeyResult, AppError> {
    let base = supplier
        .anthropic_base
        .as_deref()
        .unwrap_or(&supplier.openai_base);
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Message(format!("HTTP client: {e}")))?;

    let body = serde_json::json!({
        "model": &supplier.default_model,
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "hi"}]
    });

    let resp = match client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = if e.is_timeout() {
                "连接超时，请检查网络".into()
            } else if e.is_connect() {
                "无法连接到 Anthropic 服务器".into()
            } else {
                format!("网络错误: {e}")
            };
            return Ok(VerifyKeyResult {
                ok: false,
                model_count: 0,
                error: Some(msg),
            });
        }
    };

    let status = resp.status();
    match status.as_u16() {
        200 => Ok(VerifyKeyResult {
            ok: true,
            model_count: 0, // Anthropic 不提供模型列表
            error: None,
        }),
        401 => Ok(VerifyKeyResult {
            ok: false,
            model_count: 0,
            error: Some("Key 无效或已过期，请到 Anthropic Console 重新申请".into()),
        }),
        403 => Ok(VerifyKeyResult {
            ok: false,
            model_count: 0,
            error: Some("Key 无权限，请检查 Anthropic Console 中的 API 访问设置".into()),
        }),
        429 => {
            // 429 = Key 有效但限速，视为有效
            Ok(VerifyKeyResult {
                ok: true,
                model_count: 0,
                error: Some("Key 有效，但当前限速中，稍后可正常使用".into()),
            })
        }
        other => Ok(VerifyKeyResult {
            ok: false,
            model_count: 0,
            error: Some(format!(
                "Anthropic API 返回 HTTP {}：{}",
                other,
                humanize_status(other)
            )),
        }),
    }
}
