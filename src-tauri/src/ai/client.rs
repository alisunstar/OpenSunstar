//! 统一 AI 客户端
//!
//! 通过 OpenAI 兼容 API 调用 DeepSeek / GLM / Custom 模型。
//! 复用全局 HTTP 客户端（proxy::http_client），自动继承代理配置。

use super::types::{AIProviderConfig, ChatMessage, ChatResponse};
use crate::proxy::http_client;
use std::time::Duration;

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
        Self::chat_completion_with_timeout(config, messages, max_tokens, Duration::from_secs(30))
            .await
    }

    /// 支持长任务自定义超时的非流式 chat completion。
    /// Wiki 等代码库级生成任务会显著长于普通洞察请求。
    pub async fn chat_completion_with_timeout(
        config: &AIProviderConfig,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
        timeout: Duration,
    ) -> Result<ChatResponse, String> {
        Self::chat_completion_with_timeout_and_format(config, messages, max_tokens, timeout, false)
            .await
    }

    /// 请求 OpenAI-compatible JSON Object 输出。
    ///
    /// 仅供已经确认支持 `response_format: { type: "json_object" }` 的 Provider 使用；
    /// 普通自定义 Provider 继续走 [`chat_completion_with_timeout`]，避免兼容性回退。
    pub async fn chat_completion_json_with_timeout(
        config: &AIProviderConfig,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
        timeout: Duration,
    ) -> Result<ChatResponse, String> {
        Self::chat_completion_with_timeout_and_format(config, messages, max_tokens, timeout, true)
            .await
    }

    async fn chat_completion_with_timeout_and_format(
        config: &AIProviderConfig,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
        timeout: Duration,
        json_mode: bool,
    ) -> Result<ChatResponse, String> {
        let client = http_client::get();

        let body = build_chat_completion_body(config, &messages, max_tokens, json_mode);

        let response = client
            .post(&config.api_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", config.api_key))
            .timeout(timeout)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    format!("AI 请求超时（{} 秒），请稍后重试", timeout.as_secs())
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

fn build_chat_completion_body(
    config: &AIProviderConfig,
    messages: &[ChatMessage],
    max_tokens: Option<u32>,
    json_mode: bool,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "stream": false,
    });
    if let Some(max) = max_tokens {
        body["max_tokens"] = serde_json::json!(max);
    }
    if json_mode {
        body["response_format"] = serde_json::json!({ "type": "json_object" });
    }
    body
}

/// USD → CNY 汇率。
///
/// 硬编码的近似值。真正可配置的汇率属于 `model_pricing` 表的范畴（尚未落地），
/// 在那之前至少让它只有一处，而不是散落在每条定价里。
const USD_TO_CNY: f64 = 7.2;

/// 未命中定价表时的兜底单价（CNY / 1M tokens），沿用 DeepSeek 档位。
///
/// 兜底不是「按 DeepSeek 计费」，而是「先记一个数，免得账面变成 0」。
/// 它必须配合 `ModelPricing::known == false` 使用，界面据此标注「单价未知」。
const FALLBACK_INPUT_CNY_PER_M: f64 = 2.0;
const FALLBACK_OUTPUT_CNY_PER_M: f64 = 8.0;

