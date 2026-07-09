//! `os design` — 设计合约管理：列出模板、生成、安装、导入

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct DesignArgs {
    #[command(subcommand)]
    pub action: DesignAction,
}

#[derive(Subcommand)]
pub enum DesignAction {
    /// 列出可用设计合约模板
    List,
    /// 生成设计合约并导出
    Generate {
        /// 模板 ID（可选，未提供时交互式选择；vercel/apple/stripe/linear/notion/github/shadcn/neutral）
        #[arg(long)]
        template: Option<String>,
        /// 项目路径（导出到项目目录）
        #[arg(long)]
        project_path: Option<String>,
        /// 跳过确认（当指定 --project-path 时生效）
        #[arg(long)]
        yes: bool,
    },
    /// 安装设计合约到项目
    Install {
        /// 模板 ID（可选，未提供时交互式选择）
        #[arg(long)]
        template: Option<String>,
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 仅预览安装计划，不执行
        #[arg(long)]
        dry_run: bool,
        /// 跳过确认
        #[arg(long)]
        yes: bool,
    },
    /// 从文件导入设计合约
    Import {
        /// 文件路径
        #[arg(long)]
        file: String,
    },
    /// 从模板组合设计合约（可自定义参数）
    Compose {
        /// 模板 ID（可选，未提供时交互式选择）
        #[arg(long)]
        template: Option<String>,
        /// 品牌名称覆盖
        #[arg(long)]
        brand_name: Option<String>,
        /// 主色覆盖（如 "#FF5733"）
        #[arg(long)]
        primary_color: Option<String>,
    },
    /// 输出 W3C DTCG 格式的 design tokens JSON
    Dtcg {
        /// 模板 ID（可选，未提供时交互式选择）
        #[arg(long)]
        template: Option<String>,
    },
}

pub fn run(args: DesignArgs, json: bool) -> Result<(), String> {
    match args.action {
        DesignAction::List => run_list(json),
        DesignAction::Generate {
            template,
            project_path,
            yes,
        } => {
            let template = match template {
                Some(id) => id,
                None => {
                    let templates = open_sunstar_lib::cli_api::cli_design_list();
                    let display: Vec<String> = templates.iter().map(|(id, name)| format!("{id} — {name}")).collect();
                    match output::select("Select template", &display, json) {
                        Some(idx) => templates[idx].0.clone(),
                        None => return Err("No template selected".to_string()),
                    }
                }
            };
            run_generate(&template, project_path.as_deref(), yes, json)
        }
        DesignAction::Install {
            template,
            project_path,
            dry_run,
            yes,
        } => {
            let template = match template {
                Some(id) => id,
                None => {
                    let templates = open_sunstar_lib::cli_api::cli_design_list();
                    let display: Vec<String> = templates.iter().map(|(id, name)| format!("{id} — {name}")).collect();
                    match output::select("Select template", &display, json) {
                        Some(idx) => templates[idx].0.clone(),
                        None => return Err("No template selected".to_string()),
                    }
                }
            };
            run_install(&template, &project_path, dry_run, yes, json)
        }
        DesignAction::Import { file } => run_import(&file, json),
        DesignAction::Compose {
            template,
            brand_name,
            primary_color,
        } => {
            let template = match template {
                Some(id) => Some(id),
                None => {
                    let templates = open_sunstar_lib::cli_api::cli_design_list();
                    let display: Vec<String> = templates.iter().map(|(id, name)| format!("{id} — {name}")).collect();
                    match output::select("Select template", &display, json) {
                        Some(idx) => Some(templates[idx].0.clone()),
                        None => return Err("No template selected".to_string()),
                    }
                }
            };
            run_compose(template.as_deref(), brand_name.as_deref(), primary_color.as_deref(), json)
        }
        DesignAction::Dtcg { template } => {
            let template = match template {
                Some(id) => id,
                None => {
                    let templates = open_sunstar_lib::cli_api::cli_design_list();
                    let display: Vec<String> = templates.iter().map(|(id, name)| format!("{id} — {name}")).collect();
                    match output::select("Select template", &display, json) {
                        Some(idx) => templates[idx].0.clone(),
                        None => return Err("No template selected".to_string()),
                    }
                }
            };
            run_dtcg(&template, json)
        }
    }
}

