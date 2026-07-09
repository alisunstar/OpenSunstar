//! `os drift` — 配置漂移治理（检查/修复）

use clap::Args;

use crate::output;

#[derive(Args)]
pub struct DriftArgs {
    #[command(subcommand)]
    pub action: DriftAction,
}

#[derive(clap::Subcommand)]
pub enum DriftAction {
    /// 扫描项目配置漂移（只读）
    Check {
        /// 项目路径（默认当前目录）
        #[arg(short, long, default_value = ".")]
        project_path: String,

        /// 目标 AI 工具（claude/codex/gemini/opencode/openclaw/hermes）
        #[arg(short, long)]
        app: Option<String>,

        /// 强制刷新治理缓存后再扫描
        #[arg(long)]
        refresh: bool,
    },

    /// 修复配置漂移（写操作）
    Repair {
        /// 项目路径（默认当前目录）
        #[arg(short, long, default_value = ".")]
        project_path: String,

        /// 要修复的检查项名称（不指定则修复全部）
        #[arg(short, long)]
        check: Option<String>,

        /// 目标 AI 工具
        #[arg(short, long)]
        app: Option<String>,

        /// 跳过确认，直接修复
        #[arg(short, long)]
        yes: bool,

        /// 预演模式：显示将要修复的内容，不实际执行
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn run(
    args: DriftArgs,
    state: &open_sunstar_lib::AppState,
    json: bool,
) -> Result<(), String> {
    match args.action {
        DriftAction::Check {
            project_path,
            app,
            refresh,
        } => run_check(state, &resolve_project_path(&project_path), app, refresh, json),
        DriftAction::Repair {
            project_path,
            check,
            app,
            yes,
            dry_run,
        } => run_repair(
            state,
            &resolve_project_path(&project_path),
            check,
            app,
            yes,
            dry_run,
            json,
        ),
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

fn run_check(
    state: &open_sunstar_lib::AppState,
    project_path: &str,
    target_app: Option<String>,
    refresh: bool,
    json: bool,
) -> Result<(), String> {
    // --refresh: 强制刷新治理缓存后再扫描
    if refresh {
        open_sunstar_lib::cli_api::cli_invalidate_readiness_cache(state, project_path);
        if !json {
            output::info(&format!("readiness cache invalidated for {project_path}"));
        }
    }

    let result = open_sunstar_lib::cli_api::cli_drift_check(state, project_path, target_app)?;

    if json {
        output::print_result(&result, true);
    } else {
        print_drift_report(&result);
    }

    // Exit code: 0 = 无漂移, 1 = 有漂移
    let has_drift = result.items.iter().any(|i| i.effective_state == "drifted");
    if has_drift {
        std::process::exit(1);
    }

    Ok(())
}

fn run_repair(
    state: &open_sunstar_lib::AppState,
    project_path: &str,
    check: Option<String>,
    target_app: Option<String>,
    yes: bool,
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    // 先扫描当前状态
    let scan = open_sunstar_lib::cli_api::cli_drift_check(state, project_path, target_app.clone())?;
    let drifted: Vec<_> = scan
        .items
        .iter()
        .filter(|i| i.effective_state == "drifted")
        .collect();

    if drifted.is_empty() {
        if json {
            let msg = serde_json::json!({
                "repaired": 0,
                "message": "No drift detected, nothing to repair."
            });
            output::print_result(&msg, true);
        } else {
            output::success("No drift detected, nothing to repair.");
        }
        return Ok(());
    }

    // dry-run: 只显示将要修复的内容
    if dry_run {
        if json {
            let items: Vec<_> = drifted
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
                "dry_run": true,
                "would_repair": items.len(),
                "items": items,
            });
            output::print_result(&report, true);
        } else {
            println!(
                "Would repair {} item(s) (dry-run, no changes made):\n",
                drifted.len()
            );
            for item in &drifted {
                println!(
                    "  • {} (detail: {})",
                    item.check_name,
                    item.effective_detail.as_deref().unwrap_or("-")
                );
                if let Some(ref path) = item.live_path {
                    println!("    live path: {path}");
                }
            }
        }
        return Ok(());
    }

    // 交互确认（除非 --yes 或 --json）
    let auto_confirm = yes || json;
    if !auto_confirm
        && !output::confirm(
            &format!("Found {} drifted item(s). Proceed with repair?", drifted.len()),
            false,
            false,
        )
    {
        output::info("Aborted.");
        return Ok(());
    }

    // 执行修复
    match check {
        Some(check_name) => {
            // 修复单项
            let result =
                open_sunstar_lib::cli_api::cli_drift_repair(state, project_path, &check_name, target_app)?;
            if json {
                output::print_result(&result, true);
            } else {
                let msg = format!(
                    "{}: {} → {}",
                    result.check_name, result.before_state, result.after_state
                );
                if result.repaired {
                    output::success(&msg);
                } else {
                    output::dim(&msg);
                }
            }
        }
        None => {
            // 修复全部
            let result =
                open_sunstar_lib::cli_api::cli_drift_repair_all(state, project_path, target_app)?;
            if json {
                output::print_result(&result, true);
            } else {
                output::info(&format!(
                    "Repaired {}/{} item(s). Still drifted: {}.",
                    result.repaired_count,
                    result.items.len(),
                    result.still_drifted_count
                ));
                for item in &result.items {
                    let msg = format!(
                        "{}: {} → {}",
                        item.check_name, item.before_state, item.after_state
                    );
                    if item.repaired {
                        output::success(&msg);
                    } else {
                        output::error_msg(&msg);
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_drift_report(result: &open_sunstar_lib::EffectiveScanResult) {
    let scanned_at = chrono::DateTime::from_timestamp(result.scanned_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| result.scanned_at.to_string());
    output::header(&format!(
        "Drift scan (target: {}, at: {scanned_at})",
        result.target_app
    ));
    eprintln!();

    let mut drifted_count = 0;
    for item in &result.items {
        let icon = match item.effective_state.as_str() {
            "effective" => "✓",
            "drifted" => {
                drifted_count += 1;
                "✗"
            }
            "unchecked" => "?",
            _ => "·",
        };
        let detail = item
            .effective_detail
            .as_deref()
            .unwrap_or("");
        println!(
            "  {icon} {:<24} {:<12} {detail}",
            item.check_name, item.effective_state
        );
    }

    println!();
    if drifted_count > 0 {
        output::warning(&format!(
            "{drifted_count} item(s) drifted. Run `os drift repair` to fix."
        ));
    } else {
        output::success("All items effective. No drift detected.");
    }
}