/// 定价表：(模型名片段, 输入单价, 输出单价)，单位 CNY / 1M tokens。
///
/// **匹配规则是「命中的最长片段获胜」，不是「表里靠前的先赢」。**
/// 旧实现用 `match` 守卫按书写顺序求值，`gpt-4o` 写在 `gpt-4o-mini` 前面，
/// 而 `"gpt-4o-mini".contains("gpt-4o")` 为真 —— mini 那条分支永远不可达，
/// 一直按 16.7 倍价格计费。顺序无关的匹配让这类遮蔽在结构上不可能再发生
/// （`no_entry_is_shadowed_by_a_shorter_key` 守住这条不变量）。
///
/// 片段一律小写；`lookup_model_pricing` 会先把模型名小写化，因此不必再为
/// `GLM-4` / `glm-4` 各写一行。
///
/// 匹配是按「模型家族」而非精确版本 —— `glm-4.6` 会命中 `glm-4`。这是刻意的
/// 近似：宁可给同族的新版本一个邻近价，也好过整族退化成「单价未知」。
const MODEL_PRICING_TABLE: &[(&str, f64, f64)] = &[
    // DeepSeek 系列（官方定价即 CNY）
    ("deepseek-chat", 2.0, 8.0),
    ("deepseek-v3", 2.0, 8.0),
    ("deepseek-reasoner", 4.0, 16.0),
    ("deepseek-r1", 4.0, 16.0),
    // 智谱 GLM 系列（官方定价即 CNY）
    ("glm-3", 1.0, 1.0),
    ("glm-4", 10.0, 10.0),
    ("glm-5", 15.0, 15.0),
    // OpenAI 系列（USD 定价，折算 CNY）
    ("gpt-3.5", 0.5 * USD_TO_CNY, 1.5 * USD_TO_CNY),
    ("gpt-4o", 2.5 * USD_TO_CNY, 10.0 * USD_TO_CNY),
    ("gpt-4o-mini", 0.15 * USD_TO_CNY, 0.6 * USD_TO_CNY),
    ("gpt-4-turbo", 10.0 * USD_TO_CNY, 30.0 * USD_TO_CNY),
    ("gpt-4-0125", 10.0 * USD_TO_CNY, 30.0 * USD_TO_CNY),
    // Anthropic Claude 系列（USD 定价，折算 CNY）
    ("claude-3-5-sonnet", 3.0 * USD_TO_CNY, 15.0 * USD_TO_CNY),
    ("claude-3.5-sonnet", 3.0 * USD_TO_CNY, 15.0 * USD_TO_CNY),
    ("claude-3-5-haiku", 0.8 * USD_TO_CNY, 4.0 * USD_TO_CNY),
    ("claude-3.5-haiku", 0.8 * USD_TO_CNY, 4.0 * USD_TO_CNY),
];

/// 单模型定价（CNY / 1M tokens）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    pub input_cny_per_m: f64,
    pub output_cny_per_m: f64,
    /// 是否命中定价表。
    ///
    /// `false` 表示这是兜底猜测。界面必须据此标注「单价未知」—— 把猜出来的
    /// 数字当账单展示，比不展示更糟。
    pub known: bool,
}

/// 查模型单价。未命中定价表时返回兜底价并置 `known: false`。
pub fn lookup_model_pricing(model: &str) -> ModelPricing {
    let needle = model.to_ascii_lowercase();
    let best = MODEL_PRICING_TABLE
        .iter()
        .filter(|(key, _, _)| needle.contains(key))
        .max_by_key(|(key, _, _)| key.len());

    match best {
        Some(&(_, input_cny_per_m, output_cny_per_m)) => ModelPricing {
            input_cny_per_m,
            output_cny_per_m,
            known: true,
        },
        None => ModelPricing {
            input_cny_per_m: FALLBACK_INPUT_CNY_PER_M,
            output_cny_per_m: FALLBACK_OUTPUT_CNY_PER_M,
            known: false,
        },
    }
}

/// 该模型是否有可信单价。`false` 时界面应显示「单价未知」而非一个精确数字。
pub fn is_model_pricing_known(model: &str) -> bool {
    lookup_model_pricing(model).known
}

