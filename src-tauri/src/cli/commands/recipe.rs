//! `os recipe` — Recipe 管理：列出、预览、安装、删除、预检

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct RecipeArgs {
    #[command(subcommand)]
    pub action: RecipeAction,
}

#[derive(Subcommand)]
pub enum RecipeAction {
    /// 列出已保存的 recipe
    List {
        /// 项目路径
        #[arg(long)]
        project_path: String,
    },
    /// 预览 recipe 内容
    Preview {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// recipe 名称（可选，未提供时交互式选择）
        #[arg(long)]
        name: Option<String>,
    },
    /// 安装 recipe 到项目
    Install {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// recipe 名称（可选，未提供时交互式选择）
        #[arg(long)]
        name: Option<String>,
        /// 变更 ID
        #[arg(long)]
        change_id: Option<String>,
        /// 仅预览安装计划，不执行
        #[arg(long)]
        dry_run: bool,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
    /// 删除已保存的 recipe
    Delete {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// recipe 名称（可选，未提供时交互式选择）
        #[arg(long)]
        name: Option<String>,
    },
    /// 安装前预检（dry-run）：文件列表与审计摘要
    Plan {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// recipe 名称（可选，未提供时交互式选择）
        #[arg(long)]
        name: Option<String>,
        /// 变更 ID
        #[arg(long)]
        change_id: Option<String>,
    },
}

pub fn run(args: RecipeArgs, json: bool) -> Result<(), String> {
    match args.action {
        RecipeAction::List { project_path } => run_list(&project_path, json),
        RecipeAction::Preview {
            project_path,
            name,
        } => {
            let name = match name {
                Some(n) => n,
                None => {
                    let names = open_sunstar_lib::cli_api::cli_recipe_list(&project_path)?;
                    let display: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                    match output::select("Select recipe", &display, json) {
                        Some(idx) => names[idx].clone(),
                        None => return Err("No recipe selected".to_string()),
                    }
                }
            };
            run_preview(&project_path, &name, json)
        }
        RecipeAction::Install {
            project_path,
            name,
            change_id,
            dry_run,
            yes,
        } => {
            let name = match name {
                Some(n) => n,
                None => {
                    let names = open_sunstar_lib::cli_api::cli_recipe_list(&project_path)?;
                    let display: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                    match output::select("Select recipe", &display, json) {
                        Some(idx) => names[idx].clone(),
                        None => return Err("No recipe selected".to_string()),
                    }
                }
            };
            run_install(&project_path, &name, change_id.as_deref(), dry_run, yes, json)
        }
        RecipeAction::Delete {
            project_path,
            name,
        } => {
            let name = match name {
                Some(n) => n,
                None => {
                    let names = open_sunstar_lib::cli_api::cli_recipe_list(&project_path)?;
                    let display: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                    match output::select("Select recipe", &display, json) {
                        Some(idx) => names[idx].clone(),
                        None => return Err("No recipe selected".to_string()),
                    }
                }
            };
            run_delete(&project_path, &name, json)
        }
        RecipeAction::Plan {
            project_path,
            name,
            change_id,
        } => {
            let name = match name {
                Some(n) => n,
                None => {
                    let names = open_sunstar_lib::cli_api::cli_recipe_list(&project_path)?;
                    let display: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                    match output::select("Select recipe", &display, json) {
                        Some(idx) => names[idx].clone(),
                        None => return Err("No recipe selected".to_string()),
                    }
                }
            };
            run_plan(&project_path, &name, change_id.as_deref(), json)
        }
    }
}

fn run_list(project_path: &str, json: bool) -> Result<(), String> {
    let names = open_sunstar_lib::cli_api::cli_recipe_list(project_path)?;

    if json {
        output::print_result(&names, true);
    } else if names.is_empty() {
        println!("No saved recipes found in {project_path}");
    } else {
        println!("Saved Recipes ({}):\n", names.len());
        for name in &names {
            println!("  · {name}");
        }
    }

    Ok(())
}

fn run_preview(project_path: &str, name: &str, json: bool) -> Result<(), String> {
    let content = open_sunstar_lib::cli_api::cli_recipe_read(project_path, name)?;

    if json {
        let result = serde_json::json!({
            "name": name,
            "project_path": project_path,
            "content": content,
        });
        output::print_result(&result, true);
    } else {
        // Recipe is a YAML+Markdown hybrid — output directly for readability
        print!("{content}");
    }

    Ok(())
}

fn run_install(
    project_path: &str,
    name: &str,
    change_id: Option<&str>,
    dry_run: bool,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    if dry_run {
        // Dry-run: show recipe preview and indicate install would proceed
        let content = open_sunstar_lib::cli_api::cli_recipe_read(project_path, name)?;

        if json {
            let result = serde_json::json!({
                "dry_run": true,
                "name": name,
                "project_path": project_path,
                "change_id": change_id,
                "recipe_content": content,
            });
            output::print_result(&result, true);
        } else {
            println!("Recipe Install Plan (dry-run)\n");
            println!("  Recipe:    {name}");
            println!("  Project:   {project_path}");
            if let Some(cid) = change_id {
                println!("  Change ID: {cid}");
            }
            println!();
            println!("Would create:");
            println!("  + .specs/ directory (if not exists)");
            println!("  + .specs/CONTEXT.md, ARCHITECTURE.md, LESSONS.md");
            println!("  + STATE.md");
            if let Some(cid) = change_id {
                println!("  + .specs/{cid}/ with stage artifact templates");
            }
            println!("  + .opensunstar/recipe/{name}.recipe.md");
            println!("\nExisting files would be skipped (no overwrite).\n");
            println!("--- Recipe Preview ---\n");
            print!("{content}");
        }

        return Ok(());
    }

    // Interactive confirmation (unless --yes or --json)
    if !yes && !json {
        output::header("Recipe Install Plan");
        output::info(&format!("  Recipe:    {name}"));
        output::info(&format!("  Project:   {project_path}"));
        if let Some(cid) = change_id {
            output::info(&format!("  Change ID: {cid}"));
        }
        output::info("Would create:");
        output::dim("  + .specs/ directory (if not exists)");
        output::dim("  + .specs/CONTEXT.md, ARCHITECTURE.md, LESSONS.md");
        output::dim("  + STATE.md");
        if let Some(cid) = change_id {
            output::dim(&format!("  + .specs/{cid}/ with stage artifact templates"));
        }
        output::dim(&format!("  + .opensunstar/recipe/{name}.recipe.md"));
        output::dim("Existing files would be skipped (no overwrite).");

        if !output::confirm("确认执行?", false, false) {
            output::warning("已取消。");
            return Ok(());
        }
    }

    // Actual install — read recipe content first, then install from content
    let content = open_sunstar_lib::cli_api::cli_recipe_read(project_path, name)?;
    let result = open_sunstar_lib::cli_api::cli_recipe_install(
        project_path,
        &content,
        change_id,
    )?;

    if json {
        output::print_result(&result, true);
    } else {
        output::success("Recipe Installed");
        eprintln!();
        println!("  Change ID: {}", result.change_id);
        if result.specs_dir_created {
            output::success(".specs/ directory created");
        }
        if result.state_file_created {
            output::success("STATE.md created");
        }
        if !result.files_created.is_empty() {
            eprintln!();
            println!("  Files created ({}):", result.files_created.len());
            for f in &result.files_created {
                println!("    + {f}");
            }
        }
        if !result.files_skipped.is_empty() {
            eprintln!();
            println!("  Files skipped ({}):", result.files_skipped.len());
            for f in &result.files_skipped {
                output::dim(&format!("    · {f} (already exists)"));
            }
        }
    }

    Ok(())
}

fn run_delete(project_path: &str, name: &str, json: bool) -> Result<(), String> {
    open_sunstar_lib::cli_api::cli_recipe_delete(project_path, name)?;

    if json {
        let result = serde_json::json!({
            "deleted": true,
            "name": name,
        });
        output::print_result(&result, true);
    } else {
        output::success(&format!("Recipe '{name}' deleted"));
    }

    Ok(())
}

fn run_plan(
    project_path: &str,
    name: &str,
    change_id: Option<&str>,
    json: bool,
) -> Result<(), String> {
    // cli_recipe_plan takes full YAML+MD content, not just the name
    let content = open_sunstar_lib::cli_api::cli_recipe_read(project_path, name)?;

    // Generate a default change_id when none is provided (same pattern as cli_recipe_install)
    let default_cid;
    let cid: &str = match change_id {
        Some(s) => s,
        None => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            default_cid = format!("{}-{}", name.to_lowercase(), ts);
            &default_cid
        }
    };

