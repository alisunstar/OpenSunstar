//! `os profile` — 项目基线快照与蓝图管理：导出、列出、预览、应用

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: ProfileAction,
}

#[derive(Subcommand)]
pub enum ProfileAction {
    /// 导出项目基线快照
    Export {
        /// 项目 ID
        #[arg(long)]
        project_id: Option<String>,
    },
    /// 列出可用蓝图
    List,
    /// 预览蓝图应用效果
    Preview {
        /// 项目 ID
        #[arg(long)]
        project_id: Option<String>,
        /// 蓝图 ID
        #[arg(long)]
        blueprint_id: Option<String>,
    },
    /// 应用蓝图到项目
    Apply {
        /// 项目 ID
        #[arg(long)]
        project_id: Option<String>,
        /// 蓝图 ID
        #[arg(long)]
        blueprint_id: Option<String>,
        /// 仅预览变更，不执行
        #[arg(long)]
        dry_run: bool,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
}

pub fn run(
    args: ProfileArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        ProfileAction::Export { project_id } => {
            let project_id = resolve_project_id(state, project_id, json)?;
            run_export(state, &project_id, json)
        }
        ProfileAction::List => run_list(json),
        ProfileAction::Preview {
            project_id,
            blueprint_id,
        } => {
            let project_id = resolve_project_id(state, project_id, json)?;
            let blueprint_id = resolve_blueprint_id(blueprint_id, json)?;
            run_preview(state, &project_id, &blueprint_id, json)
        }
        ProfileAction::Apply {
            project_id,
            blueprint_id,
            dry_run,
            yes,
        } => {
            let project_id = resolve_project_id(state, project_id, json)?;
            let blueprint_id = resolve_blueprint_id(blueprint_id, json)?;
            run_apply(state, &project_id, &blueprint_id, dry_run, yes, json)
        }
    }
}

/// Resolve project_id: use provided value, or show interactive select, or error in json mode.
fn resolve_project_id(
    state: &open_sunstar_lib::AppState,
    project_id: Option<String>,
    json: bool,
) -> Result<String, String> {
    match project_id {
        Some(id) => Ok(id),
        None => {
            if json {
                return Err("--project-id is required in --json mode".to_string());
            }
            let projects = open_sunstar_lib::cli_api::cli_project_list(state)?;
            if projects.is_empty() {
                return Err("No projects available. Please create a project first.".to_string());
            }
            let items: Vec<String> = projects
                .iter()
                .map(|p| format!("{} ({})", p.name, p.path))
                .collect();
            match output::select("Select project", &items, false) {
                Some(idx) => Ok(projects[idx].id.clone()),
                None => Err("No project selected.".to_string()),
            }
        }
    }
}

/// Resolve blueprint_id: use provided value, or show interactive select, or error in json mode.
fn resolve_blueprint_id(
    blueprint_id: Option<String>,
    json: bool,
) -> Result<String, String> {
    match blueprint_id {
        Some(id) => Ok(id),
        None => {
            if json {
                return Err("--blueprint-id is required in --json mode".to_string());
            }
            let blueprints = open_sunstar_lib::cli_api::cli_blueprint_list()?;
            if blueprints.is_empty() {
                return Err("No blueprints available.".to_string());
            }
            let items: Vec<String> = blueprints
                .iter()
                .map(|b| format!("{} — {}", b.id, b.description))
                .collect();
            match output::select("Select blueprint", &items, false) {
                Some(idx) => Ok(blueprints[idx].id.clone()),
                None => Err("No blueprint selected.".to_string()),
            }
        }
    }
}

fn run_export(
    state: &open_sunstar_lib::AppState,
    project_id: &str,
    json: bool,
) -> Result<(), String> {
    let project = open_sunstar_lib::cli_api::cli_project_get(state, project_id)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    let asset_counts = open_sunstar_lib::cli_api::cli_asset_counts(state, project_id)?;

    if json {
        let export_data = serde_json::json!({
            "project_id": project_id,
            "project": {
                "name": project.name,
                "stage": project.stage,
                "path": project.path,
                "target_app": project.target_app,
            },
            "asset_counts": asset_counts,
        });
        output::print_result(&export_data, true);
    } else {
        output::header("Project baseline metadata:");
        println!("  Project: {} ({})", project.name, project_id);
        println!("  Stage: {}", project.stage);
        println!("  Path: {}", project.path);
        if let Some(ref app) = project.target_app {
            println!("  Target app: {}", app);
        }
        println!("\n  Asset counts:");
        println!("    mcp: {}", asset_counts.mcp);
        println!("    skills: {}", asset_counts.skills);
        println!("    prompts: {}", asset_counts.prompts);
        println!("    commands: {}", asset_counts.commands);
        println!("    hooks: {}", asset_counts.hooks);
        println!("    ignore: {}", asset_counts.ignore);
        println!("    permissions: {}", asset_counts.permissions);
        println!("    subagents: {}", asset_counts.subagents);
    }

    Ok(())
}

