//! 8 类资产 × 目标 CLI 能力矩阵（与前端 `assetAppSupport.ts` 一致）
//!
//! 就绪度评分：unsupported 且未配置时不计为缺口；partial 保留缺口但标注部分支持。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetSupport {
    Supported,
    Partial,
    Unsupported,
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
    match (asset_type, app) {
        // MCP
        ("mcp", "claude-desktop") | ("mcp", "openclaw") => AssetSupport::Unsupported,
        ("mcp", _) => AssetSupport::Supported,

        // Skills
        ("skill", "claude-desktop") | ("skill", "openclaw") => AssetSupport::Unsupported,
        ("skill", _) => AssetSupport::Supported,

        // Prompts
        ("prompt", "claude-desktop") => AssetSupport::Unsupported,
        ("prompt", _) => AssetSupport::Supported,

        // Commands
        ("command", "claude-desktop") | ("command", "openclaw") => {
            AssetSupport::Unsupported
        }
        ("command", _) => AssetSupport::Supported,

        // Hooks
        ("hook", "claude") => AssetSupport::Supported,
        ("hook", "codex") | ("hook", "gemini") | ("hook", "hermes") => AssetSupport::Supported,
        ("hook", _) => AssetSupport::Unsupported,

        // Ignore
        ("ignore", "claude-desktop") | ("ignore", "openclaw") => AssetSupport::Unsupported,
        ("ignore", _) => AssetSupport::Supported,

        // Permissions
        ("permission", "claude")
        | ("permission", "codex")
        | ("permission", "gemini")
        | ("permission", "opencode")
        | ("permission", "openclaw") => AssetSupport::Supported,
        ("permission", "hermes") => AssetSupport::Partial,
        ("permission", _) => AssetSupport::Unsupported,

        // Subagents
        ("subagent", "claude-desktop") | ("subagent", "openclaw") | ("subagent", "hermes") => {
            AssetSupport::Unsupported
        }
        ("subagent", "codex") => AssetSupport::Partial,
        ("subagent", _) => AssetSupport::Supported,

        _ => AssetSupport::Supported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_commands_supported() {
        assert_eq!(
            asset_support("command", "codex"),
            AssetSupport::Supported
        );
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
        assert_eq!(
            asset_support("permission", "hermes"),
            AssetSupport::Partial
        );
    }

    #[test]
    fn codex_subagent_partial() {
        assert_eq!(asset_support("subagent", "codex"), AssetSupport::Partial);
    }
}
