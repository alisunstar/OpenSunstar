//! `os asset` — 资产管理（kubectl 风格）
//!
//! 支持 8 类资产的列出和漂移对比：
//! mcp, skill, prompt, command, hook, ignore, permission, subagent

use clap::{Args, Subcommand};

use crate::output;

const ASSET_TYPES: &[&str] = &[
    "mcp",
    "skill",
    "prompt",
    "command",
    "hook",
    "ignore",
    "permission",
    "subagent",
];

#[derive(Args)]
pub struct AssetArgs {
    #[command(subcommand)]
    pub action: AssetAction,
}

#[derive(Subcommand)]
pub enum AssetAction {
    /// 列出资产
    List {
        /// 资产类型: mcp|skill|prompt|command|hook|ignore|permission|subagent（不指定则列出全部类型的计数）
        asset_type: Option<String>,
        /// 项目 ID（用于查看项目级资产链接）
        #[arg(long)]
        project_id: Option<String>,
    },
    /// 对比 SSOT 与磁盘的差异（复用 drift 检测）
    Diff {
        /// 资产类型
        asset_type: String,
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 目标应用
        #[arg(long)]
        target_app: Option<String>,
    },
}

pub fn run(args: AssetArgs, state: &open_sunstar_lib::AppState, json: bool) -> Result<(), String> {
    match args.action {
        AssetAction::List {
            asset_type,
            project_id,
        } => run_list(state, asset_type, project_id, json),
        AssetAction::Diff {
            asset_type,
            project_path,
            target_app,
        } => run_diff(state, &asset_type, &project_path, target_app, json),
    }
}

fn run_list(
    state: &open_sunstar_lib::AppState,
    asset_type: Option<String>,
    project_id: Option<String>,
    json: bool,
) -> Result<(), String> {
    // 如果指定了 project_id，显示项目级资产链接
    if let Some(ref pid) = project_id {
        return list_project_assets(state, pid, asset_type.as_deref(), json);
    }

    // Interactive select for asset_type when not specified and not json
    let asset_type = if asset_type.is_none() && !json {
        let items: Vec<String> = ASSET_TYPES.iter().map(|s| s.to_string()).collect();
        match output::select("Select asset type", &items, false) {
            Some(idx) => Some(ASSET_TYPES[idx].to_string()),
            None => None,
        }
    } else {
        asset_type
    };

    // 否则显示全局资产计数
    if let Some(ref at) = asset_type {
        // 验证类型
        if !ASSET_TYPES.contains(&at.as_str()) {
            return Err(format!(
                "不支持的资产类型: {at}（支持: {}）",
                ASSET_TYPES.join(", ")
            ));
        }
        let count = count_global_assets(state, at)?;
        if json {
            let result = serde_json::json!({
                "asset_type": at,
                "count": count,
            });
            output::print_result(&result, true);
        } else {
            println!("{at}: {count}");
        }
        return Ok(());
    }

    // 列出所有类型的计数
    let mut counts = serde_json::Map::new();
    if json {
        for at in ASSET_TYPES {
            let count = count_global_assets(state, at).unwrap_or(0);
            counts.insert(at.to_string(), serde_json::json!(count));
        }
        output::print_result(&serde_json::Value::Object(counts), true);
    } else {
        output::header("Asset counts:");
        eprintln!();
        for at in ASSET_TYPES {
            let count = count_global_assets(state, at).unwrap_or(0);
            println!("  · {:<14} {count}", at);
        }
    }

    Ok(())
}