fn run_list(json: bool) -> Result<(), String> {
    let templates = open_sunstar_lib::cli_api::cli_design_list();

    if json {
        output::print_result(&templates, true);
    } else {
        println!("Design Contract Templates ({}):\n", templates.len());
        println!("{:<12} {}", "ID", "Name");
        println!("{}", "-".repeat(36));
        for (id, name) in &templates {
            println!("  {:<10} {name}", id);
        }
    }

    Ok(())
}

fn run_generate(template_id: &str, project_path: Option<&str>, yes: bool, json: bool) -> Result<(), String> {
    // 1. Compose design contract from template
    let contract = open_sunstar_lib::cli_api::cli_design_get(template_id)?;

    // 2. Generate Markdown content
    let md_content = open_sunstar_lib::cli_api::cli_design_md(&contract)?;

    // 3. Optionally export to project directory (with confirmation)
    let export_path = if let Some(pp) = project_path {
        if !yes && !json {
            output::header("Design Contract Generate");
            output::info(&format!("  Template:  {template_id}"));
            output::info(&format!("  Name:      {}", contract.name));
            output::info(&format!("  Export to: {pp}"));
            output::info("Would create:");
            output::dim("  + DESIGN.md");
            output::dim("  + design-tokens.json");

            if !output::confirm("确认执行?", false, false) {
                output::warning("已取消。");
                return Ok(());
            }
        }
        Some(open_sunstar_lib::cli_api::cli_design_export(pp, &contract)?)
    } else {
        None
    };

    if json {
        let result = serde_json::json!({
            "contract": contract,
            "design_md": md_content,
            "exported_to": export_path,
        });
        output::print_result(&result, true);
    } else {
        println!("Design Contract: {}\n", contract.name);
        if let Some(ref desc) = contract.description {
            println!("  {desc}\n");
        }
        println!("  Template:   {}", contract.source_template.as_deref().unwrap_or("custom"));
        println!("  Primary:    {}", contract.colors.primary);
        println!("  Font:       {}", contract.typography.font_family_base);
        println!("  Components: {}", contract.components.len());
        println!("  Guardrails: {}", contract.guardrails.len());

        if let Some(ref path) = export_path {
            println!("\n  ✓ Exported to: {path}");
        }

        println!("\n--- DESIGN.md ---\n");
        print!("{md_content}");
    }

    Ok(())
}

fn run_install(template_id: &str, project_path: &str, dry_run: bool, yes: bool, json: bool) -> Result<(), String> {
    // Compose contract from template
    let contract = open_sunstar_lib::cli_api::cli_design_get(template_id)?;

    // Dry-run: show install plan and exit
    if dry_run {
        let plan = open_sunstar_lib::cli_api::cli_design_plan(project_path, &contract)?;

        if json {
            let result = serde_json::json!({
                "dry_run": true,
                "template": template_id,
                "project_path": project_path,
                "contract_name": contract.name,
                "plan": plan,
            });
            output::print_result(&result, true);
        } else {
            println!("Design Install Plan (dry-run)\n");
            println!("  Template:  {template_id}");
            println!("  Name:      {}", contract.name);
            println!("  Project:   {project_path}");
            println!();
            println!("  Files ({}):", plan.files.len());
            for f in &plan.files {
                println!("    {} {}", f.status, f.path);
            }
            println!();
            println!("  Audit: {} file(s) scanned, {} finding(s) [critical={}, high={}, medium={}, low={}]",
                plan.audit.files_scanned, plan.audit.total_findings,
                plan.audit.critical, plan.audit.high, plan.audit.medium, plan.audit.low);
            if plan.audit.blocked {
                println!("  ⚠ Blocked: yes");
            }
            if !plan.audit.findings.is_empty() {
                println!("\n  Findings:");
                for f in &plan.audit.findings {
                    println!("    [{}] {} — {} ({})", f.severity, f.rule_id, f.message, f.file);
                }
            }
        }

        return Ok(());
    }

    // Interactive confirmation (unless --yes or --json)
    if !yes && !json {
        let plan = open_sunstar_lib::cli_api::cli_design_plan(project_path, &contract)?;

        output::header("Design Contract Install");
        output::info(&format!("  Template:  {template_id}"));
        output::info(&format!("  Name:      {}", contract.name));
        output::info(&format!("  Project:   {project_path}"));
        output::info(&format!("  Files ({}):", plan.files.len()));
        for f in &plan.files {
            output::dim(&format!("    {} {}", f.status, f.path));
        }
        if plan.audit.blocked {
            output::warning("Audit blocked — review findings with --dry-run first.");
        }

        if !output::confirm("确认执行?", false, false) {
            output::warning("已取消。");
            return Ok(());
        }
    }

    // Install to project
    let result = open_sunstar_lib::cli_api::cli_design_install(project_path, &contract)?;

    if json {
        output::print_result(&result, true);
    } else {
        println!("Design Contract Installed: {}\n", contract.name);

        if result.design_md_created {
            println!("  ✓ DESIGN.md created");
        } else {
            println!("  · DESIGN.md (already exists, skipped)");
        }

        if result.dtchg_json_created {
            println!("  ✓ design-tokens.json created");
        } else {
            println!("  · design-tokens.json (already exists, skipped)");
        }

        if !result.files_created.is_empty() {
            println!("\n  Files created ({}):", result.files_created.len());
            for f in &result.files_created {
                println!("    + {f}");
            }
        }
        if !result.files_skipped.is_empty() {
            println!("\n  Files skipped ({}):", result.files_skipped.len());
            for f in &result.files_skipped {
                println!("    · {f}");
            }
        }
    }

    Ok(())
}

