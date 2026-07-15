//! 8 类资产 × 目标 CLI 能力矩阵（与前端 `assetAppSupport.ts` 一致）
//!
//! 就绪度评分：unsupported 且未配置时不计为缺口；partial 保留缺口但标注部分支持。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetSupport {
    Supported,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCapabilityDescriptor {
    pub support: String,
    pub write_mode: String,
    pub verify_modes: Vec<String>,
    pub limitations: Vec<String>,
    pub adapter_id: String,
    pub fixture_id: String,
}

/// 优先支持的 CLI（Claude / Codex / Gemini / OpenCode）
pub const PRIORITY_APPS: &[&str] = &["claude", "codex", "gemini", "opencode"];

/// 就绪度检查项 → 资产类型
pub fn check_name_to_asset_type(check_name: &str) -> Option<&'static str> {
    match check_name {
        "mcp_enabled" => Some("mcp"),
        "skills_configured" => Some("skill"),
        "prompt_files" => Some("prompt"),
        "commands_configured" => Some("command"),
        "hooks_configured" => Some("hook"),
        "ignore_rules" => Some("ignore"),
        "permissions" => Some("permission"),
        "subagents_configured" => Some("subagent"),
        _ => None,
    }
}

/// 归一化看板传入的目标 CLI（claude-desktop 按 Claude Code 计分）
pub fn normalize_target_app(app: Option<&str>) -> &'static str {
    match app {
        Some("claude-desktop") | None | Some("") => "claude",
        Some("codex") => "codex",
        Some("gemini") => "gemini",
        Some("opencode") => "opencode",
        Some("openclaw") => "openclaw",
        Some("hermes") => "hermes",
        Some(other) if other == "claude" => "claude",
        _ => "claude",
    }
}

pub fn app_display_label(app: &str) -> &'static str {
    match app {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "opencode" => "OpenCode",
        "openclaw" => "OpenClaw",
        "hermes" => "Hermes",
        "claude-desktop" => "Claude Desktop",
        _ => "Claude Code",
    }
}

pub fn asset_support(asset_type: &str, app: &str) -> AssetSupport {
    match asset_capability(asset_type, app).support.as_str() {
        "supported" => AssetSupport::Supported,
        "partial" => AssetSupport::Partial,
        _ => AssetSupport::Unsupported,
    }
}

pub fn asset_capability(asset_type: &str, app: &str) -> AssetCapabilityDescriptor {
    let contract: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../src/lib/projectAssets/assetAppSupport.contract.json"
    )))
    .expect("asset support contract must be valid JSON");
    let Some(asset) = contract
        .get("assets")
        .and_then(|assets| assets.get(asset_type))
    else {
        return AssetCapabilityDescriptor {
            support: "unsupported".into(),
            write_mode: "none".into(),
            verify_modes: Vec::new(),
            limitations: vec!["missing_contract_entry".into()],
            adapter_id: format!("{app}-{asset_type}-unsupported-v1"),
            fixture_id: "unsupported".into(),
        };
    };
    let contains_app = |key: &str| {
        asset
            .get(key)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|apps| apps.iter().any(|value| value.as_str() == Some(app)))
    };

    let support = if contains_app("supported") {
        "supported"
    } else if contains_app("partial") {
        "partial"
    } else {
        "unsupported"
    };
    let write_mode = if support == "unsupported" {
        "none".to_string()
    } else {
        asset
            .get("write_mode_overrides")
            .and_then(|value| value.get(app))
            .or_else(|| asset.get("write_mode"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("none")
            .to_string()
    };
    let verify_modes = if support == "unsupported" {
        Vec::new()
    } else {
        asset
            .get("verify_modes")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    let limitations = asset
        .get("limitations")
        .and_then(|value| value.get(app))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    AssetCapabilityDescriptor {
        support: support.into(),
        write_mode,
        verify_modes,
        limitations,
        adapter_id: asset
            .get("adapter_id")
            .and_then(serde_json::Value::as_str)
            .map(|adapter| format!("{adapter}:{app}"))
            .unwrap_or_else(|| format!("{app}-{asset_type}-v1")),
        fixture_id: asset
            .get("fixture_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("missing-fixture")
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_commands_supported() {
        assert_eq!(asset_support("command", "codex"), AssetSupport::Supported);
    }

    #[test]
    fn claude_hooks_supported() {
        assert_eq!(asset_support("hook", "claude"), AssetSupport::Supported);
    }

    #[test]
    fn gemini_hooks_supported() {
        assert_eq!(asset_support("hook", "gemini"), AssetSupport::Supported);
    }

    #[test]
    fn codex_permissions_supported() {
        assert_eq!(
            asset_support("permission", "codex"),
            AssetSupport::Supported
        );
    }

    #[test]
    fn hermes_permissions_partial() {
        assert_eq!(asset_support("permission", "hermes"), AssetSupport::Partial);
    }

    #[test]
    fn codex_subagent_partial() {
        assert_eq!(asset_support("subagent", "codex"), AssetSupport::Partial);
        let capability = asset_capability("subagent", "codex");
        assert_eq!(capability.write_mode, "global_path");
        assert!(capability
            .limitations
            .iter()
            .any(|item| item == "global_side_effect"));
        assert_eq!(
            capability.adapter_id,
            "project-config-sync:subagents_configured:v1:codex"
        );
        assert_eq!(capability.fixture_id, "project-config-sync:subagent");
    }

    #[test]
    fn shared_contract_covers_all_asset_app_pairs() {
        let contract: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../src/lib/projectAssets/assetAppSupport.contract.json"
        )))
        .unwrap();
        for asset in [
            "mcp",
            "skill",
            "prompt",
            "command",
            "hook",
            "ignore",
            "permission",
            "subagent",
        ] {
            for app in [
                "claude",
                "claude-desktop",
                "codex",
                "gemini",
                "opencode",
                "openclaw",
                "hermes",
            ] {
                let source = &contract["assets"][asset];
                let contains = |key: &str| {
                    source[key].as_array().is_some_and(|values| {
                        values.iter().any(|value| value.as_str() == Some(app))
                    })
                };
                let expected = if contains("supported") {
                    AssetSupport::Supported
                } else if contains("partial") {
                    AssetSupport::Partial
                } else {
                    AssetSupport::Unsupported
                };
                assert_eq!(asset_support(asset, app), expected, "{asset}/{app}");
                let capability = asset_capability(asset, app);
                if expected != AssetSupport::Unsupported {
                    assert_ne!(capability.write_mode, "none", "{asset}/{app}");
                    assert!(!capability.adapter_id.is_empty(), "{asset}/{app}");
                    assert_ne!(capability.fixture_id, "missing-fixture", "{asset}/{app}");
                }
            }
        }
    }
}
