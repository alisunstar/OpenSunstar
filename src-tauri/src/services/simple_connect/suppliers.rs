//! Simple Connect 预设供应商（评审决议：不含 BeeAPI 默认）

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierProfile {
    pub id: String,
    pub name: String,
    /// OpenAI-compatible root, e.g. https://api.deepseek.com
    pub openai_base: String,
    /// Anthropic-compatible root for Claude Code (optional)
    pub anthropic_base: Option<String>,
    pub default_model: String,
    pub website: Option<String>,
}

pub fn list_builtin_suppliers() -> Vec<SupplierProfile> {
    vec![
        SupplierProfile {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            openai_base: "https://api.deepseek.com".into(),
            anthropic_base: Some("https://api.deepseek.com/anthropic".into()),
            default_model: "deepseek-chat".into(),
            website: Some("https://platform.deepseek.com".into()),
        },
        SupplierProfile {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            openai_base: "https://openrouter.ai/api".into(),
            anthropic_base: Some("https://openrouter.ai/api".into()),
            default_model: "anthropic/claude-3.5-sonnet".into(),
            website: Some("https://openrouter.ai".into()),
        },
        SupplierProfile {
            id: "zhipu".into(),
            name: "智谱 GLM".into(),
            openai_base: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            anthropic_base: None,
            default_model: "glm-4-flash".into(),
            website: Some("https://open.bigmodel.cn".into()),
        },
        SupplierProfile {
            id: "anthropic".into(),
            name: "Anthropic 官方".into(),
            openai_base: "https://api.anthropic.com".into(),
            anthropic_base: Some("https://api.anthropic.com".into()),
            default_model: "claude-sonnet-4-20250514".into(),
            website: Some("https://console.anthropic.com".into()),
        },
    ]
}

pub fn get_supplier(id: &str) -> Option<SupplierProfile> {
    list_builtin_suppliers()
        .into_iter()
        .find(|s| s.id == id)
}

pub fn resolve_supplier(id: &str, custom_openai_base: Option<&str>) -> Option<SupplierProfile> {
    if id == "custom" {
        let base = custom_openai_base
            .map(str::trim)
            .filter(|s| !s.is_empty())?;
        return Some(SupplierProfile {
            id: "custom".into(),
            name: "自定义 OpenAI 兼容".into(),
            openai_base: base.trim_end_matches('/').to_string(),
            anthropic_base: Some(base.trim_end_matches('/').to_string()),
            default_model: String::new(),
            website: None,
        });
    }
    get_supplier(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_excludes_beeapi_and_matches_resolution() {
        let list = list_builtin_suppliers();
        assert_eq!(list.len(), 4);
        assert!(
            list.iter()
                .all(|s| s.id != "beeapi" && !s.openai_base.contains("beeapi.ai"))
        );
        assert!(list.iter().any(|s| s.id == "deepseek"));
        assert!(list.iter().any(|s| s.id == "openrouter"));
        assert!(list.iter().any(|s| s.id == "zhipu"));
        assert!(list.iter().any(|s| s.id == "anthropic"));
    }

    #[test]
    fn deepseek_has_anthropic_base() {
        let ds = get_supplier("deepseek").unwrap();
        assert!(ds.anthropic_base.is_some());
    }

    #[test]
    fn custom_supplier_resolves_base() {
        let custom = resolve_supplier("custom", Some("https://api.example.com/v1")).unwrap();
        assert_eq!(custom.openai_base, "https://api.example.com/v1");
    }
}
