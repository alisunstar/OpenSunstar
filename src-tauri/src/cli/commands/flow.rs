//! `os flow` — 工作流编排：列出模块/预设、阶段门禁校验、导出 profile

use clap::{Args, Subcommand};

use crate::output;

#[derive(Args)]
pub struct FlowArgs {
    #[command(subcommand)]
    pub action: FlowAction,
}

#[derive(Subcommand)]
pub enum FlowAction {
    /// 列出可用工作流模块
    List {
        /// 项目路径（可选，用于加载项目级覆盖）
        #[arg(long)]
        project_path: Option<String>,
        /// 列出预设而非模块
        #[arg(long)]
        presets: bool,
    },
    /// 阶段门禁校验（CI 关卡）
    Validate {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 预设 ID（可选，未提供时交互式选择）
        #[arg(long)]
        preset_id: Option<String>,
        /// 项目类型
        #[arg(long)]
        project_type: String,
        /// 变更 ID
        #[arg(long)]
        change_id: String,
        /// 目标阶段
        #[arg(long)]
        target_stage: String,
        /// 跨模块治理校验（检查设计合约/Recipe/Specs 就位）
        #[arg(long)]
        strict: bool,
    },
    /// 导出工作流 profile
    Export {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 预设 ID（可选，未提供时交互式选择）
        #[arg(long)]
        preset_id: Option<String>,
        /// 项目类型
        #[arg(long)]
        project_type: String,
        /// 严格校验阶段依赖；发现 S1-S5 语义冲突时拒绝导出
        #[arg(long)]
        strict: bool,
    },
    /// 查看预设详情
    Get {
        /// 预设 ID（可选，未提供时交互式选择）
        #[arg(long)]
        preset_id: Option<String>,
        /// 项目路径（加载项目级覆盖）
        #[arg(long)]
        project_path: Option<String>,
    },
    /// 扫描项目 .specs/ 目录与工作流索引
    Scan {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 预设 ID
        #[arg(long)]
        preset_id: Option<String>,
        /// 项目类型
        #[arg(long)]
        project_type: Option<String>,
    },
    /// 导出 FlowConfig（CI 门禁配置）
    Config {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 预设 ID（可选，未提供时交互式选择）
        #[arg(long)]
        preset_id: Option<String>,
        /// 项目类型
        #[arg(long)]
        project_type: String,
        /// 严格校验阶段依赖；发现 S1-S5 语义冲突时拒绝导出
        #[arg(long)]
        strict: bool,
    },
    /// 构建阶段 DAG 图
    Graph {
        /// 预设 ID（可选，未提供时交互式选择）
        #[arg(long)]
        preset_id: Option<String>,
        /// 项目路径（加载项目级覆盖）
        #[arg(long)]
        project_path: Option<String>,
    },
}

/// Resolve preset_id: use the provided value, or present an interactive selector.
fn resolve_preset_id(
    preset_id: Option<String>,
    project_path: Option<&str>,
    json: bool,
) -> Result<String, String> {
    match preset_id {
        Some(id) => Ok(id),
        None => {
            let presets = open_sunstar_lib::cli_api::cli_flow_presets(project_path)?;
            let display: Vec<String> = presets
                .iter()
                .map(|p| format!("{} — {}", p.id, p.name))
                .collect();
            match output::select("Select preset", &display, json) {
                Some(idx) => Ok(presets[idx].id.clone()),
                None => Err("No preset selected".to_string()),
            }
        }
    }
}

