//! `os readiness` — Agent 就绪度评分

use clap::Args;

use crate::output;

#[derive(Args)]
pub struct ReadinessArgs {
    #[command(subcommand)]
    pub action: ReadinessAction,
}

#[derive(clap::Subcommand)]
pub enum ReadinessAction {
    /// 计算 Agent 就绪度评分
    Score {
        /// 项目路径（默认当前目录）
        #[arg(short, long, default_value = ".")]
        project_path: String,

        /// 目标 AI 工具（claude/codex/gemini/opencode/openclaw/hermes）
        #[arg(short, long)]
        app: Option<String>,
    },
}

pub fn run(
    args: ReadinessArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        ReadinessAction::Score {
            project_path,
            app,
        } => {
            let path = resolve_project_path(&project_path);
            let result =
                open_sunstar_lib::cli_api::cli_readiness_score(state, &path, app)?;

            if json {
                output::print_result(&result, true);
            } else {
                print_readiness_report(&result);
            }

            // Exit code: 0 = score >= 60, 1 = score < 60 (below readiness threshold)
            if result.score < 60 {
                std::process::exit(1);
            }

            Ok(())
        }
    }
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

fn print_readiness_report(result: &open_sunstar_lib::cli_api::ReadinessScoreOutput) {
    let pct = (result.score as f64 / result.max_score as f64 * 100.0) as u32;
    let bar = render_progress_bar(pct);

    // Colored score: green ≥80%, yellow ≥60%, red <60%
    let colored_score = if pct >= 80 {
        console::style(format!("{}/{}", result.score, result.max_score))
            .green()
            .bold()
    } else if pct >= 60 {
        console::style(format!("{}/{}", result.score, result.max_score))
            .yellow()
            .bold()
    } else {
        console::style(format!("{}/{}", result.score, result.max_score))
            .red()
            .bold()
    };
    eprintln!("Agent Readiness Score: {colored_score} ({pct}%)");
    eprintln!("{bar}");
    eprintln!();

    output::info(&format!(
        "Project: {}  Target: {}",
        result.project_path, result.target_app
    ));
    eprintln!();

    output::header(&format!("{:<28} {:>6}  {}", "Check", "Score", "Detail"));
    output::dim(&"-".repeat(72));

    for item in &result.details {
        let status_icon = match item.status.as_deref() {
            Some("ready") => "✓",
            Some("partial") => "◐",
            Some("missing") => "✗",
            _ => "·",
        };
        let drift_marker = item
            .effective_state
            .as_deref()
            .map(|s| if s == "drifted" { " [drifted]" } else { "" })
            .unwrap_or("");
        println!(
            "  {status_icon} {:<26} {:>6}  {}{drift_marker}",
            item.check_name, item.score, item.detail
        );
    }

    if !result.drift_items.is_empty() {
        output::warning(&format!(
            "{} item(s) drifted: {}",
            result.drift_items.len(),
            result.drift_items.join(", ")
        ));
        output::info("Run `os drift repair` to fix.");
    }
}

fn render_progress_bar(pct: u32) -> String {
    let width = 40;
    let filled = (pct as usize * width) / 100;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