fn run_import(file_path: &str, json: bool) -> Result<(), String> {
    let result = open_sunstar_lib::cli_api::cli_design_import(file_path)?;

    if json {
        output::print_result(&result, true);
    } else {
        println!("Design Contract Imported\n");
        println!("  Source:     {}", result.source);
        println!("  Name:       {}", result.contract.name);
        println!("  Template:   {}", result.contract.source_template.as_deref().unwrap_or("custom"));
        println!("  Primary:    {}", result.contract.colors.primary);
        println!("  Font:       {}", result.contract.typography.font_family_base);

        if !result.warnings.is_empty() {
            println!("\n  Warnings:");
            for w in &result.warnings {
                println!("    · {w}");
            }
        }
    }

    Ok(())
}

fn run_compose(
    template_id: Option<&str>,
    brand_name: Option<&str>,
    primary_color: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let name = brand_name
        .map(|n| n.to_string())
        .unwrap_or_else(|| {
            template_id
                .map(|id| format!("{} Design", id))
                .unwrap_or_else(|| "Custom Design".to_string())
        });

    // If primary_color override is given, fetch base template colors and override primary
    let colors = if let Some(pc) = primary_color {
        let base_id = template_id.unwrap_or("neutral");
        let base_contract = open_sunstar_lib::cli_api::cli_design_get(base_id)?;
        let mut c = base_contract.colors;
        c.primary = pc.to_string();
        Some(c)
    } else {
        None
    };

    let params = open_sunstar_lib::DesignContractParams {
        template_id: template_id.map(|s| s.to_string()),
        name,
        description: None,
        colors,
        typography: None,
        spacing: None,
        elevation: None,
        shapes: None,
        components: None,
        guardrails: None,
    };

    let contract = open_sunstar_lib::cli_api::cli_design_compose(&params)?;

    if json {
        output::print_result(&contract, true);
    } else {
        output::header("Design Contract Compose");
        output::info(&format!("  Name:        {}", contract.name));
        if let Some(ref desc) = contract.description {
            output::info(&format!("  Description: {desc}"));
        }
        output::info(&format!(
            "  Template:    {}",
            contract.source_template.as_deref().unwrap_or("custom")
        ));
        output::info(&format!("  Primary:     {}", contract.colors.primary));
        output::info(&format!("  Secondary:   {}", contract.colors.accent));
        output::info(&format!("  Font:        {}", contract.typography.font_family_base));
        output::info(&format!("  Components:  {}", contract.components.len()));
        output::info(&format!("  Guardrails:  {}", contract.guardrails.len()));
    }

    Ok(())
}

fn run_dtcg(template_id: &str, json: bool) -> Result<(), String> {
    let contract = open_sunstar_lib::cli_api::cli_design_get(template_id)?;
    let dtcg_json = open_sunstar_lib::cli_api::cli_design_dtcg(&contract)?;

    if json {
        let value: serde_json::Value = serde_json::from_str(&dtcg_json)
            .map_err(|e| format!("Failed to parse DTCG JSON: {e}"))?;
        output::print_result(&value, true);
    } else {
        output::header("Design Tokens (W3C DTCG)");
        println!("{dtcg_json}");
    }

    Ok(())
}
