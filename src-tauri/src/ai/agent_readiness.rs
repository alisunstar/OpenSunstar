//! Agent 配置就绪度 100 分模型（8 类资产 + 近 90 天更新）

use crate::ai::asset_app_support::{
    app_display_label, asset_support, check_name_to_asset_type, normalize_target_app, AssetSupport,
};
use crate::ai::types::AgentReadinessItem;

pub const AGENT_READINESS_MAX_SCORE: u32 = 100;

/// 单项状态（与开发文档 3.4 一致）
pub const STATUS_READY: &str = "ready";
pub const STATUS_PARTIAL: &str = "partial";
pub const STATUS_GLOBAL_ONLY: &str = "global_only";
pub const STATUS_DETECTED_ONLY: &str = "detected_only";
pub const STATUS_MISSING: &str = "missing";
pub const STATUS_NOT_APPLICABLE: &str = "not_applicable";

#[derive(Debug, Clone, Default)]
pub struct ReadinessCheckInput {
    pub mcp_project_count: u32,
    pub has_repo_mcp: bool,
    pub skills_count: u32,
    pub prompt_db_count: u32,
    pub prompt_files: Vec<String>,
    pub commands_count: u32,
    pub hooks_count: u32,
    pub ignore_project_count: u32,
    pub ignore_global_count: u32,
    pub permissions_project_count: u32,
    pub permissions_global_count: u32,
    pub subagents_count: u32,
    pub recent_update_within_90d: bool,
    /// 看板当前目标 CLI（claude / codex / gemini / opencode 等）
    pub target_app: Option<String>,
}

fn push_item(
    details: &mut Vec<AgentReadinessItem>,
    total: &mut u32,
    check_name: &str,
    label: &str,
    weight: u32,
    score: u32,
    detail: String,
    status: &str,
) {
    *total += score;
    details.push(AgentReadinessItem {
        check_name: check_name.to_string(),
        label: label.to_string(),
        weight,
        score,
        detail,
        status: Some(status.to_string()),
        configured_state: None,
        effective_state: None,
        effective_detail: None,
        effective_scanned_at: None,
        live_path: None,
    });
}

/// 按目标 CLI 能力调整单项得分与状态（§7.4：unsupported 不计缺口）
fn apply_target_app_support(
    check_name: &str,
    weight: u32,
    score: u32,
    status: &str,
    detail: String,
    target_app: &str,
) -> (u32, String, String) {
    let Some(asset_type) = check_name_to_asset_type(check_name) else {
        return (score, status.to_string(), detail);
    };

    match asset_support(asset_type, target_app) {
        AssetSupport::Supported => (score, status.to_string(), detail),
        AssetSupport::Unsupported => (
            0,
            STATUS_NOT_APPLICABLE.to_string(),
            format!(
                "{}（当前目标 CLI「{}」不支持此项，已从评分中排除）",
                detail,
                app_display_label(target_app)
            ),
        ),
        AssetSupport::Partial => {
            let st = if score >= weight {
                STATUS_READY.to_string()
            } else {
                STATUS_PARTIAL.to_string()
            };
            let detail = if score < weight {
                format!(
                    "{}（{} 对该项为部分支持，建议按需配置）",
                    detail,
                    app_display_label(target_app)
                )
            } else {
                detail
            };
            (score, st, detail)
        }
    }
}