pub fn run(args: FlowArgs, json: bool) -> Result<(), String> {
    match args.action {
        FlowAction::List {
            project_path,
            presets,
        } => {
            if presets {
                run_list_presets(project_path.as_deref(), json)
            } else {
                run_list_modules(project_path.as_deref(), json)
            }
        }
        FlowAction::Validate {
            project_path,
            preset_id,
            project_type,
            change_id,
            target_stage,
            strict,
        } => {
            let preset_id = resolve_preset_id(preset_id, Some(project_path.as_str()), json)?;
            run_validate(
                &project_path,
                &preset_id,
                &project_type,
                &change_id,
                &target_stage,
                strict,
                json,
            )
        }
        FlowAction::Export {
            project_path,
            preset_id,
            project_type,
            strict,
        } => {
            let preset_id = resolve_preset_id(preset_id, Some(project_path.as_str()), json)?;
            run_export(&project_path, &preset_id, &project_type, strict, json)
        }
        FlowAction::Get {
            preset_id,
            project_path,
        } => {
            let preset_id = resolve_preset_id(preset_id, project_path.as_deref(), json)?;
            run_get(&preset_id, project_path.as_deref(), json)
        }
        FlowAction::Scan {
            project_path,
            preset_id,
            project_type,
        } => run_scan(
            &project_path,
            preset_id.as_deref(),
            project_type.as_deref(),
            json,
        ),
        FlowAction::Config {
            project_path,
            preset_id,
            project_type,
            strict,
        } => {
            let preset_id = resolve_preset_id(preset_id, Some(project_path.as_str()), json)?;
            run_config(&project_path, &preset_id, &project_type, strict, json)
        }
        FlowAction::Graph {
            preset_id,
            project_path,
        } => {
            let preset_id = resolve_preset_id(preset_id, project_path.as_deref(), json)?;
            run_graph(&preset_id, project_path.as_deref(), json)
        }
    }
}

fn run_list_modules(project_path: Option<&str>, json: bool) -> Result<(), String> {
    let modules = open_sunstar_lib::cli_api::cli_flow_list(project_path)?;

    if json {
        output::print_result(&modules, true);
    } else {
        println!("Workflow Modules ({}):\n", modules.len());
        println!("{:<24} {:<12} {}", "ID", "Source", "Description");
        println!("{}", "-".repeat(64));
        for m in &modules {
            println!("  {:<22} {:<12} {}", m.id, m.source, m.description);
        }
    }

    Ok(())
}

fn run_list_presets(project_path: Option<&str>, json: bool) -> Result<(), String> {
    let presets = open_sunstar_lib::cli_api::cli_flow_presets(project_path)?;

    if json {
        output::print_result(&presets, true);
    } else {
        println!("Workflow Presets ({}):\n", presets.len());
        println!(
            "{:<16} {:<20} {:>7} {:>7}  {}",
            "ID", "Name", "Modules", "Stages", "Description"
        );
        println!("{}", "-".repeat(72));
        for p in &presets {
            println!(
                "  {:<14} {:<20} {:>5} {:>7}  {}",
                p.id, p.name, p.module_count, p.stage_count, p.description
            );
        }
    }

    Ok(())
}

fn run_validate(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    change_id: &str,
    target_stage: &str,
    strict: bool,
    json: bool,
) -> Result<(), String> {
    let result = open_sunstar_lib::cli_api::cli_flow_validate(
        project_path,
        preset_id,
        project_type,
        change_id,
        target_stage,
    )?;

    // ── Cross-module governance checks (strict mode) ──
    let mut governance_warnings: Vec<String> = Vec::new();
    if strict {
        use std::path::Path;
        let pp = Path::new(project_path);
        let dot = pp.join(".opensunstar");

        if !pp.join("DESIGN.md").is_file() {
            governance_warnings.push(
                "Design contract not installed (DESIGN.md missing at project root)".to_string(),
            );
        }
        let recipe_count = std::fs::read_dir(dot.join("recipe"))
            .map(|d| d.count())
            .unwrap_or(0);
        if recipe_count == 0 {
            governance_warnings
                .push("No recipes saved (.opensunstar/recipe/ is empty)".to_string());
        }
        if !pp.join(".specs").is_dir() {
            governance_warnings
                .push("Specs directory not initialized (.specs/ missing)".to_string());
        }
        if !dot.join("workflow.profile.json").is_file() {
            governance_warnings.push(
                "Workflow profile not exported (.opensunstar/workflow.profile.json missing)"
                    .to_string(),
            );
        }
        if !dot.join("flow-config.yaml").is_file() {
            governance_warnings.push(
                "FlowConfig not exported (.opensunstar/flow-config.yaml missing)".to_string(),
            );
        }
    }

    if json {
        let mut json_result = serde_json::to_value(&result).unwrap();
        if strict {
            json_result["governance_warnings"] = serde_json::json!(governance_warnings);
            json_result["governance_passed"] = serde_json::json!(governance_warnings.is_empty());
            json_result["strict"] = serde_json::json!(true);
        }
        output::print_result(&json_result, true);
    } else {
        let icon = if result.allowed { "✓" } else { "✗" };
        println!(
            "{icon} Stage Gate: {} → {} (change: {})\n",
            result.target_stage,
            if result.allowed { "ALLOWED" } else { "BLOCKED" },
            result.change_id
        );

        if !result.satisfied_artifacts.is_empty() {
            println!("Satisfied ({}):", result.satisfied_artifacts.len());
            for a in &result.satisfied_artifacts {
                output::success(a);
            }
        }

        if !result.missing_artifacts.is_empty() {
            println!("\nMissing ({}):", result.missing_artifacts.len());
            for a in &result.missing_artifacts {
                output::error_msg(a);
            }
        }

        if !result.warnings.is_empty() {
            println!("\nWarnings:");
            for w in &result.warnings {
                output::warning(w);
            }
        }

        // Governance checks
        if strict {
            eprintln!();
            output::header("Governance Checks (strict):");
            if governance_warnings.is_empty() {
                output::success("All governance assets in place");
            } else {
                for w in &governance_warnings {
                    output::warning(w);
                }
            }
        }
    }

    // Exit code 1 if artifact gate blocked (CI usage)
    if !result.allowed {
        std::process::exit(1);
    }

    // Exit code 2 if governance gate blocked (strict mode)
    if strict && !governance_warnings.is_empty() {
        std::process::exit(2);
    }

    Ok(())
}

