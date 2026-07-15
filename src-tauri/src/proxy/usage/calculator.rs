//! Cost Calculator - 计算 API 请求成本
//!
//! 使用高精度 Decimal 类型避免浮点数精度问题

use super::parser::TokenUsage;
use rust_decimal::Decimal;
use std::str::FromStr;

/// 成本明细
#[derive(Debug, Clone)]
pub struct CostBreakdown {
    pub input_cost: Decimal,
    pub output_cost: Decimal,
    pub cache_read_cost: Decimal,
    pub cache_creation_cost: Decimal,
    pub total_cost: Decimal,
}

/// 模型定价信息
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_million: Decimal,
    pub output_cost_per_million: Decimal,
    pub cache_read_cost_per_million: Decimal,
    pub cache_creation_cost_per_million: Decimal,
    pub long_context_surcharge: Option<LongContextSurcharge>,
}

/// 对超过指定原始输入长度的整次请求应用的公开阶梯费率。
///
/// 缓存读写保留各自独立价格；上游若公布不同的阶梯缓存规则，应通过新的
/// 费率版本更新，而不是隐式复用输入/输出倍率。
#[derive(Debug, Clone)]
pub struct LongContextSurcharge {
    pub threshold_tokens: u32,
    pub input_multiplier: Decimal,
    pub output_multiplier: Decimal,
}

/// 成本计算器
pub struct CostCalculator;

impl CostCalculator {
    /// 计算请求成本
    ///
    /// # 参数
    /// - `usage`: Token 使用量
    /// - `pricing`: 模型定价
    /// - `cost_multiplier`: 成本倍数 (provider 自定义)
    ///
    /// # 计算逻辑
    /// - input_cost: input_tokens × 输入价格
    /// - cache_read_cost: cache_read_tokens × 缓存读取价格
    /// - Claude/Anthropic 的 input_tokens 已经不包含 cache_read_tokens
    /// - total_cost: 各项成本之和 × 倍率（倍率只作用于最终总价）
    pub fn calculate(
        usage: &TokenUsage,
        pricing: &ModelPricing,
        cost_multiplier: Decimal,
    ) -> CostBreakdown {
        Self::calculate_with_cache_semantics(usage, pricing, cost_multiplier, false)
    }

    /// 按 app_type 选择输入 token 语义后计算成本。
    ///
    /// Codex/OpenAI Responses 与 Gemini 的输入 token 字段包含 cache read 部分；
    /// Claude/Anthropic 的 input_tokens 已经是 fresh input。
    pub fn calculate_for_app(
        app_type: &str,
        usage: &TokenUsage,
        pricing: &ModelPricing,
        cost_multiplier: Decimal,
    ) -> CostBreakdown {
        let input_includes_cache_read = matches!(app_type, "codex" | "gemini");
        Self::calculate_with_cache_semantics(
            usage,
            pricing,
            cost_multiplier,
            input_includes_cache_read,
        )
    }

    fn calculate_with_cache_semantics(
        usage: &TokenUsage,
        pricing: &ModelPricing,
        cost_multiplier: Decimal,
        input_includes_cache_read: bool,
    ) -> CostBreakdown {
        let million = Decimal::from(1_000_000);

        // OpenAI/Gemini 风格的 input_tokens 包含缓存命中，需要扣除后再按输入价计费；
        // Claude/Anthropic 风格的 input_tokens 已经是 fresh input，不能再次扣减。
        let billable_input_tokens = if input_includes_cache_read {
            usage.input_tokens.saturating_sub(usage.cache_read_tokens)
        } else {
            usage.input_tokens
        };

        // 各项基础成本（不含倍率）
        let long_context_surcharge = pricing
            .long_context_surcharge
            .as_ref()
            .filter(|surcharge| usage.input_tokens > surcharge.threshold_tokens);
        let input_multiplier = long_context_surcharge
            .map(|surcharge| surcharge.input_multiplier)
            .unwrap_or(Decimal::ONE);
        let output_multiplier = long_context_surcharge
            .map(|surcharge| surcharge.output_multiplier)
            .unwrap_or(Decimal::ONE);

        let input_cost = Decimal::from(billable_input_tokens)
            * pricing.input_cost_per_million
            * input_multiplier
            / million;
        let output_cost = Decimal::from(usage.output_tokens)
            * pricing.output_cost_per_million
            * output_multiplier
            / million;
        let cache_read_cost =
            Decimal::from(usage.cache_read_tokens) * pricing.cache_read_cost_per_million / million;
        let cache_creation_cost = Decimal::from(usage.cache_creation_tokens)
            * pricing.cache_creation_cost_per_million
            / million;

        // 总成本 = 各项基础成本之和 × 倍率
        let base_total = input_cost + output_cost + cache_read_cost + cache_creation_cost;
        let total_cost = base_total * cost_multiplier;

        CostBreakdown {
            input_cost,
            output_cost,
            cache_read_cost,
            cache_creation_cost,
            total_cost,
        }
    }

