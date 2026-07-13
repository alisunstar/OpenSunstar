//! 统一 AI 客户端
//!
//! 通过 OpenAI 兼容 API 调用 DeepSeek / GLM / Custom 模型。
//! 复用全局 HTTP 客户端（proxy::http_client），自动继承代理配置。

use super::types::{AIProviderConfig, ChatMessage, ChatResponse};
use crate::proxy::http_client;

/// AI 客户端 — 无状态，所有配置通过参数传入
pub struct AIClient;

impl AIClient {
    /// 非流式 chat completion（OpenAI 兼容格式）
    ///
    /// # 参数
    /// - `config`: AI 提供方配置（key/url/model）
    /// - `messages`: 对话消息列表
    /// - `max_tokens`: 最大生成 token 数（可选）
    ///
    /// # 返回
    /// 解析后的 ChatResponse，包含 choices 和 usage
    pub async fn chat_completion(
        config: &AIProviderConfig,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse, String> {
        let client = http_client::get();

        // 构建请求体
        let mut body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "stream": false,
        });
        if let Some(max) = max_tokens {
            body["max_tokens"] = serde_json::json!(max);
        }

        // 设置较短超时（洞察生成不需要太长）
        let response = client
            .post(&config.api_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .timeout(std::time::Duration::from_secs(30))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "AI 请求超时（30 秒），请稍后重试".to_string()
                } else if e.is_connect() {
                    format!("AI 服务连接失败: {e}")
                } else {
                    format!("AI 请求失败: {e}")
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!(
                "AI API 返回错误 {}: {}",
                status.as_u16(),
                truncate_error(&error_body)
            ));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("AI 响应解析失败: {e}"))?;

        if chat_response.choices.is_empty() {
            return Err("AI 返回了空的响应".to_string());
        }

        Ok(chat_response)
    }
}

/// 根据模型名称估算调用成本（CNY）
///
/// 价格基于 2024 年公开定价，Phase 2 可接入 model_pricing 表
pub fn estimate_cost(model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
    let (input_price, output_price) = match model {
        // DeepSeek 系列 (CNY / 1M tokens)
        m if m.contains("deepseek-chat") || m.contains("deepseek-v3") => (2.0, 8.0),
        m if m.contains("deepseek-reasoner") || m.contains("deepseek-r1") => (4.0, 16.0),
        // GLM 系列 (CNY / 1M tokens)
        m if m.contains("glm-4") || m.contains("GLM-4") => (10.0, 10.0),
        m if m.contains("glm-5") || m.contains("GLM-5") => (15.0, 15.0),
        m if m.contains("glm-3") || m.contains("GLM-3") => (1.0, 1.0),
        // OpenAI 系列 (USD / 1M tokens, 按 7.2 汇率折算 CNY)
        m if m.contains("gpt-4o") => (2.5 * 7.2, 10.0 * 7.2),
        m if m.contains("gpt-4o-mini") => (0.15 * 7.2, 0.6 * 7.2),
        m if m.contains("gpt-4-turbo") || m.contains("gpt-4-0125") => (10.0 * 7.2, 30.0 * 7.2),
        m if m.contains("gpt-3.5") => (0.5 * 7.2, 1.5 * 7.2),
        // Claude 系列 (USD / 1M tokens)
        m if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") => {
            (3.0 * 7.2, 15.0 * 7.2)
        }
        m if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") => {
            (0.8 * 7.2, 4.0 * 7.2)
        }
        // 默认: 按 DeepSeek 价格估算
        _ => (2.0, 8.0),
    };

    let input_cost = (prompt_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (completion_tokens as f64 / 1_000_000.0) * output_price;
    input_cost + output_cost
}

/// 截断错误信息，避免日志过长
fn truncate_error(s: &str) -> String {
    if s.len() > 500 {
        format!("{}...(truncated)", &s[..500])
    } else {
        s.to_string()
    }
}