/// 根据模型名称估算调用成本（CNY）
///
/// 注意返回值恒为一个数字，即便单价未知 —— 判断可信度请用
/// [`is_model_pricing_known`]，不要从金额本身反推。
pub fn estimate_cost(model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
    let pricing = lookup_model_pricing(model);
    let input_cost = (prompt_tokens as f64 / 1_000_000.0) * pricing.input_cny_per_m;
    let output_cost = (completion_tokens as f64 / 1_000_000.0) * pricing.output_cny_per_m;
    input_cost + output_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_mode_request_sets_response_format_without_changing_messages() {
        let config = AIProviderConfig {
            provider: "deepseek".to_string(),
            api_key: "test-key".to_string(),
            api_url: "https://api.deepseek.com/v1/chat/completions".to_string(),
            model: "deepseek-chat".to_string(),
        };
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Return json".to_string(),
        }];

        let body = build_chat_completion_body(&config, &messages, Some(8_000), true);

        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["messages"][0]["content"], "Return json");
        assert_eq!(body["max_tokens"], 8_000);
    }

    /// 1M 输入 + 1M 输出，方便直接读出「每百万 token 单价」。
    fn cost_1m(model: &str) -> (f64, f64) {
        let input = estimate_cost(model, 1_000_000, 0);
        let output = estimate_cost(model, 0, 1_000_000);
        (input, output)
    }

    #[test]
    fn gpt_4o_mini_must_not_be_priced_as_gpt_4o() {
        // `"gpt-4o-mini".contains("gpt-4o")` 为真，match 自上而下匹配，
        // 于是 mini 的那条分支永远不可达 —— mini 一直按 gpt-4o 计价。
        let (mini_in, mini_out) = cost_1m("gpt-4o-mini");
        let (full_in, full_out) = cost_1m("gpt-4o");
        assert!(
            mini_in < full_in,
            "gpt-4o-mini 输入单价({mini_in}) 必须低于 gpt-4o({full_in})"
        );
        assert!(
            mini_out < full_out,
            "gpt-4o-mini 输出单价({mini_out}) 必须低于 gpt-4o({full_out})"
        );
    }

    #[test]
    fn no_entry_is_shadowed_by_a_shorter_key() {
        // 上面那个 bug 的通用形式：只要表里存在 A ⊂ B 两个片段，而匹配又依赖
        // 书写顺序，B 就可能永远拿不到自己的价。这条测试守住「最长命中获胜」
        // 这个不变量 —— 每个片段拿自己名字去查，必须查回自己的价。
        for &(key, input, output) in MODEL_PRICING_TABLE {
            let pricing = lookup_model_pricing(key);
            assert!(pricing.known, "片段 {key} 必须命中定价表");
            assert_eq!(
                (pricing.input_cny_per_m, pricing.output_cny_per_m),
                (input, output),
                "片段 {key} 被其它条目遮蔽了"
            );
        }
    }

    #[test]
    fn known_models_are_flagged_known() {
        for model in [
            "deepseek-chat",
            "DeepSeek-V3",
            "glm-4-plus",
            "GLM-4.6",
            "gpt-4o-2024-11-20",
            "claude-3-5-sonnet-20241022",
        ] {
            assert!(is_model_pricing_known(model), "{model} 应命中定价表");
        }
    }

    #[test]
    fn unknown_model_is_flagged_unknown_instead_of_silently_priced_as_deepseek() {
        // 旧实现的 `_ => (2.0, 8.0)` 把任何不认识的模型静默按 DeepSeek 计价，
        // 用户看到的是一个精确到小数点的数字，底层却是「我不认识它，姑且按
        // 最便宜的算」。金额可以继续兜底，但必须带着「不可信」这个标记出来。
        let pricing = lookup_model_pricing("some-brand-new-model-2027");
        assert!(!pricing.known, "未知模型必须标记为单价未知");
        assert_eq!(pricing.input_cny_per_m, FALLBACK_INPUT_CNY_PER_M);

        // 兜底仍要出数：0 会让账面显示「没花钱」，那是另一种谎。
        assert!(estimate_cost("some-brand-new-model-2027", 1_000_000, 0) > 0.0);
    }

    #[test]
    fn cost_scales_linearly_with_tokens() {
        let one = estimate_cost("deepseek-chat", 1_000_000, 1_000_000);
        let two = estimate_cost("deepseek-chat", 2_000_000, 2_000_000);
        assert!((two - one * 2.0).abs() < 1e-9);
        assert_eq!(estimate_cost("deepseek-chat", 0, 0), 0.0);
    }

    #[test]
    fn model_name_matching_is_case_insensitive() {
        assert_eq!(
            lookup_model_pricing("GLM-4-Plus"),
            lookup_model_pricing("glm-4-plus"),
        );
    }

    #[test]
    fn truncate_error_keeps_short_messages_intact() {
        assert_eq!(truncate_error("短"), "短");
    }

    #[test]
    fn truncate_error_is_safe_for_long_unicode_messages() {
        let message = "错误".repeat(400);
        let truncated = truncate_error(&message);
        assert!(truncated.ends_with("...(truncated)"));
        assert!(truncated.chars().count() <= 500 + "...(truncated)".chars().count());
    }
}

/// 截断错误信息，避免日志过长
fn truncate_error(s: &str) -> String {
    if s.chars().count() > 500 {
        format!("{}...(truncated)", s.chars().take(500).collect::<String>())
    } else {
        s.to_string()
    }
}
