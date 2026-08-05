//! `os wiki` — 项目 Wiki / 知识基线管理
//!
//! 子命令：init, status, lint, inventory, changed
//! 所有操作均为本地离线命令，不依赖控制面。

use clap::{Args, Subcommand};

use crate::output;
use open_sunstar_lib::project_wiki;

#[derive(Args)]
pub struct WikiArgs {
    #[command(subcommand)]
    pub action: WikiAction,
}

#[derive(Subcommand)]
pub enum WikiAction {
    /// 初始化 Wiki 脚手架（safe install，never overwrite）
    Init {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 项目名称（用于模板变量替换，默认从路径推断）
        #[arg(long)]
        project_name: Option<String>,
        /// 预检模式：只展示将创建哪些文件，不实际写入
        #[arg(long)]
        dry_run: bool,
        /// 跳过确认提示
        #[arg(long)]
        yes: bool,
    },
    /// 查看 Wiki 状态（扫描结果）
    Status {
        /// 项目路径
        #[arg(long)]
        project_path: String,
    },
    /// 运行 Wiki Lint（Rust 原生，零 Python 依赖）
    Lint {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 启用 Quality 模式（Q001-Q010 规则）
        #[arg(long)]
        quality: bool,
        /// 仅在 lint 失败时返回非零退出码（默认行为）
        /// 不加此选项时，有 warnings 也会返回退出码 1
        #[arg(long)]
        strict: bool,
    },
    /// 查看 Wiki Inventory（页面级清单）
    Inventory {
        /// 项目路径
        #[arg(long)]
        project_path: String,
    },
    /// 映射变更文件到 wiki 页面（含冷启动检测）
    Changed {
        /// 项目路径
        #[arg(long)]
        project_path: String,
        /// 手动指定变更文件列表（不指定则使用 git diff --name-only）
        #[arg(long)]
        changed_files: Option<String>,
    },
    /// 合并 knowledge/ 正式区锚点到 ROUTING.md 受管表区（确定性，幂等）
    Routing {
        /// 项目路径
        #[arg(long)]
        project_path: String,
    },
}

pub fn run(args: WikiArgs, json: bool) -> Result<(), String> {
    match args.action {
        WikiAction::Init {
            project_path,
            project_name,
            dry_run,
            yes,
        } => run_init(&project_path, project_name, dry_run, yes, json),
        WikiAction::Status { project_path } => run_status(&project_path, json),
        WikiAction::Lint {
            project_path,
            quality,
            strict,
        } => run_lint(&project_path, quality, strict, json),
        WikiAction::Inventory { project_path } => run_inventory(&project_path, json),
        WikiAction::Changed {
            project_path,
            changed_files,
        } => run_changed(&project_path, changed_files, json),
        WikiAction::Routing { project_path } => {
            let report = open_sunstar_lib::knowledge_routing::merge_routing_index(
                &project_path,
            )?;
            if json {
                crate::output::print_result(&report, true);
            } else {
                crate::output::header("Knowledge ROUTING Merge");
                println!("  Pages scanned: {}", report.pages_scanned);
                println!("  Anchors:       {}", report.anchors);
                if report.routing_written {
                    crate::output::success("ROUTING.md 受管表区已更新");
                } else {
                    crate::output::success("ROUTING.md 已是最新（幂等）");
                }
            }
            Ok(())
        }
    }
}