/// 根据输入计算 9 项检查明细与总分（满分 100；按 target_app 动态调分）
pub fn compute_readiness_items(input: &ReadinessCheckInput) -> (u32, Vec<AgentReadinessItem>) {
    let target_app = normalize_target_app(input.target_app.as_deref());
    let mut details = Vec::with_capacity(9);
    let mut total_score = 0u32;

    // MCP 15
    let (mcp_score, mcp_status, mcp_detail) = if input.mcp_project_count > 0 {
        (
            15,
            STATUS_READY,
            format!("项目已关联 {} 个 MCP 服务器", input.mcp_project_count),
        )
    } else if input.has_repo_mcp {
        (
            6,
            STATUS_DETECTED_ONLY,
            "项目目录检测到 .mcp.json（尚未在 OpenSunstar 中关联）".to_string(),
        )
    } else {
        (
            0,
            STATUS_MISSING,
            "未关联 MCP，且项目目录无 .mcp.json".to_string(),
        )
    };
    push_item(
        &mut details,
        &mut total_score,
        "mcp_enabled",
        "MCP 服务器",
        15,
        mcp_score,
        mcp_detail,
        mcp_status,
    );

    // Skills 12
    let skills_score = if input.skills_count > 0 { 12 } else { 0 };
    push_item(
        &mut details,
        &mut total_score,
        "skills_configured",
        "Skills",
        12,
        skills_score,
        if input.skills_count > 0 {
            format!("项目已启用 {} 个 Skills", input.skills_count)
        } else {
            "项目未启用任何 Skills".to_string()
        },
        if skills_score > 0 {
            STATUS_READY
        } else {
            STATUS_MISSING
        },
    );

    // Prompts 12
    let has_db = input.prompt_db_count > 0;
    let has_files = !input.prompt_files.is_empty();
    let prompt_score = if has_db {
        12
    } else if has_files {
        5
    } else {
        0
    };
    let prompt_status = match (has_db, has_files) {
        (true, true) => STATUS_READY,
        (true, false) => STATUS_READY,
        (false, true) => STATUS_DETECTED_ONLY,
        (false, false) => STATUS_MISSING,
    };
    let prompt_detail = match (has_db, has_files) {
        (false, false) => "未关联 Prompt，且项目目录无提示词文件".to_string(),
        (true, false) => format!("项目已关联 {} 条 Prompt", input.prompt_db_count),
        (false, true) => format!(
            "项目目录有 {} 个提示词文件: {}",
            input.prompt_files.len(),
            input.prompt_files.join(", ")
        ),
        (true, true) => format!(
            "项目关联 {} 条 Prompt + 目录文件: {}",
            input.prompt_db_count,
            input.prompt_files.join(", ")
        ),
    };
    push_item(
        &mut details,
        &mut total_score,
        "prompt_files",
        "Prompt / AGENTS",
        12,
        prompt_score,
        prompt_detail,
        prompt_status,
    );

    // Commands 10
    let commands_score = if input.commands_count > 0 { 10 } else { 0 };
    push_item(
        &mut details,
        &mut total_score,
        "commands_configured",
        "Commands",
        10,
        commands_score,
        if input.commands_count > 0 {
            format!("项目已启用 {} 个 Commands", input.commands_count)
        } else {
            "项目未启用任何 Commands".to_string()
        },
        if commands_score > 0 {
            STATUS_READY
        } else {
            STATUS_MISSING
        },
    );

    // Hooks 10
    let hooks_score = if input.hooks_count > 0 { 10 } else { 0 };
    push_item(
        &mut details,
        &mut total_score,
        "hooks_configured",
        "Hooks",
        10,
        hooks_score,
        if input.hooks_count > 0 {
            format!("项目已启用 {} 个 Hooks", input.hooks_count)
        } else {
            "项目未启用任何 Hooks（当前写回以 Claude Code 为主）".to_string()
        },
        if hooks_score > 0 {
            STATUS_READY
        } else {
            STATUS_MISSING
        },
    );

    // Ignore 10 — 项目子集优先，否则全局基线
    let (ignore_score, ignore_status, ignore_detail) = if input.ignore_project_count > 0 {
        (
            10,
            STATUS_READY,
            format!("项目已启用 {} 条 Ignore 规则", input.ignore_project_count),
        )
    } else if input.ignore_global_count > 0 {
        (
            6,
            STATUS_GLOBAL_ONLY,
            format!(
                "使用全局基线 {} 条 Ignore 规则（可为项目单独启用子集）",
                input.ignore_global_count
            ),
        )
    } else {
        (0, STATUS_MISSING, "未配置 Ignore 规则".to_string())
    };
    push_item(
        &mut details,
        &mut total_score,
        "ignore_rules",
        "Ignore 规则",
        10,
        ignore_score,
        ignore_detail,
        ignore_status,
    );

    // Permissions 10
    let (perm_score, perm_status, perm_detail) = if input.permissions_project_count > 0 {
        (
            10,
            STATUS_READY,
            format!(
                "项目已启用 {} 条 Permissions",
                input.permissions_project_count
            ),
        )
    } else if input.permissions_global_count > 0 {
        (
            6,
            STATUS_GLOBAL_ONLY,
            format!(
                "使用全局基线 {} 条 Permissions（可为项目单独启用子集）",
                input.permissions_global_count
            ),
        )
    } else {
        (0, STATUS_MISSING, "未配置工具权限".to_string())
    };
    push_item(
        &mut details,
        &mut total_score,
        "permissions",
        "工具权限",
        10,
        perm_score,
        perm_detail,
        perm_status,
    );

    // Subagents 12
    let sub_score = if input.subagents_count > 0 { 12 } else { 0 };
    push_item(
        &mut details,
        &mut total_score,
        "subagents_configured",
        "Subagents",
        12,
        sub_score,
        if input.subagents_count > 0 {
            format!("项目已启用 {} 个 Subagents", input.subagents_count)
        } else {
            "项目未启用任何 Subagents".to_string()
        },
        if sub_score > 0 {
            STATUS_READY
        } else {
            STATUS_MISSING
        },
    );

    // 近 90 天项目资产关联更新 9
    let update_score = if input.recent_update_within_90d { 9 } else { 0 };
    push_item(
        &mut details,
        &mut total_score,
        "recent_updates",
        "近 90 天项目资产关联更新",
        9,
        update_score,
        if input.recent_update_within_90d {
            "近 90 天内有项目级 AI 资产配置变更".to_string()
        } else {
            "最近 90 天内无项目级资产配置变更".to_string()
        },
        if update_score > 0 {
            STATUS_READY
        } else {
            STATUS_MISSING
        },
    );

    // 按目标 CLI 能力矩阵调分（unsupported 缺口自动满分，partial 标注）
    total_score = 0;
    for item in &mut details {
        let base_status = item.status.as_deref().unwrap_or(STATUS_MISSING);
        let (score, status, detail) = apply_target_app_support(
            &item.check_name,
            item.weight,
            item.score,
            base_status,
            item.detail.clone(),
            target_app,
        );
        item.score = score;
        item.status = Some(status);
        item.detail = detail;
        total_score += score;
    }

    (total_score, details)
}