fn run_export(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    strict: bool,
    json: bool,
) -> Result<(), String> {
    let profile =
        open_sunstar_lib::cli_api::cli_flow_export(project_path, preset_id, project_type, strict)?;

    if json {
        output::print_result(&profile, true);
    } else {
        println!("Workflow Profile Exported\n");
        println!("  Preset:     {}", profile.preset_id);
        println!("  Type:       {}", profile.project_type);
        println!("  Modules:    {}", profile.modules.join(", "));
        println!("  Stages:     {}", profile.resolved_stages.join(" → "));
        println!("  Exported:   {}", profile.exported_at);
        if !profile.semantic_warnings.is_empty() {
            println!("\n  Semantic Warnings:");
            for w in &profile.semantic_warnings {
                println!("    · {w}");
            }
        }
    }

    Ok(())
}

fn run_get(preset_id: &str, project_path: Option<&str>, json: bool) -> Result<(), String> {
    let preset = open_sunstar_lib::cli_api::cli_flow_preset_get(preset_id, project_path)?;

    if json {
        output::print_result(&preset, true);
    } else {
        output::header(&format!("Preset: {}", preset.name));
        output::dim(&format!("  ID:          {}", preset.id));
        if let Some(ref zh) = preset.name_zh {
            output::dim(&format!("  Name (zh):   {zh}"));
        }
        output::info(&format!("  Description: {}", preset.description));
        if let Some(ref tier) = preset.r3_tier {
            output::dim(&format!("  R3 Tier:     {tier}"));
        }

        if !preset.modules.is_empty() {
            println!();
            output::info(&format!("  Modules ({}):", preset.modules.len()));
            for m in &preset.modules {
                output::success(&format!("    · {m}"));
            }
        }

        if !preset.stages.is_empty() {
            println!();
            output::info(&format!("  Stages ({}):", preset.stages.len()));
            for s in &preset.stages {
                let deps = if s.depends_on.is_empty() {
                    String::new()
                } else {
                    format!("  ← {}", s.depends_on.join(", "))
                };
                println!("    • {} ({}){}", s.id, s.name, deps);
                if !s.artifacts.is_empty() {
                    output::dim(&format!(
                        "      artifacts: {}",
                        s.artifacts
                            .iter()
                            .map(|a| a.file.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }
    }

    Ok(())
}

fn run_scan(
    project_path: &str,
    preset_id: Option<&str>,
    project_type: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let index = open_sunstar_lib::cli_api::cli_flow_scan(project_path, preset_id, project_type)?;

    if json {
        output::print_result(&index, true);
    } else {
        output::header("Specs Workflow Scan");
        output::dim(&format!("  Project:       {}", index.project_path));
        output::dim(&format!(
            "  Workspace:     {}",
            if index.workspace_exists { "yes" } else { "no" }
        ));
        output::dim(&format!(
            "  Flow-Kit:      {}",
            if index.has_flow_kit { "yes" } else { "no" }
        ));
        output::dim(&format!(
            "  .specs/ dir:   {}",
            if index.has_specs_dir { "yes" } else { "no" }
        ));
        if let Some(ref cid) = index.active_change_id {
            output::info(&format!("  Active change: {cid}"));
        }

        if let Some(ref profile) = index.saved_profile {
            println!();
            output::info("  Saved Profile:");
            output::dim(&format!("    preset:  {}", profile.preset_id));
            output::dim(&format!("    type:    {}", profile.project_type));
            output::dim(&format!(
                "    stages:  {}",
                profile.resolved_stages.join(" → ")
            ));
        }

        if !index.changes.is_empty() {
            println!();
            output::info(&format!("  Changes ({}):", index.changes.len()));
            for c in &index.changes {
                let icon = if c.artifact_completeness == 100 {
                    "✓"
                } else {
                    "·"
                };
                println!(
                    "    {icon} {} — {}% complete ({} artifacts)",
                    c.change_id,
                    c.artifact_completeness,
                    c.artifacts.len()
                );
            }
        } else {
            println!();
            output::dim("  No changes found in .specs/");
        }
    }

    Ok(())
}

fn run_config(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    strict: bool,
    json: bool,
) -> Result<(), String> {
    let config =
        open_sunstar_lib::cli_api::cli_flow_config(project_path, preset_id, project_type, strict)?;

    if json {
        output::print_result(&config, true);
    } else {
        output::header("Flow Config");
        output::dim(&format!("  Schema:      v{}", config.schema_version));
        output::dim(&format!("  Preset:      {}", config.preset_id));
        output::dim(&format!("  Type:        {}", config.project_type));
        output::dim(&format!("  Modules:     {}", config.modules.join(", ")));

        if !config.stages.is_empty() {
            println!();
            output::info(&format!("  Stages ({}):", config.stages.len()));
            for s in &config.stages {
                let status = if s.enabled { "✓" } else { "✗" };
                println!("    {status} {}", s.id);
                if !s.depends_on.is_empty() {
                    output::dim(&format!("      depends_on: {}", s.depends_on.join(", ")));
                }
                if !s.gates.is_empty() {
                    for g in &s.gates {
                        println!("      gate [{}]: {}", g.gate_type, g.artifacts.join(", "));
                    }
                }
            }
        }

        println!();
        output::info("  Rules:");
        output::dim(&format!(
            "    max_auto_retry:     {}",
            config.rules.max_auto_retry
        ));
        output::dim(&format!(
            "    role_separation:    {}",
            config.rules.role_separation
        ));
        output::dim(&format!(
            "    require_diff_boundary: {}",
            config.rules.require_diff_boundary
        ));

        if !config.semantic_warnings.is_empty() {
            println!();
            output::warning("  Semantic Warnings:");
            for w in &config.semantic_warnings {
                output::dim(&format!("    · {w}"));
            }
        }
    }

    Ok(())
}

fn run_graph(preset_id: &str, project_path: Option<&str>, json: bool) -> Result<(), String> {
    let graph = open_sunstar_lib::cli_api::cli_flow_graph(preset_id, project_path)?;

    if json {
        output::print_result(&graph, true);
    } else {
        output::header(&format!("Stage Graph: {}", graph.preset_name));
        output::dim(&format!("  Preset:    {}", graph.preset_id));
        output::dim(&format!("  Framework: {}", graph.source_framework));
        output::dim(&format!(
            "  Nodes:     {} (+ {} lateral)",
            graph.nodes.len(),
            graph.lateral_nodes.len()
        ));
        output::dim(&format!("  Edges:     {}", graph.edges.len()));

        if !graph.nodes.is_empty() {
            println!();
            output::info("  Nodes:");
            for n in &graph.nodes {
                println!("    [d{}] {} ({})", n.depth, n.id, n.name);
                if !n.artifacts.is_empty() {
                    output::dim(&format!("      artifacts:  {}", n.artifacts.join(", ")));
                }
                if !n.depends_on.is_empty() {
                    output::dim(&format!("      depends_on: {}", n.depends_on.join(", ")));
                }
            }
        }

        if !graph.lateral_nodes.is_empty() {
            println!();
            output::info("  Lateral (cross-cutting):");
            for n in &graph.lateral_nodes {
                println!("    ↔ {} ({})", n.id, n.name);
            }
        }

        if !graph.edges.is_empty() {
            println!();
            output::info("  Edges:");
            for e in &graph.edges {
                println!("    {} → {}", e.source, e.target);
            }
        }
    }

    Ok(())
}