    let plan = open_sunstar_lib::cli_api::cli_recipe_plan(project_path, &content, cid)?;

    if json {
        output::print_result(&plan, true);
    } else {
        output::header("Recipe Install Plan");
        output::info(&format!("  Recipe:    {name}"));
        output::info(&format!("  Project:   {project_path}"));
        output::info(&format!("  Change ID: {cid}"));
        eprintln!();

        // File list
        let create_count = plan.files.iter().filter(|f| f.status == "create").count();
        let skip_count = plan.files.iter().filter(|f| f.status == "skip").count();

        if !plan.files.is_empty() {
            output::info(&format!("Files ({}) :", plan.files.len()));
            for entry in &plan.files {
                if entry.status == "create" {
                    output::success(&format!("  + {} [create]", entry.path));
                } else {
                    output::dim(&format!("  · {} [{}]", entry.path, entry.status));
                }
            }
            eprintln!();
            output::info(&format!(
                "  {create_count} to create, {skip_count} to skip"
            ));
        }

        eprintln!();

        // Audit summary
        output::info(&format!(
            "Audit: {} files scanned, {} findings",
            plan.audit.files_scanned, plan.audit.total_findings
        ));
        if plan.audit.total_findings > 0 {
            output::info(&format!(
                "  critical: {}, high: {}, medium: {}, low: {}",
                plan.audit.critical, plan.audit.high, plan.audit.medium, plan.audit.low
            ));
            for finding in &plan.audit.findings {
                output::warning(&format!(
                    "  [{}] {} — {} ({})",
                    finding.severity, finding.rule_id, finding.message, finding.file
                ));
            }
        }
        if plan.audit.blocked {
            eprintln!();
            output::warning("Install is BLOCKED due to audit findings.");
        }
    }

    Ok(())
}