fn run_list(json: bool) -> Result<(), String> {
    let blueprints = open_sunstar_lib::cli_api::cli_blueprint_list()?;

    if json {
        output::print_result(&blueprints, true);
    } else {
        output::header(&format!("Available Blueprints ({}):", blueprints.len()));
        eprintln!();
        println!("{:<20} {:<14} {:<12} {}", "ID", "Project Type", "Target App", "Description");
        println!("{}", "-".repeat(72));
        for bp in &blueprints {
            println!(
                "  {:<18} {:<14} {:<12} {}",
                bp.id, bp.project_type, bp.target_app, bp.description
            );
        }
    }

    Ok(())
}

fn run_preview(
    state: &open_sunstar_lib::AppState,
    project_id: &str,
    blueprint_id: &str,
    json: bool,
) -> Result<(), String> {
    let preview = open_sunstar_lib::cli_api::cli_blueprint_preview(
        state,
        project_id,
        blueprint_id,
    )?;

    if json {
        output::print_result(&preview, true);
    } else {
        output::header(&format!(
            "Blueprint Preview: {} → {}",
            preview.blueprint_name, preview.target_app
        ));
        eprintln!();

        if preview.to_link.is_empty() {
            println!("  No assets to link — project is already fully configured.");
        } else {
            println!("  Assets to link ({}):\n", preview.to_link.len());
            for action in &preview.to_link {
                let app_label = action
                    .app_type
                    .as_deref()
                    .unwrap_or("-");
                println!(
                    "    + {:<12} {:<24} app: {app_label}",
                    action.asset_type, action.asset_id
                );
            }
        }

        if !preview.warnings.is_empty() {
            output::warning("Warnings:");
            for w in &preview.warnings {
                output::dim(&format!("    · {w}"));
            }
        }
    }

    Ok(())
}

fn run_apply(
    state: &open_sunstar_lib::AppState,
    project_id: &str,
    blueprint_id: &str,
    dry_run: bool,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    // Preview first to show what will happen
    let preview = open_sunstar_lib::cli_api::cli_blueprint_preview(
        state,
        project_id,
        blueprint_id,
    )?;

    // Dry-run: show preview and exit without applying
    if dry_run {
        if json {
            let result = serde_json::json!({
                "dry_run": true,
                "project_id": project_id,
                "blueprint_id": blueprint_id,
                "preview": preview,
            });
            output::print_result(&result, true);
        } else {
            output::header(&format!(
                "Blueprint Apply Plan (dry-run): {} → {}",
                preview.blueprint_name, preview.target_app
            ));
            eprintln!();

            if preview.to_link.is_empty() {
                println!("  No assets to link — project is already fully configured.");
            } else {
                println!("  Assets to link ({}):\n", preview.to_link.len());
                for action in &preview.to_link {
                    let app_label = action
                        .app_type
                        .as_deref()
                        .unwrap_or("-");
                    println!(
                        "    + {:<12} {:<24} app: {app_label}",
                        action.asset_type, action.asset_id
                    );
                }
            }

            if !preview.warnings.is_empty() {
                output::warning("Warnings:");
                for w in &preview.warnings {
                    output::dim(&format!("    · {w}"));
                }
            }
        }

        return Ok(());
    }

    // Interactive confirmation (unless --yes or --json)
    if !yes && !json {
        output::header(&format!(
            "Blueprint: {} → {}",
            preview.blueprint_name, preview.target_app
        ));
        output::info(&format!(
            "This will link {} asset(s) to project '{}'.",
            preview.to_link.len(),
            project_id
        ));

        if !preview.warnings.is_empty() {
            output::warning("Warnings:");
            for w in &preview.warnings {
                output::dim(&format!("  · {w}"));
            }
        }

        if !output::confirm("Proceed?", false, false) {
            output::info("Aborted.");
            return Ok(());
        }
    }

    // Apply the blueprint
    let result = open_sunstar_lib::cli_api::cli_blueprint_apply(
        state,
        project_id,
        blueprint_id,
    )?;

    if json {
        output::print_result(&result, true);
    } else {
        output::success(&format!(
            "Blueprint '{}' applied to project '{}'",
            result.blueprint_name, project_id
        ));
        println!("  Linked {} asset(s):", result.to_link.len());
        for action in &result.to_link {
            println!(
                "    + {:<12} {}",
                action.asset_type, action.asset_id
            );
        }

        if !result.warnings.is_empty() {
            output::warning("Warnings:");
            for w in &result.warnings {
                output::dim(&format!("    · {w}"));
            }
        }
    }

    Ok(())
}