    /// 尝试计算成本，如果模型未知则返回 None
    #[allow(dead_code)]
    pub fn try_calculate(
        usage: &TokenUsage,
        pricing: Option<&ModelPricing>,
        cost_multiplier: Decimal,
    ) -> Option<CostBreakdown> {
        pricing.map(|p| Self::calculate(usage, p, cost_multiplier))
    }

    pub fn try_calculate_for_app(
        app_type: &str,
        usage: &TokenUsage,
        pricing: Option<&ModelPricing>,
        cost_multiplier: Decimal,
    ) -> Option<CostBreakdown> {
        pricing.map(|p| Self::calculate_for_app(app_type, usage, p, cost_multiplier))
    }
}

impl ModelPricing {
    /// 从字符串创建定价信息
    pub fn from_strings(
        input: &str,
        output: &str,
        cache_read: &str,
        cache_creation: &str,
    ) -> Result<Self, rust_decimal::Error> {
        Ok(Self {
            input_cost_per_million: Decimal::from_str(input)?,
            output_cost_per_million: Decimal::from_str(output)?,
            cache_read_cost_per_million: Decimal::from_str(cache_read)?,
            cache_creation_cost_per_million: Decimal::from_str(cache_creation)?,
            long_context_surcharge: None,
        })
    }

    pub fn from_strings_with_long_context_surcharge(
        input: &str,
        output: &str,
        cache_read: &str,
        cache_creation: &str,
        threshold_tokens: u32,
        long_context_input_multiplier: &str,
        long_context_output_multiplier: &str,
    ) -> Result<Self, rust_decimal::Error> {
        let mut pricing = Self::from_strings(input, output, cache_read, cache_creation)?;
        pricing.long_context_surcharge = Some(LongContextSurcharge {
            threshold_tokens,
            input_multiplier: Decimal::from_str(long_context_input_multiplier)?,
            output_multiplier: Decimal::from_str(long_context_output_multiplier)?,
        });
        Ok(pricing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_creation_tokens: 100,
            model: None,
            message_id: None,
        };

        let pricing = ModelPricing::from_strings("3.0", "15.0", "0.3", "3.75").unwrap();
        let multiplier = Decimal::from_str("1.0").unwrap();

        let cost = CostCalculator::calculate(&usage, &pricing, multiplier);

        // Claude/Anthropic 语义：input_tokens 已经不含 cache_read_tokens
        // input: 1000 * 3.0 / 1M = 0.003
        assert_eq!(cost.input_cost, Decimal::from_str("0.003").unwrap());
        // output: 500 * 15.0 / 1M = 0.0075
        assert_eq!(cost.output_cost, Decimal::from_str("0.0075").unwrap());
        // cache_read: 200 * 0.3 / 1M = 0.00006
        assert_eq!(cost.cache_read_cost, Decimal::from_str("0.00006").unwrap());
        // cache_creation: 100 * 3.75 / 1M = 0.000375
        assert_eq!(
            cost.cache_creation_cost,
            Decimal::from_str("0.000375").unwrap()
        );
        // total: 0.003 + 0.0075 + 0.00006 + 0.000375 = 0.010935
        assert_eq!(cost.total_cost, Decimal::from_str("0.010935").unwrap());
    }