pub fn detect_repo_mcp_file(project_path: &str) -> bool {
    std::path::Path::new(project_path)
        .join(".mcp.json")
        .is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_baseline_scores_100() {
        let input = ReadinessCheckInput {
            mcp_project_count: 1,
            skills_count: 1,
            prompt_db_count: 1,
            commands_count: 1,
            hooks_count: 1,
            ignore_project_count: 1,
            permissions_project_count: 1,
            subagents_count: 1,
            recent_update_within_90d: true,
            ..Default::default()
        };
        let (score, items) = compute_readiness_items(&input);
        assert_eq!(score, 100);
        assert_eq!(items.len(), 9);
    }

    #[test]
    fn global_ignore_counts_with_global_only_status() {
        let input = ReadinessCheckInput {
            ignore_global_count: 3,
            recent_update_within_90d: true,
            ..Default::default()
        };
        let (score, items) = compute_readiness_items(&input);
        let ignore = items
            .iter()
            .find(|i| i.check_name == "ignore_rules")
            .unwrap();
        assert_eq!(ignore.score, 6);
        assert_eq!(ignore.status.as_deref(), Some(STATUS_GLOBAL_ONLY));
        assert_eq!(score, 15); // ignore 6 + recent 9
    }

    #[test]
    fn detected_prompt_files_score_without_db() {
        let input = ReadinessCheckInput {
            prompt_files: vec!["CLAUDE.md".to_string()],
            recent_update_within_90d: true,
            ..Default::default()
        };
        let (score, items) = compute_readiness_items(&input);
        let prompt = items
            .iter()
            .find(|i| i.check_name == "prompt_files")
            .unwrap();
        assert_eq!(prompt.score, 5);
        assert_eq!(prompt.status.as_deref(), Some(STATUS_DETECTED_ONLY));
        assert_eq!(score, 14); // prompt 5 + recent 9
    }

    #[test]
    fn codex_skips_unsupported_gaps() {
        let input = ReadinessCheckInput {
            recent_update_within_90d: true,
            target_app: Some("codex".to_string()),
            ..Default::default()
        };
        let (score, items) = compute_readiness_items(&input);
        let commands = items
            .iter()
            .find(|i| i.check_name == "commands_configured")
            .unwrap();
        assert_eq!(commands.score, 10);
        assert_eq!(commands.status.as_deref(), Some(STATUS_PARTIAL));
        let hooks = items
            .iter()
            .find(|i| i.check_name == "hooks_configured")
            .unwrap();
        assert_eq!(hooks.score, 10);
        let perms = items
            .iter()
            .find(|i| i.check_name == "permissions")
            .unwrap();
        assert_eq!(perms.score, 10);
        assert_eq!(score, 39);
    }

    #[test]
    fn claude_still_penalizes_missing_hooks() {
        let input = ReadinessCheckInput {
            recent_update_within_90d: true,
            target_app: Some("claude".to_string()),
            ..Default::default()
        };
        let (score, items) = compute_readiness_items(&input);
        let hooks = items
            .iter()
            .find(|i| i.check_name == "hooks_configured")
            .unwrap();
        assert_eq!(hooks.score, 0);
        assert_eq!(hooks.status.as_deref(), Some(STATUS_MISSING));
        assert_eq!(score, 9);
    }

    #[test]
    fn claude_desktop_unsupported_items_excluded() {
        let input = ReadinessCheckInput {
            recent_update_within_90d: true,
            target_app: Some("claude-desktop".to_string()),
            ..Default::default()
        };
        let (score, items) = compute_readiness_items(&input);
        // MCP is unsupported for claude-desktop
        let mcp = items
            .iter()
            .find(|i| i.check_name == "mcp_enabled")
            .unwrap();
        assert_eq!(mcp.score, 0);
        assert_eq!(mcp.status.as_deref(), Some(STATUS_NOT_APPLICABLE));
        // All asset types are unsupported for claude-desktop
        for check in &[
            "mcp_enabled",
            "skills_configured",
            "prompt_files",
            "commands_configured",
            "hooks_configured",
            "ignore_rules",
            "permissions",
            "subagents_configured",
        ] {
            let item = items.iter().find(|i| i.check_name == *check).unwrap();
            assert_eq!(item.score, 0, "{} should score 0 for claude-desktop", check);
            assert_eq!(
                item.status.as_deref(),
                Some(STATUS_NOT_APPLICABLE),
                "{} should be not_applicable for claude-desktop",
                check
            );
        }
        // Only recent_updates contributes to score
        assert_eq!(score, 9);
    }
}