fn count_global_assets(
    state: &open_sunstar_lib::AppState,
    asset_type: &str,
) -> Result<u32, String> {
    match asset_type {
        "mcp" => state
            .db
            .get_all_mcp_servers()
            .map(|m| m.len() as u32)
            .map_err(|e| e.to_string()),
        "skill" => state
            .db
            .get_all_installed_skills()
            .map(|s| s.len() as u32)
            .map_err(|e| e.to_string()),
        "prompt" => {
            // 汇总所有 app_type 的 prompts
            let apps = [
                "claude", "codex", "gemini", "opencode", "openclaw", "hermes",
            ];
            let mut total = 0u32;
            for app in &apps {
                total += state
                    .db
                    .get_prompts(app)
                    .map(|p| p.len() as u32)
                    .unwrap_or(0);
            }
            Ok(total)
        }
        "command" => state
            .db
            .get_all_commands()
            .map(|c| c.len() as u32)
            .map_err(|e| e.to_string()),
        "hook" => state
            .db
            .get_all_hooks()
            .map(|h| h.len() as u32)
            .map_err(|e| e.to_string()),
        "ignore" => state
            .db
            .count_global_ignore_rules()
            .map_err(|e| e.to_string()),
        "permission" => state
            .db
            .count_global_permissions()
            .map_err(|e| e.to_string()),
        "subagent" => state
            .db
            .get_all_agents()
            .map(|a| a.len() as u32)
            .map_err(|e| e.to_string()),
        _ => Err(format!("不支持的资产类型: {asset_type}")),
    }
}

fn list_project_assets(
    state: &open_sunstar_lib::AppState,
    project_id: &str,
    asset_type: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let links = state
        .db
        .get_project_asset_links(project_id, asset_type)
        .map_err(|e| e.to_string())?;

    if json {
        output::print_result(&links, true);
    } else {
        if links.is_empty() {
            output::info(&format!("No asset links found for project '{project_id}'."));
            return Ok(());
        }

        output::header(&format!(
            "Asset links for project '{project_id}' ({} total):",
            links.len()
        ));
        eprintln!();
        println!(
            "  {:<12} {:<24} {:<8} {}",
            "TYPE", "ASSET_ID", "ENABLED", "SOURCE"
        );
        println!("  {}", "-".repeat(60));
        for link in &links {
            let enabled = if link.enabled { "✓" } else { "·" };
            println!(
                "  {:<12} {:<24} {:<8} {}",
                link.asset_type, link.asset_id, enabled, link.source
            );
        }
    }

    Ok(())
}

fn run_diff(
    state: &open_sunstar_lib::AppState,
    asset_type: &str,
    project_path: &str,
    target_app: Option<String>,
    json: bool,
) -> Result<(), String> {
    if !ASSET_TYPES.contains(&asset_type) {
        return Err(format!(
            "不支持的资产类型: {asset_type}（支持: {}）",
            ASSET_TYPES.join(", ")
        ));
    }

    let project_path = resolve_project_path(project_path);
    let result = open_sunstar_lib::cli_api::cli_drift_check(state, &project_path, target_app)?;

    // 过滤出与指定资产类型相关的漂移项
    let filtered: Vec<_> = result
        .items
        .iter()
        .filter(|i| {
            // check_name 通常包含资产类型关键字，如 "mcp_configured"、"skills_configured"
            i.check_name.starts_with(asset_type)
                || i.check_name.contains(asset_type)
                || (asset_type == "skill" && i.check_name.contains("skill"))
                || (asset_type == "subagent" && i.check_name.contains("agent"))
        })
        .collect();

    if json {
        let items: Vec<_> = filtered
            .iter()
            .map(|i| {
                serde_json::json!({
                    "check_name": i.check_name,
                    "state": i.effective_state,
                    "detail": i.effective_detail,
                    "live_path": i.live_path,
                })
            })
            .collect();
        let report = serde_json::json!({
            "asset_type": asset_type,
            "project_path": project_path,
            "items": items,
        });
        output::print_result(&report, true);
    } else {
        output::header(&format!(
            "Drift diff for asset type '{asset_type}' at {project_path}:"
        ));
        eprintln!();
        if filtered.is_empty() {
            output::success(&format!("No drift detected for '{asset_type}' assets."));
        } else {
            for item in &filtered {
                let icon = if item.effective_state == "drifted" {
                    "✗"
                } else {
                    "✓"
                };
                println!(
                    "  {icon} {:<24} {} {}",
                    item.check_name,
                    item.effective_state,
                    item.effective_detail.as_deref().unwrap_or("")
                );
            }
        }
    }

    let has_drift = filtered.iter().any(|i| i.effective_state == "drifted");
    if has_drift {
        std::process::exit(1);
    }

    Ok(())
}

fn resolve_project_path(path: &str) -> String {
    if path == "." {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string())
    } else {
        std::fs::canonicalize(path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.to_string())
    }
}