    #[test]
    fn test_cost_calculation_for_cache_inclusive_app() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_creation_tokens: 100,
            model: None,
            message_id: None,
        };

        let pricing = ModelPricing::from_strings("3.0", "15.0", "0.3", "3.75").unwrap();
        let multiplier = Decimal::from_str("1.0").unwrap();

        let cost = CostCalculator::calculate_for_app("codex", &usage, &pricing, multiplier);

        // Codex/OpenAI 语义：input_tokens 包含 cached_tokens，需要扣除 cache_read_tokens
        assert_eq!(cost.input_cost, Decimal::from_str("0.0024").unwrap());
        assert_eq!(cost.output_cost, Decimal::from_str("0.0075").unwrap());
        assert_eq!(cost.cache_read_cost, Decimal::from_str("0.00006").unwrap());
        assert_eq!(
            cost.cache_creation_cost,
            Decimal::from_str("0.000375").unwrap()
        );
        assert_eq!(cost.total_cost, Decimal::from_str("0.010335").unwrap());
    }

    #[test]
    fn gpt_5_6_long_context_surcharge_applies_to_full_request() {
        let usage = TokenUsage {
            input_tokens: 272_001,
            output_tokens: 10_000,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: Some("gpt-5.6-sol".to_string()),
            message_id: None,
        };
        let pricing = ModelPricing::from_strings_with_long_context_surcharge(
            "5", "30", "0.5", "6.25", 272_000, "2", "1.5",
        )
        .unwrap();

        let cost = CostCalculator::calculate_for_app("codex", &usage, &pricing, Decimal::ONE);

        assert_eq!(cost.input_cost, Decimal::from_str("2.72001").unwrap());
        assert_eq!(cost.output_cost, Decimal::from_str("0.45").unwrap());
        assert_eq!(cost.total_cost, Decimal::from_str("3.17001").unwrap());
    }

    #[test]
    fn gpt_5_6_long_context_surcharge_does_not_apply_at_threshold() {
        let usage = TokenUsage {
            input_tokens: 272_000,
            output_tokens: 10_000,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: Some("gpt-5.6-sol".to_string()),
            message_id: None,
        };
        let pricing = ModelPricing::from_strings_with_long_context_surcharge(
            "5", "30", "0.5", "6.25", 272_000, "2", "1.5",
        )
        .unwrap();

        let cost = CostCalculator::calculate_for_app("codex", &usage, &pricing, Decimal::ONE);

        assert_eq!(cost.input_cost, Decimal::from_str("1.36").unwrap());
        assert_eq!(cost.output_cost, Decimal::from_str("0.3").unwrap());
    }

    #[test]
    fn test_cost_multiplier() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: None,
            message_id: None,
        };

        let pricing = ModelPricing::from_strings("3.0", "15.0", "0", "0").unwrap();
        let multiplier = Decimal::from_str("1.5").unwrap();

        let cost = CostCalculator::calculate(&usage, &pricing, multiplier);

        // input_cost: 基础价格（不含倍率）= 1000 * 3.0 / 1M = 0.003
        assert_eq!(cost.input_cost, Decimal::from_str("0.003").unwrap());
        // total_cost: 基础价格 × 倍率 = 0.003 * 1.5 = 0.0045
        assert_eq!(cost.total_cost, Decimal::from_str("0.0045").unwrap());
    }

    #[test]
    fn test_unknown_model_handling() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: None,
            message_id: None,
        };

        let multiplier = Decimal::from_str("1.0").unwrap();
        let cost = CostCalculator::try_calculate(&usage, None, multiplier);

        assert!(cost.is_none());
    }

    #[test]
    fn test_decimal_precision() {
        let usage = TokenUsage {
            input_tokens: 1,
            output_tokens: 1,
            cache_read_tokens: 1,
            cache_creation_tokens: 1,
            model: None,
            message_id: None,
        };

        let pricing = ModelPricing::from_strings("0.075", "0.3", "0.01875", "0.075").unwrap();
        let multiplier = Decimal::from_str("1.0").unwrap();

        let cost = CostCalculator::calculate(&usage, &pricing, multiplier);

        // 验证高精度计算
        assert!(cost.total_cost > Decimal::ZERO);
        assert!(cost.total_cost.to_string().len() > 2); // 确保保留了小数位
    }
}
