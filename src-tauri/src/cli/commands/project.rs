//! `os project` — 项目管理：列出项目、扫描框架检测、全景状态

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub action: ProjectAction,
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// 列出所有项目
    List,
    /// 扫描项目框架检测
    Scan {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 保存到数据库
        #[arg(long)]
        save: bool,
    },
    /// 项目全景状态（聚合编排 + 资产 + 漂移）
    Status {
        /// 项目路径（可选，未提供时从已注册项目交互式选择）
        #[arg(long)]
        project_path: Option<String>,
    },
}

pub fn run(
    args: ProjectArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        ProjectAction::List => run_list(state, json),
        ProjectAction::Scan { project_path, save } => run_scan(state, &project_path, save, json),
        ProjectAction::Status { project_path } => {
            let path = match project_path {
                Some(p) => p,
                None => {
                    let projects = open_sunstar_lib::cli_api::cli_project_list(state)?;
                    if projects.is_empty() {
                        return Err("No projects registered".to_string());
                    }
                    let display: Vec<String> = projects
                        .iter()
                        .map(|p| format!("{} — {}", p.name, p.path))
                        .collect();
                    match output::select("Select project", &display, json) {
                        Some(idx) => projects[idx].path.clone(),
                        None => return Err("No project selected".to_string()),
                    }
                }
            };
            run_status(state, &path, json)
        }
    }
}

fn run_list(state: &open_sunstar_lib::AppState, json: bool) -> Result<(), String> {
    let projects = open_sunstar_lib::cli_api::cli_project_list(state)?;

    if json {
        output::print_result(&projects, true);
    } else if projects.is_empty() {
        output::info("No projects registered.");
    } else {
        output::header(&format!("Projects ({}):", projects.len()));
        eprintln!();
        println!("{:<20} {:<12} {:<10} {}", "Name", "Stage", "Target", "Path");
        println!("{}", "-".repeat(72));
        for p in &projects {
            let target = p.target_app.as_deref().unwrap_or("-");
            println!("  {:<18} {:<12} {:<10} {}", p.name, p.stage, target, p.path);
        }
    }

    Ok(())
}

fn run_scan(
    _state: &open_sunstar_lib::AppState,
    project_path: &str,
    save: bool,
    json: bool,
) -> Result<(), String> {
    let results = open_sunstar_lib::cli_api::cli_sdd_detect(project_path);

    if save {
        output::warning("保存功能暂未实现 (cli_project_save_scan)，请使用 GUI 保存检测结果。");
    }

    if json {
        output::print_result(&results, true);
    } else {
        output::header(&format!("Framework Detection: {project_path}"));
        eprintln!();
        println!(
            "{:<18} {:<10} {:<10} {}",
            "Framework", "Detected", "Confidence", "Signals"
        );
        println!("{}", "-".repeat(64));

        let mut detected_count = 0;
        for r in &results {
            let icon = if r.detected { "✓" } else { "·" };
            if r.detected {
                detected_count += 1;
            }
            let signals = r
                .signal_matches
                .iter()
                .map(|s| s.signal.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let signals_display = if signals.is_empty() {
                "-".to_string()
            } else {
                signals
            };
            println!(
                "  {icon} {:<16} {:<10} {:<10} {}",
                r.descriptor_id,
                if r.detected { "yes" } else { "no" },
                r.confidence,
                signals_display
            );
        }

        println!();
        if detected_count > 0 {
            output::success(&format!("{detected_count} framework(s) detected."));
            // Recommend preset
            let detected_ids: Vec<&str> = results
                .iter()
                .filter(|r| r.detected)
                .map(|r| r.descriptor_id.as_str())
                .collect();
            let recommended = if detected_ids.contains(&"flow-kit") {
                "full"
            } else if detected_ids.contains(&"spec-kit") || detected_ids.contains(&"openspec") {
                "standard"
            } else if detected_ids.contains(&"bmad-method") || detected_ids.contains(&"gstack") {
                "standard"
            } else {
                "mvp"
            };
            output::info(&format!("Recommended preset: `{recommended}`"));
        } else {
            output::info("No frameworks detected. Consider `review-only` preset.");
        }

        if save {
            output::dim("(保存功能暂未实现)");
        }
    }

    Ok(())
}

fn run_status(
    state: &open_sunstar_lib::AppState,
    project_path: &str,
    json: bool,
) -> Result<(), String> {
    let ctx = open_sunstar_lib::cli_api::cli_project_context(state, project_path)?;

    if json {
        output::print_result(&ctx, true);
    } else {
        // ── Project metadata ──
        output::header(&format!("Project: {}", ctx.project.name));
        output::info(&format!("  Path:       {}", ctx.project.path));
        output::info(&format!("  Stage:      {}", ctx.project.stage));
        if let Some(ref target) = ctx.project.target_app {
            output::info(&format!("  Target App: {target}"));
        }
        if let Some(ref bp) = ctx.project.blueprint_id {
            output::info(&format!("  Blueprint:  {bp}"));
        }
        if let Some(ref git) = ctx.project.git_remote_url {
            output::info(&format!("  Git Remote: {git}"));
        }

        eprintln!();

        // ── Orchestration state ──
        output::header("Orchestration:");
        let status_icon = |ok: bool| if ok { "✓" } else { "·" };

        output::info(&format!(
            "  {} Workspace (.opensunstar/)",
            status_icon(ctx.workspace_exists)
        ));
        if ctx.workspace_exists {
            output::info(&format!(
                "    {} Workflow Profile",
                status_icon(ctx.has_flow_profile)
            ));
            output::info(&format!(
                "    {} FlowConfig (CI gate)",
                status_icon(ctx.has_flow_config)
            ));
            output::info(&format!(
                "    {} Recipes saved ({})",
                status_icon(ctx.recipe_count > 0),
                ctx.recipe_count
            ));
            output::info(&format!(
                "    {} Design Contracts ({})",
                status_icon(ctx.contract_count > 0),
                ctx.contract_count
            ));
        }
        output::info(&format!(
            "  {} Design Contract (DESIGN.md)",
            status_icon(ctx.has_design_contract)
        ));
        output::info(&format!(
            "  {} Specs Directory (.specs/)",
            status_icon(ctx.specs_exists)
        ));
        if let Some(ref cid) = ctx.active_change_id {
            output::info(&format!("    Active Change: {cid}"));
        }
        if let Some(score) = ctx.total_artifact_completeness {
            let label = if score >= 80 {
                format!("{score}%")
            } else if score >= 50 {
                format!("{score}%")
            } else {
                format!("{score}%")
            };
            output::info(&format!("    Artifact Completeness: {label}"));
        }

        eprintln!();

        // ── Asset counts ──
        let ac = &ctx.asset_counts;
        let total = ac.mcp
            + ac.skills
            + ac.prompts
            + ac.commands
            + ac.hooks
            + ac.ignore
            + ac.permissions
            + ac.subagents;

        output::header(&format!("Assets ({total} total):"));
        let types = [
            ("MCP Servers", ac.mcp),
            ("Skills", ac.skills),
            ("Prompts", ac.prompts),
            ("Commands", ac.commands),
            ("Hooks", ac.hooks),
            ("Ignore Rules", ac.ignore),
            ("Permissions", ac.permissions),
            ("Subagents", ac.subagents),
        ];
        for (label, count) in &types {
            if *count > 0 {
                output::info(&format!("  {label:<20} {count}"));
            } else {
                output::dim(&format!("  {label:<20} 0"));
            }
        }
    }

    Ok(())
}