fn derive_project_name(project_path: &str, override_name: &Option<String>) -> String {
    if let Some(name) = override_name {
        return name.clone();
    }
    std::path::Path::new(project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unnamed Project".to_string())
}

fn run_init(
    project_path: &str,
    project_name: Option<String>,
    dry_run: bool,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    let name = derive_project_name(project_path, &project_name);

    if dry_run {
        let plan =
            project_wiki::preview_wiki_init(project_path, "cli").map_err(|e| e.to_string())?;

        if json {
            output::print_result(&plan, true);
            return Ok(());
        }

        println!("Wiki 初始化预检（dry-run）");
        println!("项目: {project_path}");
        let will_create = plan.files.iter().filter(|f| f.will_create).count();
        let already = plan.files.iter().filter(|f| f.already_exists).count();
        println!("将创建 {will_create} 个文件，{already} 个已存在（将跳过）");
        if !plan.audit.warnings.is_empty() {
            println!("⚠ 警告:");
            for w in &plan.audit.warnings {
                println!("  • {w}");
            }
        }
        return Ok(());
    }

    if !yes && !json {
        println!("将为项目 '{name}' ({project_path}) 初始化 Wiki 脚手架。");
        println!("已存在的文件将被跳过（safe install）。");
        print!("确认继续？ [y/N] ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("已取消。");
            return Ok(());
        }
    }

    let result =
        project_wiki::init_project_wiki(project_path, "cli", &name).map_err(|e| e.to_string())?;

    if json {
        output::print_result(&result, true);
    } else {
        println!("✓ Wiki 初始化完成");
        println!("  创建 {} 个文件", result.files_created.len());
        if !result.files_created.is_empty() {
            for f in &result.files_created {
                println!("    + {f}");
            }
        }
        if !result.files_skipped.is_empty() {
            println!("  跳过 {} 个已存在文件", result.files_skipped.len());
            for f in &result.files_skipped {
                println!("    = {f}");
            }
        }
        println!("  profile: {}", result.profile_path);
    }
    Ok(())
}

fn run_status(project_path: &str, json: bool) -> Result<(), String> {
    let result = project_wiki::scan_project_wiki(project_path, "cli").map_err(|e| e.to_string())?;

    if json {
        output::print_result(&result, true);
    } else if !result.exists {
        println!("Wiki 未初始化。运行 `os wiki init --project-path {project_path}` 创建脚手架。");
    } else {
        println!("Wiki 状态: {}", result.base_status);
        println!("质量等级: {}", result.quality_level);
        println!("页面数: {}", result.page_count);
        println!("源码引用: {}", result.source_ref_count);
        println!("待查问题: {}", result.question_count);
        if let Some(latest) = result.latest_mtime {
            let dt = chrono::DateTime::from_timestamp(latest, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            println!("最近更新: {dt}");
        }
        println!("核心页面覆盖:");
        let c = &result.core_page_coverage;
        println!(
            "  index: {}  overview: {}  source-map: {}  log: {}  SCHEMA: {}",
            yn(c.has_index),
            yn(c.has_overview),
            yn(c.has_source_map),
            yn(c.has_log),
            yn(c.has_schema)
        );
        println!(
            "  component: {}  flow: {}  api: {}  runbook: {}",
            c.component_pages, c.flow_pages, c.api_pages, c.runbook_pages
        );
    }
    Ok(())
}

fn run_lint(project_path: &str, quality: bool, strict: bool, json: bool) -> Result<(), String> {
    let result =
        project_wiki::run_wiki_lint(project_path, "cli", quality).map_err(|e| e.to_string())?;

    if json {
        output::print_result(&result, true);
    } else {
        let s = &result.summary;
        if s.total_files == 0 {
            println!("Wiki 未初始化或无 .md 文件。");
        } else if s.passed {
            println!(
                "✓ Lint 通过（{} 文件，{} 警告，等级 {}）",
                s.total_files, s.warning_count, s.quality_level
            );
        } else {
            println!(
                "✗ Lint 失败（{} 文件，{} 错误，{} 警告）",
                s.total_files, s.error_count, s.warning_count
            );
        }

        if !result.errors.is_empty() {
            println!("\n错误:");
            for e in &result.errors {
                let loc = e.line.map(|l| format!(":{l}")).unwrap_or_default();
                println!("  [{}] {}{} — {}", e.rule_id, e.file, loc, e.message);
            }
        }
        if !result.warnings.is_empty() {
            println!("\n警告:");
            for w in &result.warnings {
                let loc = w.line.map(|l| format!(":{l}")).unwrap_or_default();
                println!("  [{}] {}{} — {}", w.rule_id, w.file, loc, w.message);
            }
        }
    }

    // 退出码：error_count > 0 返回 1；strict 模式下 warning_count > 0 也返回 1
    if result.summary.error_count > 0 {
        std::process::exit(1);
    }
    if strict && result.summary.warning_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn run_inventory(project_path: &str, json: bool) -> Result<(), String> {
    let result =
        project_wiki::build_wiki_inventory(project_path, "cli").map_err(|e| e.to_string())?;

    if json {
        output::print_result(&result, true);
    } else if result.pages.is_empty() {
        println!("Wiki 未初始化或无页面。");
    } else {
        println!("Wiki Inventory（{} 个页面）", result.pages.len());
        println!(
            "{:<50} {:<12} {:<10} {:<6}",
            "PATH", "TYPE", "STATUS", "SOURCES"
        );
        for p in &result.pages {
            println!(
                "{:<50} {:<12} {:<10} {:<6}",
                p.path,
                p.page_type,
                p.status,
                p.source_files.len()
            );
        }
    }
    Ok(())
}

fn run_changed(
    project_path: &str,
    changed_files: Option<String>,
    json: bool,
) -> Result<(), String> {
    let files = changed_files.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    });

    let result =
        project_wiki::map_wiki_changed_files(project_path, files).map_err(|e| e.to_string())?;

    if json {
        output::print_result(&result, true);
    } else if result.cold_start {
        println!("⚠ 冷启动态：changed-files 映射暂不生效");
        println!(
            "  有效 source_pages: {}/{}",
            result.effective_source_pages, result.threshold
        );
        if let Some(g) = &result.guidance {
            println!("  {g}");
        }
    } else {
        println!("Changed-files 映射结果:");
        println!("  变更文件: {}", result.changed_files.len());
        println!("  受影响 wiki 页面: {}", result.affected_pages.len());
        if !result.affected_pages.is_empty() {
            for p in &result.affected_pages {
                println!("    → {p}");
            }
        }
        if !result.unmapped_changed_files.is_empty() {
            println!("  未映射文件: {}", result.unmapped_changed_files.len());
            for f in &result.unmapped_changed_files {
                println!("    ? {f}");
            }
        }
    }
    Ok(())
}

fn yn(b: bool) -> &'static str {
    if b {
        "✓"
    } else {
        "✗"
    }
}
